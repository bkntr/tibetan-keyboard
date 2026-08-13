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
            GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
            KEYEVENTF_UNICODE, SendInput, VK_BACK, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
        },
        WindowsAndMessaging::{
            CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, MB_ICONERROR, MB_OK, MessageBoxW,
            SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
            WM_SYSKEYDOWN, WM_SYSKEYUP,
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
    enabled: bool,
    proxy: EventLoopProxy<UserEvent>,
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
    hook: HHOOK,
}

impl Application {
    fn new(config: Config, proxy: EventLoopProxy<UserEvent>) -> Result<Self, Box<dyn Error>> {
        let hotkey = config.parsed_hotkey()?;
        HOOK_RUNTIME
            .set(Mutex::new(HookRuntime {
                composer: Composer::default(),
                hotkey_matcher: HotkeyMatcher::new(hotkey),
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
            hook: ptr::null_mut(),
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
        self.hook = unsafe {
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                ptr::null_mut(),
                0,
            )
        };
        if self.hook.is_null() {
            return Err("SetWindowsHookExW failed".into());
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
        if !self.hook.is_null() {
            // SAFETY: this is the hook handle returned by SetWindowsHookExW.
            unsafe { UnhookWindowsHookEx(self.hook) };
            self.hook = ptr::null_mut();
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
        let modifiers = current_modifiers();
        let key = event.vkCode as u16;
        let hotkey_match = runtime.hotkey_matcher.handle(key, is_down, modifiers);
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

        if process_normally && runtime.enabled {
            let control_chord = modifiers.ctrl || modifiers.alt || modifiers.win;
            let handled = if key == VK_BACK {
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
            } else if !control_chord {
                match translate_virtual_key(event.vkCode, modifiers.shift) {
                    KeyTranslation::Ewts(ch) => {
                        if is_down {
                            actions.push(SynthAction::Replacement(runtime.composer.push(ch)));
                        }
                        true
                    }
                    KeyTranslation::Suppress => true,
                    KeyTranslation::Pass => {
                        if is_down {
                            runtime.composer.commit();
                        }
                        false
                    }
                }
            } else {
                if is_down {
                    runtime.composer.commit();
                }
                false
            };

            suppress |= handled;
            if reprocess_suppressed_event && !handled {
                actions.push(SynthAction::RawKey { key, is_down });
            }
        } else if process_normally && reprocess_suppressed_event {
            actions.push(SynthAction::RawKey { key, is_down });
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

    for key in keys {
        if runtime.enabled && !control_chord {
            match translate_virtual_key(key as u32, modifiers.shift) {
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

fn current_modifiers() -> Modifiers {
    // SAFETY: GetAsyncKeyState accepts these documented virtual-key constants.
    unsafe {
        Modifiers {
            ctrl: GetAsyncKeyState(VK_CONTROL as i32) < 0,
            alt: GetAsyncKeyState(VK_MENU as i32) < 0,
            shift: GetAsyncKeyState(VK_SHIFT as i32) < 0,
            win: GetAsyncKeyState(VK_LWIN as i32) < 0 || GetAsyncKeyState(VK_RWIN as i32) < 0,
        }
    }
}

fn translate_virtual_key(vk: u32, shift: bool) -> KeyTranslation {
    if let Some(ch) = virtual_key_to_ewts(vk, shift) {
        KeyTranslation::Ewts(ch)
    } else if matches!(vk, 0x41..=0x5A) {
        KeyTranslation::Suppress
    } else {
        KeyTranslation::Pass
    }
}

fn virtual_key_to_ewts(vk: u32, shift: bool) -> Option<char> {
    match vk {
        // Lowercase x is a convenient alias for EWTS underscore, which emits
        // a regular word space. Shift+X retains its standard EWTS meaning.
        0x58 if !shift => Some('_'),
        0x41..=0x5A => {
            let ch = char::from_u32(vk)?;
            let ch = if shift { ch } else { ch.to_ascii_lowercase() };
            is_supported_ewts_letter(ch).then_some(ch)
        }
        0x30..=0x39 => {
            if shift {
                Some(")!@#$%^&*(".chars().nth((vk - 0x30) as usize)?)
            } else {
                char::from_u32(vk)
            }
        }
        0x20 => Some(' '),
        0xBA => Some(if shift { ':' } else { ';' }),
        0xBB => Some(if shift { '+' } else { '=' }),
        0xBC => Some(if shift { '<' } else { ',' }),
        0xBD => Some(if shift { '_' } else { '-' }),
        0xBE => Some(if shift { '>' } else { '.' }),
        0xBF => Some(if shift { '?' } else { '/' }),
        0xC0 => Some(if shift { '~' } else { '`' }),
        0xDB => Some(if shift { '{' } else { '[' }),
        0xDC => Some(if shift { '|' } else { '\\' }),
        0xDD => Some(if shift { '}' } else { ']' }),
        0xDE => Some(if shift { '"' } else { '\'' }),
        _ => None,
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
    fn maps_us_keyboard_ewts_characters() {
        assert_eq!(virtual_key_to_ewts(0x54, false), Some('t'));
        assert_eq!(virtual_key_to_ewts(0x54, true), Some('T'));
        assert_eq!(virtual_key_to_ewts(0x32, true), Some('@'));
        assert_eq!(virtual_key_to_ewts(0xBD, true), Some('_'));
        assert_eq!(virtual_key_to_ewts(0xBF, false), Some('/'));
    }

    #[test]
    fn lowercase_x_is_a_regular_word_space_alias() {
        assert_eq!(
            translate_virtual_key(0x58, false),
            KeyTranslation::Ewts('_')
        );
        assert_eq!(translate_virtual_key(0x58, true), KeyTranslation::Ewts('X'));
    }

    #[test]
    fn unsupported_latin_letters_are_suppressed() {
        assert_eq!(translate_virtual_key(0x4C, true), KeyTranslation::Suppress);
        assert_eq!(translate_virtual_key(0x51, false), KeyTranslation::Suppress);
        assert_eq!(translate_virtual_key(0x09, false), KeyTranslation::Pass);
    }
}
