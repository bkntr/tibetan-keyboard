use image::ImageReader;
use std::{
    error::Error,
    io::Cursor,
    mem::size_of,
    process::Command,
    ptr,
    sync::{Mutex, OnceLock},
};
use tibetan_ewts_keyboard::{
    composition::{Composer, Replacement},
    config::{Config, HotkeyMatch, HotkeyMatcher, Modifiers},
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{
            GetKeyState, GetKeyboardLayout, HKL, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
            KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC_EX, MapVirtualKeyExW, SendInput,
            ToUnicodeEx, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT,
            VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
        },
        WindowsAndMessaging::{
            CallNextHookEx, GetForegroundWindow, GetWindowThreadProcessId, HHOOK, KBDLLHOOKSTRUCT,
            LLKHF_EXTENDED, MB_ICONERROR, MB_OK, MessageBoxW, SetWindowsHookExW,
            UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
            WM_MBUTTONDOWN, WM_RBUTTONDOWN, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN,
        },
    },
};
use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::WindowId,
};

const MENU_TOGGLE: &str = "toggle";
const MENU_OPEN_SETTINGS: &str = "open-settings";
const MENU_RELOAD_SETTINGS: &str = "reload-settings";
const MENU_EXIT: &str = "exit";
const INPUT_MARKER: usize = 0x4557_5453_4B42_4455;

#[derive(Debug, Clone)]
enum UserEvent {
    Menu(MenuEvent),
    StateChanged(bool),
}

struct HookRuntime {
    composer: Composer,
    hotkey_matcher: HotkeyMatcher,
    keyboard_state: KeyboardState,
    consumed_keys: [bool; 256],
    enabled: bool,
    proxy: EventLoopProxy<UserEvent>,
}

#[derive(Clone)]
struct KeyboardState {
    keys: [u8; 256],
}

impl KeyboardState {
    fn new() -> Self {
        let mut keys = [0; 256];
        // SAFETY: VK_CAPITAL is a documented virtual-key value.
        if unsafe { GetKeyState(VK_CAPITAL as i32) } & 1 != 0 {
            keys[VK_CAPITAL as usize] = 1;
        }
        Self { keys }
    }

    fn update(&mut self, key: u16, is_down: bool) {
        let Some(state) = self.keys.get_mut(key as usize) else {
            return;
        };
        let was_down = *state & 0x80 != 0;
        if key == VK_CAPITAL && is_down && !was_down {
            *state ^= 1;
        }
        *state = (*state & 1) | if is_down { 0x80 } else { 0 };

        match key {
            VK_LSHIFT | VK_RSHIFT => self.refresh_aggregate(VK_SHIFT, VK_LSHIFT, VK_RSHIFT),
            VK_LCONTROL | VK_RCONTROL => {
                self.refresh_aggregate(VK_CONTROL, VK_LCONTROL, VK_RCONTROL)
            }
            VK_LMENU | VK_RMENU => self.refresh_aggregate(VK_MENU, VK_LMENU, VK_RMENU),
            _ => {}
        }
    }

    fn refresh_aggregate(&mut self, aggregate: u16, left: u16, right: u16) {
        let is_down = self.is_down(left) || self.is_down(right);
        let toggle = self.keys[aggregate as usize] & 1;
        self.keys[aggregate as usize] = toggle | if is_down { 0x80 } else { 0 };
    }

    fn is_down(&self, key: u16) -> bool {
        self.keys
            .get(key as usize)
            .is_some_and(|state| state & 0x80 != 0)
    }

    fn modifiers(&self) -> Modifiers {
        Modifiers {
            ctrl: self.is_down(VK_CONTROL)
                || self.is_down(VK_LCONTROL)
                || self.is_down(VK_RCONTROL),
            alt: self.is_down(VK_MENU) || self.is_down(VK_LMENU) || self.is_down(VK_RMENU),
            shift: self.is_down(VK_SHIFT) || self.is_down(VK_LSHIFT) || self.is_down(VK_RSHIFT),
            win: self.is_down(VK_LWIN) || self.is_down(VK_RWIN),
        }
    }

    fn for_modifiers(&self, modifiers: Modifiers) -> Self {
        let mut state = self.clone();
        for key in [
            VK_SHIFT,
            VK_LSHIFT,
            VK_RSHIFT,
            VK_CONTROL,
            VK_LCONTROL,
            VK_RCONTROL,
            VK_MENU,
            VK_LMENU,
            VK_RMENU,
            VK_LWIN,
            VK_RWIN,
        ] {
            state.keys[key as usize] &= 1;
        }
        state.update(VK_SHIFT, modifiers.shift);
        state.update(VK_CONTROL, modifiers.ctrl);
        state.update(VK_MENU, modifiers.alt);
        state.update(VK_LWIN, modifiers.win);
        state
    }
}

enum SynthAction {
    Replacement(Replacement),
    RawKey { key: u16, is_down: bool },
}

#[derive(Debug, PartialEq, Eq)]
enum KeyTranslation {
    Ewts(char),
    Suppress,
    Pass,
}

static HOOK_RUNTIME: OnceLock<Mutex<HookRuntime>> = OnceLock::new();

struct Application {
    config: Config,
    tray: Option<TrayIcon>,
    toggle_item: Option<MenuItem>,
    enabled_icon: Icon,
    disabled_icon: Icon,
    keyboard_hook: HHOOK,
    mouse_hook: HHOOK,
}

impl Application {
    fn new(config: Config, proxy: EventLoopProxy<UserEvent>) -> Result<Self, Box<dyn Error>> {
        let hotkey = config.parsed_hotkey()?;
        HOOK_RUNTIME
            .set(Mutex::new(HookRuntime {
                composer: Composer::default(),
                hotkey_matcher: HotkeyMatcher::new(hotkey),
                keyboard_state: KeyboardState::new(),
                consumed_keys: [false; 256],
                enabled: config.enabled_on_start,
                proxy,
            }))
            .map_err(|_| "keyboard runtime was initialized more than once")?;

        Ok(Self {
            config,
            tray: None,
            toggle_item: None,
            // Use shell-size artwork rather than asking Windows to reduce the
            // 256px master all the way to a DPI-scaled tray slot.
            enabled_icon: load_icon(include_bytes!("../assets/om-enabled-tray.png"))?,
            disabled_icon: load_icon(include_bytes!("../assets/om-disabled-tray.png"))?,
            keyboard_hook: ptr::null_mut(),
            mouse_hook: ptr::null_mut(),
        })
    }

    fn enabled(&self) -> bool {
        HOOK_RUNTIME
            .get()
            .and_then(|runtime| runtime.lock().ok().map(|runtime| runtime.enabled))
            .unwrap_or(false)
    }

    fn initialize(&mut self) -> Result<(), Box<dyn Error>> {
        if self.tray.is_some() {
            return Ok(());
        }

        let menu = Menu::new();
        let toggle_item = MenuItem::with_id(
            MENU_TOGGLE,
            toggle_label(self.enabled(), &self.config.hotkey),
            true,
            None,
        );
        let open_settings = MenuItem::with_id(MENU_OPEN_SETTINGS, "Open settings…", true, None);
        let reload_settings =
            MenuItem::with_id(MENU_RELOAD_SETTINGS, "Reload settings", true, None);
        let exit = MenuItem::with_id(MENU_EXIT, "Exit", true, None);
        menu.append_items(&[
            &toggle_item,
            &PredefinedMenuItem::separator(),
            &open_settings,
            &reload_settings,
            &PredefinedMenuItem::separator(),
            &exit,
        ])?;

        let icon = if self.enabled() {
            self.enabled_icon.clone()
        } else {
            self.disabled_icon.clone()
        };
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(tooltip(self.enabled(), &self.config.hotkey))
            .with_icon(icon)
            .build()?;

        // SAFETY: `low_level_keyboard_proc` has the required ABI and remains
        // valid for the process lifetime. The winit event loop pumps messages
        // on this same thread until the hook is removed in `Drop`.
        self.keyboard_hook = unsafe {
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                ptr::null_mut(),
                0,
            )
        };
        if self.keyboard_hook.is_null() {
            return Err("SetWindowsHookExW failed".into());
        }

        // A click can move the caret without producing a keyboard event. End
        // the live EWTS span before the application handles that click so the
        // next keystroke cannot replace text at the caret's new location.
        self.mouse_hook = unsafe {
            SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), ptr::null_mut(), 0)
        };
        if self.mouse_hook.is_null() {
            // SAFETY: this is the hook handle returned just above.
            unsafe { UnhookWindowsHookEx(self.keyboard_hook) };
            self.keyboard_hook = ptr::null_mut();
            return Err("SetWindowsHookExW failed for mouse hook".into());
        }

        self.toggle_item = Some(toggle_item);
        self.tray = Some(tray);
        Ok(())
    }

    fn set_visual_state(&mut self, enabled: bool) {
        if let Some(tray) = &self.tray {
            let icon = if enabled {
                self.enabled_icon.clone()
            } else {
                self.disabled_icon.clone()
            };
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_tooltip(Some(tooltip(enabled, &self.config.hotkey)));
        }
        if let Some(item) = &self.toggle_item {
            item.set_text(toggle_label(enabled, &self.config.hotkey));
        }
    }

    fn toggle(&mut self) {
        let enabled = if let Some(runtime) = HOOK_RUNTIME.get() {
            let mut runtime = runtime.lock().expect("hook runtime poisoned");
            runtime.composer.commit();
            runtime.enabled = !runtime.enabled;
            runtime.enabled
        } else {
            return;
        };
        self.set_visual_state(enabled);
    }

    fn reload_settings(&mut self) -> Result<(), Box<dyn Error>> {
        let config = Config::load_or_create()?;
        let hotkey = config.parsed_hotkey()?;
        if let Some(runtime) = HOOK_RUNTIME.get() {
            let mut runtime = runtime.lock().expect("hook runtime poisoned");
            runtime.hotkey_matcher = HotkeyMatcher::new(hotkey);
            runtime.composer.commit();
        }
        self.config = config;
        self.set_visual_state(self.enabled());
        Ok(())
    }

    fn handle_menu(&mut self, event_loop: &ActiveEventLoop, event: MenuEvent) {
        match event.id.as_ref() {
            MENU_TOGGLE => self.toggle(),
            MENU_OPEN_SETTINGS => {
                if let Err(error) = open_settings() {
                    show_error("Unable to open settings", &error.to_string());
                }
            }
            MENU_RELOAD_SETTINGS => {
                if let Err(error) = self.reload_settings() {
                    show_error("Invalid settings", &error.to_string());
                }
            }
            MENU_EXIT => event_loop.exit(),
            _ => {}
        }
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        if !self.keyboard_hook.is_null() {
            // SAFETY: this is the hook handle returned by SetWindowsHookExW.
            unsafe { UnhookWindowsHookEx(self.keyboard_hook) };
            self.keyboard_hook = ptr::null_mut();
        }
        if !self.mouse_hook.is_null() {
            // SAFETY: this is the hook handle returned by SetWindowsHookExW.
            unsafe { UnhookWindowsHookEx(self.mouse_hook) };
            self.mouse_hook = ptr::null_mut();
        }
    }
}

impl ApplicationHandler<UserEvent> for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.initialize() {
            show_error("Tibetan EWTS Keyboard", &error.to_string());
            event_loop.exit();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Menu(event) => self.handle_menu(event_loop, event),
            UserEvent::StateChanged(enabled) => self.set_visual_state(enabled),
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let config = Config::load_or_create()?;
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    let proxy = event_loop.create_proxy();
    let mut app = Application::new(config, proxy)?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn load_icon(bytes: &[u8]) -> Result<Icon, Box<dyn Error>> {
    let image = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .decode()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Ok(Icon::from_rgba(image.into_raw(), width, height)?)
}

fn toggle_label(enabled: bool, hotkey: &str) -> String {
    format!(
        "{} Tibetan ({hotkey})",
        if enabled { "Disable" } else { "Enable" }
    )
}

fn tooltip(enabled: bool, hotkey: &str) -> String {
    format!(
        "Tibetan EWTS Keyboard: {}\nToggle: {hotkey}",
        if enabled { "enabled" } else { "disabled" }
    )
}

fn open_settings() -> Result<(), Box<dyn Error>> {
    let path = Config::path();
    if !path.exists() {
        Config::default().save()?;
    }
    Command::new("notepad.exe").arg(path).spawn()?;
    Ok(())
}

pub fn show_error(title: &str, message: &str) {
    let title = wide_null(title);
    let message = wide_null(message);
    // SAFETY: both strings are valid, NUL-terminated UTF-16 buffers.
    unsafe {
        MessageBoxW(
            ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        )
    };
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn low_level_mouse_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0
        && is_mouse_button_down(wparam)
        && let Some(runtime_mutex) = HOOK_RUNTIME.get()
        && let Ok(mut runtime) = runtime_mutex.lock()
    {
        runtime.composer.commit();
    }

    // SAFETY: forwarding the hook parameters is required by the hook contract.
    unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) }
}

fn is_mouse_button_down(message: WPARAM) -> bool {
    matches!(
        message as u32,
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
    )
}

unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code < 0 {
        // SAFETY: forwarding the hook parameters is required by the hook contract.
        return unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) };
    }

    // SAFETY: for HC_ACTION keyboard messages, lparam points to KBDLLHOOKSTRUCT.
    let event = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
    if event.dwExtraInfo == INPUT_MARKER {
        return unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) };
    }
    // Release builds ignore automation and other injected keystrokes. Debug
    // builds accept foreign injected input so `scripts/e2e.ps1` can exercise
    // the complete hook -> composer -> SendInput path in a real edit control.
    #[cfg(not(debug_assertions))]
    if event.flags & windows_sys::Win32::UI::WindowsAndMessaging::LLKHF_INJECTED != 0 {
        return unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) };
    }

    let is_down = wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize;
    let is_up = wparam == WM_KEYUP as usize || wparam == WM_SYSKEYUP as usize;
    if !is_down && !is_up {
        return unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) };
    }

    let Some(runtime_mutex) = HOOK_RUNTIME.get() else {
        return unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) };
    };

    let mut actions = Vec::new();
    let mut suppress = false;
    {
        let mut runtime = runtime_mutex.lock().expect("hook runtime poisoned");
        let key = event.vkCode as u16;
        let modifiers = runtime.keyboard_state.modifiers();
        let hotkey_match = runtime.hotkey_matcher.handle(key, is_down, modifiers);
        runtime.keyboard_state.update(
            normalized_state_key(key, event.scanCode, event.flags),
            is_down,
        );
        let mut process_normally = false;
        let mut reprocess_suppressed_event = false;

        match hotkey_match {
            HotkeyMatch::Pass => process_normally = true,
            HotkeyMatch::Suppress => suppress = true,
            HotkeyMatch::Trigger => {
                suppress = true;
                runtime.composer.commit();
                runtime.enabled = !runtime.enabled;
                let _ = runtime
                    .proxy
                    .send_event(UserEvent::StateChanged(runtime.enabled));
            }
            HotkeyMatch::CancelAndPass(replay) => {
                queue_hotkey_replay(&mut runtime, replay, &mut actions);
                // The cancelling event is a required modifier release. Put it
                // after the replayed prefix so applications observe the same
                // ordering the user typed, then suppress the physical event.
                actions.push(SynthAction::RawKey {
                    key,
                    is_down: false,
                });
                suppress = true;
            }
            HotkeyMatch::CancelAndReprocess(replay) => {
                queue_hotkey_replay(&mut runtime, replay, &mut actions);
                process_normally = true;
                reprocess_suppressed_event = true;
                suppress = true;
            }
        }

        if process_normally {
            let handled = if is_up && take_consumed_key(&mut runtime, key) {
                true
            } else if !runtime.enabled {
                false
            } else if key == VK_BACK {
                if is_down {
                    if let Some(edit) = runtime.composer.backspace() {
                        actions.push(SynthAction::Replacement(edit));
                        true
                    } else {
                        false
                    }
                } else {
                    !runtime.composer.is_empty()
                }
            } else if is_modifier_key(key) {
                if is_down {
                    runtime.composer.commit();
                }
                false
            } else if is_down && !is_shortcut_chord(&runtime.keyboard_state) {
                match translate_key_for_active_layout(
                    event.vkCode,
                    event.scanCode,
                    &runtime.keyboard_state,
                ) {
                    KeyTranslation::Ewts(ch) => {
                        actions.push(SynthAction::Replacement(runtime.composer.push(ch)));
                        mark_consumed_key(&mut runtime, key);
                        true
                    }
                    KeyTranslation::Suppress => {
                        mark_consumed_key(&mut runtime, key);
                        true
                    }
                    KeyTranslation::Pass => {
                        runtime.composer.commit();
                        false
                    }
                }
            } else if is_down {
                runtime.composer.commit();
                false
            } else {
                false
            };

            suppress |= handled;
            if reprocess_suppressed_event && !handled {
                actions.push(SynthAction::RawKey { key, is_down });
            }
        }
    }

    for action in actions {
        match action {
            SynthAction::Replacement(edit) => send_replacement(&edit),
            SynthAction::RawKey { key, is_down } => send_raw_key(key, is_down),
        }
    }
    if suppress {
        1
    } else {
        unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) }
    }
}

fn queue_hotkey_replay(runtime: &mut HookRuntime, keys: Vec<u16>, actions: &mut Vec<SynthAction>) {
    let modifiers = runtime.hotkey_matcher.hotkey().modifiers;
    let control_chord = modifiers.ctrl || modifiers.alt || modifiers.win;
    let keyboard_state = runtime.keyboard_state.for_modifiers(modifiers);

    for key in keys {
        if runtime.enabled && !control_chord {
            let layout = active_keyboard_layout();
            // SAFETY: the layout belongs to the foreground thread and the map
            // type requests a scan code for the supplied virtual key.
            let scan_code = unsafe { MapVirtualKeyExW(key as u32, MAPVK_VK_TO_VSC_EX, layout) };
            match translate_key(key as u32, scan_code, &keyboard_state, layout) {
                KeyTranslation::Ewts(ch) => {
                    actions.push(SynthAction::Replacement(runtime.composer.push(ch)));
                    continue;
                }
                KeyTranslation::Suppress => continue,
                KeyTranslation::Pass => {}
            }
        }

        if runtime.enabled {
            runtime.composer.commit();
        }
        actions.push(SynthAction::RawKey { key, is_down: true });
        actions.push(SynthAction::RawKey {
            key,
            is_down: false,
        });
    }
}

fn mark_consumed_key(runtime: &mut HookRuntime, key: u16) {
    if let Some(consumed) = runtime.consumed_keys.get_mut(key as usize) {
        *consumed = true;
    }
}

fn take_consumed_key(runtime: &mut HookRuntime, key: u16) -> bool {
    let Some(consumed) = runtime.consumed_keys.get_mut(key as usize) else {
        return false;
    };
    std::mem::take(consumed)
}

fn is_modifier_key(key: u16) -> bool {
    matches!(
        key,
        VK_SHIFT
            | VK_LSHIFT
            | VK_RSHIFT
            | VK_CONTROL
            | VK_LCONTROL
            | VK_RCONTROL
            | VK_MENU
            | VK_LMENU
            | VK_RMENU
            | VK_LWIN
            | VK_RWIN
    )
}

fn normalized_state_key(key: u16, scan_code: u32, flags: u32) -> u16 {
    match key {
        VK_SHIFT => {
            if scan_code & 0xff == 0x36 {
                VK_RSHIFT
            } else {
                VK_LSHIFT
            }
        }
        VK_CONTROL => {
            if flags & LLKHF_EXTENDED != 0 {
                VK_RCONTROL
            } else {
                VK_LCONTROL
            }
        }
        VK_MENU => {
            if flags & LLKHF_EXTENDED != 0 {
                VK_RMENU
            } else {
                VK_LMENU
            }
        }
        _ => key,
    }
}

fn is_shortcut_chord(state: &KeyboardState) -> bool {
    let modifiers = state.modifiers();
    let altgr = state.is_down(VK_RMENU);
    modifiers.win || ((modifiers.ctrl || modifiers.alt) && !altgr)
}

fn active_keyboard_layout() -> HKL {
    // SAFETY: the returned handles and thread IDs are only passed back to
    // user32 for layout lookup; null foreground windows are explicitly handled.
    unsafe {
        let foreground = GetForegroundWindow();
        let thread = if foreground.is_null() {
            0
        } else {
            GetWindowThreadProcessId(foreground, ptr::null_mut())
        };
        GetKeyboardLayout(thread)
    }
}

fn translate_key_for_active_layout(
    virtual_key: u32,
    scan_code: u32,
    state: &KeyboardState,
) -> KeyTranslation {
    translate_key(virtual_key, scan_code, state, active_keyboard_layout())
}

fn translate_key(
    virtual_key: u32,
    scan_code: u32,
    state: &KeyboardState,
    layout: HKL,
) -> KeyTranslation {
    let mut buffer = [0_u16; 8];
    // Bit 2 prevents dead-key and other kernel keyboard state from being
    // modified. If translation is ambiguous, the physical event is passed on.
    let translated = unsafe {
        ToUnicodeEx(
            virtual_key,
            scan_code,
            state.keys.as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as i32,
            1 << 2,
            layout,
        )
    };
    if translated != 1 {
        return KeyTranslation::Pass;
    }

    char::from_u32(buffer[0] as u32).map_or(KeyTranslation::Pass, classify_character)
}

fn classify_character(ch: char) -> KeyTranslation {
    // Lowercase x is a convenient alias for EWTS underscore, which emits a
    // regular word space. Uppercase X retains its standard EWTS meaning.
    if ch == 'x' {
        KeyTranslation::Ewts('_')
    } else if ch.is_ascii_alphabetic() {
        if is_supported_ewts_letter(ch) {
            KeyTranslation::Ewts(ch)
        } else {
            KeyTranslation::Suppress
        }
    } else if ch == ' ' || ch.is_ascii_digit() || ch.is_ascii_punctuation() {
        KeyTranslation::Ewts(ch)
    } else {
        KeyTranslation::Pass
    }
}

fn is_supported_ewts_letter(ch: char) -> bool {
    // Single-letter tokens and prefixes accepted by the EWTS converter.
    // q has no lowercase token; unsupported capitals such as L must not leak
    // into the foreground application as Latin text.
    "abcdefghijklmnoprstuvwyzADHIMNRSTUWXY".contains(ch)
}

fn send_replacement(edit: &Replacement) {
    let mut inputs = Vec::with_capacity(edit.backspaces * 2 + edit.text.len() * 2);
    for _ in 0..edit.backspaces {
        inputs.push(key_input(VK_BACK, 0, 0));
        inputs.push(key_input(VK_BACK, 0, KEYEVENTF_KEYUP));
    }
    for unit in edit.text.encode_utf16() {
        inputs.push(key_input(0, unit, KEYEVENTF_UNICODE));
        inputs.push(key_input(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
    }
    if !inputs.is_empty() {
        // SAFETY: `inputs` is a contiguous array of initialized INPUT values.
        unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            )
        };
    }
}

fn send_raw_key(key: u16, is_down: bool) {
    let input = key_input(key, 0, if is_down { 0 } else { KEYEVENTF_KEYUP });
    // SAFETY: `input` is one initialized INPUT value.
    unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) };
}

fn key_input(vk: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INPUT_MARKER,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_layout_translated_ewts_characters() {
        assert_eq!(classify_character('t'), KeyTranslation::Ewts('t'));
        assert_eq!(classify_character('T'), KeyTranslation::Ewts('T'));
        assert_eq!(classify_character('@'), KeyTranslation::Ewts('@'));
        assert_eq!(classify_character('&'), KeyTranslation::Ewts('&'));
        assert_eq!(classify_character('1'), KeyTranslation::Ewts('1'));
    }

    #[test]
    fn lowercase_x_is_a_regular_word_space_alias() {
        assert_eq!(classify_character('x'), KeyTranslation::Ewts('_'));
        assert_eq!(classify_character('X'), KeyTranslation::Ewts('X'));
    }

    #[test]
    fn unsupported_or_non_ascii_layout_output_is_safe() {
        assert_eq!(classify_character('L'), KeyTranslation::Suppress);
        assert_eq!(classify_character('q'), KeyTranslation::Suppress);
        assert_eq!(classify_character('\t'), KeyTranslation::Pass);
        assert_eq!(classify_character('\u{00e9}'), KeyTranslation::Pass);
    }

    #[test]
    fn hook_state_tracks_modifiers_before_windows_updates_async_state() {
        let mut state = KeyboardState { keys: [0; 256] };
        assert_eq!(state.modifiers(), Modifiers::default());

        state.update(VK_LSHIFT, true);
        assert!(state.modifiers().shift);
        assert!(state.is_down(VK_SHIFT));
        state.update(VK_LSHIFT, false);
        assert!(!state.modifiers().shift);
    }

    #[test]
    fn altgr_is_text_input_but_regular_control_chords_are_shortcuts() {
        let mut state = KeyboardState { keys: [0; 256] };
        state.update(VK_LCONTROL, true);
        state.update(normalized_state_key(VK_MENU, 0x38, LLKHF_EXTENDED), true);
        assert!(!is_shortcut_chord(&state));

        state.update(VK_RMENU, false);
        assert!(is_shortcut_chord(&state));
    }

    #[test]
    fn generic_modifier_events_are_normalized_to_their_physical_side() {
        assert_eq!(normalized_state_key(VK_SHIFT, 0x2a, 0), VK_LSHIFT);
        assert_eq!(normalized_state_key(VK_SHIFT, 0x36, 0), VK_RSHIFT);
        assert_eq!(
            normalized_state_key(VK_CONTROL, 0x1d, LLKHF_EXTENDED),
            VK_RCONTROL
        );
        assert_eq!(
            normalized_state_key(VK_MENU, 0x38, LLKHF_EXTENDED),
            VK_RMENU
        );
    }

    #[test]
    fn mouse_button_presses_end_composition() {
        assert!(is_mouse_button_down(WM_LBUTTONDOWN as usize));
        assert!(is_mouse_button_down(WM_RBUTTONDOWN as usize));
        assert!(is_mouse_button_down(WM_MBUTTONDOWN as usize));
        assert!(is_mouse_button_down(WM_XBUTTONDOWN as usize));
        assert!(!is_mouse_button_down(
            windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE as usize
        ));
    }
}
