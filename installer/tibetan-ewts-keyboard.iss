#define AppName "Tibetan EWTS Keyboard"
#define AppExeName "tibetan-ewts-keyboard.exe"
#define AppVersion GetEnv("TIBETAN_EWTS_APP_VERSION")
#define AppArch GetEnv("TIBETAN_EWTS_APP_ARCH")
#define SourceExe GetEnv("TIBETAN_EWTS_SOURCE_EXE")
#define OutputDir GetEnv("TIBETAN_EWTS_OUTPUT_DIR")
#define OutputName GetEnv("TIBETAN_EWTS_OUTPUT_NAME")
#define IconFile GetEnv("TIBETAN_EWTS_ICON_FILE")

#if AppVersion == ""
  #error TIBETAN_EWTS_APP_VERSION must be set
#endif
#if AppArch == ""
  #error TIBETAN_EWTS_APP_ARCH must be set to x64 or arm64
#endif
#if SourceExe == ""
  #error TIBETAN_EWTS_SOURCE_EXE must be set
#endif
#if OutputDir == ""
  #error TIBETAN_EWTS_OUTPUT_DIR must be set
#endif
#if OutputName == ""
  #error TIBETAN_EWTS_OUTPUT_NAME must be set
#endif
#if IconFile == ""
  #error TIBETAN_EWTS_ICON_FILE must be set
#endif

[Setup]
AppId={{2E96B301-A4E2-4A4F-81E2-D92C84F20AD8}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=bkntr
AppPublisherURL=https://github.com/bkntr/tibetan-keyboard
AppSupportURL=https://github.com/bkntr/tibetan-keyboard/issues
AppUpdatesURL=https://github.com/bkntr/tibetan-keyboard/releases/latest
DefaultDirName={localappdata}\Programs\Tibetan EWTS Keyboard
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir={#OutputDir}
OutputBaseFilename={#OutputName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
SetupIconFile={#IconFile}
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\{#AppExeName}
VersionInfoVersion={#AppVersion}
VersionInfoCompany=bkntr
VersionInfoDescription={#AppName} installer
VersionInfoProductName={#AppName}
VersionInfoProductVersion={#AppVersion}

#if AppArch == "x64"
ArchitecturesAllowed=x64compatible and not arm64
ArchitecturesInstallIn64BitMode=x64compatible and not arm64
#elif AppArch == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
  #error Unsupported AppArch
#endif

[Tasks]
Name: "startup"; Description: "Start automatically when I sign in"; GroupDescription: "Additional tasks:"; Flags: unchecked
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional tasks:"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "{#AppExeName}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "TibetanEWTSKeyboard"; ValueData: """{app}\{#AppExeName}"""; Flags: uninsdeletevalue; Tasks: startup

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
