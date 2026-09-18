#define AppName "WakeFrame"
#ifndef AppVersion
#define AppVersion "0.1.0"
#endif
#define Publisher "WakeFrame"
#define AppExeName "wakeframe-ui.exe"

[Setup]
AppId={{55E2C6F0-C60A-4E12-A521-27E34C52A182}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#Publisher}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\{#AppExeName}
SetupIconFile=..\assets\WakeFrame.ico
OutputDir=..\dist\installer
OutputBaseFilename=WakeFrame-Setup
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "startup"; Description: "Start WakeFrame with Windows"; GroupDescription: "Startup options:"; Flags: checkedonce
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked

[Files]
Source: "..\dist\WakeFrame\wakeframe-agent.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\WakeFrame\wakeframe-ui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\WakeFrame\wakeframe-player.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\WakeFrame\libmpv-2.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\WakeFrame\README.md"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "..\dist\WakeFrame\assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\WakeFrame Settings"; Filename: "{app}\{#AppExeName}"
Name: "{autoprograms}\WakeFrame Agent"; Filename: "{app}\wakeframe-agent.exe"; Parameters: "idle"
Name: "{autodesktop}\WakeFrame"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "WakeFrameAgent"; ValueData: """{app}\wakeframe-agent.exe"" startup"; Tasks: startup; Flags: uninsdeletevalue

[Run]
Filename: "{app}\wakeframe-agent.exe"; Parameters: "idle"; Description: "Start WakeFrame now"; Flags: nowait postinstall skipifsilent
Filename: "{app}\{#AppExeName}"; Description: "Open WakeFrame settings"; Flags: nowait postinstall skipifsilent unchecked

[UninstallRun]
Filename: "taskkill"; Parameters: "/IM wakeframe-agent.exe /F"; Flags: runhidden
Filename: "taskkill"; Parameters: "/IM wakeframe-ui.exe /F"; Flags: runhidden
Filename: "taskkill"; Parameters: "/IM wakeframe-player.exe /F"; Flags: runhidden