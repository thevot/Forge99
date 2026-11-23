; =====================================================================
; Forge99 — 64-bit Level Editor for Unreal Tournament 99
; Official Inno Setup Installer Script
; =====================================================================

#define MyAppName "Forge99"
#define MyAppVersion "0.25.0"
#define MyAppPublisher 'Nicola "thevot" Selenu & Forge99 Contributors'
#define MyAppURL "https://github.com/thevot/Forge99"
#define MyAppExeName "forge99.exe"

[Setup]
AppId={{D37E84B0-C27A-459E-9102-36E7B9A14899}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=..\LICENSE-MIT
OutputDir=..\dist
OutputBaseFilename=forge99-v{#MyAppVersion}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesAssociations=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "fileassoc_unr"; Description: "Associate .unr files with Forge99"; GroupDescription: "File Associations"
Name: "fileassoc_t3d"; Description: "Associate .t3d files with Forge99"; GroupDescription: "File Associations"

[Files]
Source: "..\target\release\forge99.exe"; DestDir: "{app}"; DestName: "{#MyAppExeName}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion

[Registry]
; Associate .unr (Unreal Tournament 99 Map)
Root: HKA; Subkey: "Software\Classes\.unr"; ValueType: string; ValueName: ""; ValueData: "Forge99.Map"; Flags: uninsdeletevalue; Tasks: fileassoc_unr
Root: HKA; Subkey: "Software\Classes\Forge99.Map"; ValueType: string; ValueName: ""; ValueData: "Unreal Tournament Map"; Flags: uninsdeletekey; Tasks: fileassoc_unr
Root: HKA; Subkey: "Software\Classes\Forge99.Map\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Tasks: fileassoc_unr
Root: HKA; Subkey: "Software\Classes\Forge99.Map\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc_unr

; Associate .t3d (Unreal Text 3D)
Root: HKA; Subkey: "Software\Classes\.t3d"; ValueType: string; ValueName: ""; ValueData: "Forge99.T3D"; Flags: uninsdeletevalue; Tasks: fileassoc_t3d
Root: HKA; Subkey: "Software\Classes\Forge99.T3D"; ValueType: string; ValueName: ""; ValueData: "Unreal Text 3D Level"; Flags: uninsdeletekey; Tasks: fileassoc_t3d
Root: HKA; Subkey: "Software\Classes\Forge99.T3D\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Tasks: fileassoc_t3d
Root: HKA; Subkey: "Software\Classes\Forge99.T3D\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc_t3d

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
