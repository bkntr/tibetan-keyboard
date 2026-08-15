$ErrorActionPreference = 'Stop'

$workspace = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $workspace 'target\debug\tibetan-ewts-keyboard.exe'
& cargo build --manifest-path (Join-Path $workspace 'Cargo.toml')
if ($LASTEXITCODE -ne 0) {
    throw "Unable to build the debug executable."
}

$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("tibetan-ewts-e2e-" + [guid]::NewGuid().ToString('N'))
$configPath = Join-Path $testRoot 'settings.toml'
New-Item -ItemType Directory -Path $testRoot | Out-Null

$oldConfig = $env:TIBETAN_EWTS_CONFIG
$keyboard = $null
try {
    $env:TIBETAN_EWTS_CONFIG = $configPath
    Set-Content -LiteralPath $configPath -Encoding ASCII -Value @(
        'hotkey = "Shift+Space+Space"'
        'enabled_on_start = false'
    )
    $keyboard = Start-Process -FilePath $exe -PassThru -WindowStyle Hidden
    Start-Sleep -Milliseconds 1800
    if (-not (Get-Process -Id $keyboard.Id -ErrorAction SilentlyContinue)) {
        throw 'Keyboard process exited during startup.'
    }

    Add-Type -AssemblyName System.Windows.Forms
    Add-Type -AssemblyName System.Drawing
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class TibetanEwtsTestInput
{
    private const uint KeyUp = 0x0002;
    private const uint MouseLeftDown = 0x0002;
    private const uint MouseLeftUp = 0x0004;

    [DllImport("user32.dll")]
    private static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extraInfo);

    [DllImport("user32.dll")]
    private static extern bool SetCursorPos(int x, int y);

    [DllImport("user32.dll")]
    private static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);

    public static void Down(byte key) { keybd_event(key, 0, 0, UIntPtr.Zero); }
    public static void Up(byte key) { keybd_event(key, 0, KeyUp, UIntPtr.Zero); }
    public static void Press(byte key) { Down(key); Up(key); }

    public static void Click(int x, int y)
    {
        SetCursorPos(x, y);
        mouse_event(MouseLeftDown, 0, 0, 0, UIntPtr.Zero);
        mouse_event(MouseLeftUp, 0, 0, 0, UIntPtr.Zero);
    }

    public static void ToggleTestHotkey()
    {
        Down(0x10); // Shift
        Press(0x20); // Space, first stroke
        Press(0x20); // Space, second stroke
        Up(0x10);
    }

    public static void CancelByReleasingShift()
    {
        Down(0x10); // Shift
        Press(0x20); // Space, first stroke
        Up(0x10); // Must cancel and replay the suppressed Space
    }

    public static void TypeTibetanSpacesAndUnsupportedLetter()
    {
        Press(0x4F); // o
        Down(0x10);
        Press(0x4D); // M
        Up(0x10);
        Press(0x20); // Tibetan tsheg
        Press(0x4B); // k
        Press(0x58); // x -> regular word space
        Down(0x10);
        Press(0x4C); // unsupported L -> suppressed
        Up(0x10);

        Press(0x47); // g
        Press(0x52); // r
        Down(0x10);
        Press(0x41); // A -> long vowel on the active gr stack
        Up(0x10);
        Press(0x58); // finish the stack with a regular word space

        Press(0x47); // g
        Press(0x52); // r
        Press(0x14); // Caps Lock on
        Press(0x41); // a key -> uppercase A
        Press(0x14); // Caps Lock off
        Press(0x58); // finish the stack with a regular word space
    }
}
'@

    $form = [System.Windows.Forms.Form]::new()
    $form.Text = 'Tibetan EWTS Keyboard E2E'
    $form.Width = 640
    $form.Height = 180
    $form.TopMost = $true

    $box = [System.Windows.Forms.TextBox]::new()
    $box.Dock = [System.Windows.Forms.DockStyle]::Fill
    $box.Font = [System.Drawing.Font]::new('Microsoft Himalaya', 28)
    $form.Controls.Add($box)

    $script:e2eStep = 0
    $script:replayPassed = $false
    $script:keyboardText = $null
    $script:mouseText = $null
    $timer = [System.Windows.Forms.Timer]::new()
    $timer.Interval = 650
    $timer.Add_Tick({
        $script:e2eStep++
        switch ($script:e2eStep) {
            1 {
                $form.Activate()
                $box.Focus()
                [TibetanEwtsTestInput]::CancelByReleasingShift()
            }
            2 {
                $script:replayPassed = $box.Text -eq ' '
                $box.Clear()
                [TibetanEwtsTestInput]::ToggleTestHotkey()
            }
            3 {
                [TibetanEwtsTestInput]::TypeTibetanSpacesAndUnsupportedLetter()
            }
            4 {
                $script:keyboardText = $box.Text
                $box.Clear()
                [TibetanEwtsTestInput]::Press(0x4E) # n
            }
            5 {
                $center = $box.PointToScreen(
                    [System.Drawing.Point]::new($box.ClientSize.Width / 2, $box.ClientSize.Height / 2)
                )
                [TibetanEwtsTestInput]::Click($center.X, $center.Y)
            }
            6 {
                $box.SelectionStart = 0
                [TibetanEwtsTestInput]::Press(0x47) # g
            }
            7 {
                $script:mouseText = $box.Text
                $timer.Stop()
                $form.Close()
            }
        }
    })
    $form.Add_Shown({ $box.Focus(); $timer.Start() })
    [void]$form.ShowDialog()

    $expected = -join @(
        [char]0x0F68,
        [char]0x0F7C,
        [char]0x0F7E,
        [char]0x0F0B,
        [char]0x0F40,
        ' ',
        [char]0x0F42,
        [char]0x0FB2,
        [char]0x0F71,
        ' ',
        [char]0x0F42,
        [char]0x0FB2,
        [char]0x0F71,
        ' '
    )
    if (-not $script:replayPassed) {
        throw 'An incomplete Shift+Space sequence was not replayed as a normal Space.'
    }
    if ($script:keyboardText -ne $expected) {
        $actualCodes = ($script:keyboardText.ToCharArray() | ForEach-Object { 'U+{0:X4}' -f [int]$_ }) -join ' '
        throw "Expected '$expected', got '$script:keyboardText' ($actualCodes)."
    }
    $expectedAfterMouseMove = -join @([char]0x0F42, [char]0x0F53)
    if ($script:mouseText -ne $expectedAfterMouseMove) {
        $actualCodes = ($script:mouseText.ToCharArray() | ForEach-Object { 'U+{0:X4}' -f [int]$_ }) -join ' '
        throw "Expected mouse click to end composition and produce '$expectedAfterMouseMove', got '$script:mouseText' ($actualCodes)."
    }
    Write-Output "E2E passed: hotkey replay/toggle, Tibetan input behavior, and mouse-click composition reset"
}
finally {
    if ($keyboard -and (Get-Process -Id $keyboard.Id -ErrorAction SilentlyContinue)) {
        Stop-Process -Id $keyboard.Id
        Wait-Process -Id $keyboard.Id -ErrorAction SilentlyContinue
    }
    $env:TIBETAN_EWTS_CONFIG = $oldConfig

    $resolved = [System.IO.Path]::GetFullPath($testRoot)
    $tempBase = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    if ($resolved.StartsWith($tempBase, [System.StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path -Leaf $resolved).StartsWith('tibetan-ewts-e2e-')) {
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
