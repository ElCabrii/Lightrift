#ifndef AppVersion
  #define AppVersion "1.0.1"
#endif
#ifndef BinaryPath
  #define BinaryPath "..\target\release\lightrift.exe"
#endif
#ifndef ReleaseDir
  #define ReleaseDir "..\dist"
#endif

[Setup]
AppId={{ED9BB099-526A-4ECD-B073-80598289A38E}
AppName=Lightrift
AppVersion={#AppVersion}
AppPublisher=ElCabrii
AppPublisherURL=https://github.com/ElCabrii/Lightrift
AppSupportURL=https://github.com/ElCabrii/Lightrift/issues
AppUpdatesURL=https://github.com/ElCabrii/Lightrift/releases
DefaultDirName={localappdata}\Programs\Lightrift
DefaultGroupName=Lightrift
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
UninstallDisplayIcon={app}\Lightrift.exe
SetupIconFile=..\assets\lightrift.ico
OutputDir={#ReleaseDir}
OutputBaseFilename=Lightrift-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
CloseApplicationsFilter=Lightrift.exe
RestartApplications=no
VersionInfoVersion={#AppVersion}
VersionInfoProductName=Lightrift

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked

[Files]
Source: "{#BinaryPath}"; DestDir: "{app}"; DestName: "Lightrift.exe"; Flags: ignoreversion
Source: "installed.flag"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD-PARTY-NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\docs\*.md"; DestDir: "{app}\docs"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\Lightrift"; Filename: "{app}\Lightrift.exe"; WorkingDir: "{app}"
Name: "{userdesktop}\Lightrift"; Filename: "{app}\Lightrift.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\Lightrift.exe"; Description: "Launch Lightrift"; Flags: nowait postinstall skipifsilent

; User builds live outside {app}; uninstall deliberately preserves them.
