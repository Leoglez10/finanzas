; Inno Setup script: per-user install, no admin rights. Built by .github/workflows/release.yml.
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
[Setup]
AppId={{6F1D2C1E-5B7A-4E0B-9A39-2B9E3F5C8A41}
AppName=Finanzas
AppVersion={#AppVersion}
AppPublisher=Finanzas
DefaultDirName={localappdata}\Programs\Finanzas
DefaultGroupName=Finanzas
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..\..\dist
OutputBaseFilename=Finanzas-windows-x64-setup
SetupIconFile=finanzas.ico
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\finanzas.ico

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\..\target\release\finanzas.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\finanzas-mcp.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "finanzas.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Finanzas"; Filename: "{app}\finanzas.exe"; IconFilename: "{app}\finanzas.ico"
Name: "{autodesktop}\Finanzas"; Filename: "{app}\finanzas.exe"; IconFilename: "{app}\finanzas.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\finanzas.exe"; Description: "{cm:LaunchProgram,Finanzas}"; Flags: nowait postinstall skipifsilent
