#ifndef Arch
#define Arch "x64"
#endif

#ifndef BuildDir
#define BuildDir "..\..\target\release"
#endif

#ifndef OutputFile
#define OutputFile "Next_Tablet_Driver_Setup_" + Arch
#endif

[Setup]
AppName=Next Tablet Driver
AppVersion=1.26.1708.00
AppPublisher=iSweat
OutputBaseFilename={#OutputFile}

#if Arch == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#endif

DefaultDirName={commonpf}\NextTabletDriver
DefaultGroupName=Next Tablet Driver
UninstallDisplayIcon={app}\next_tablet_driver.exe
Compression=lzma2
SolidCompression=yes
OutputDir=..\..\user_mode_dist

PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog

AppMutex=NextTabletDriverMutex
CloseApplications=yes
DirExistsWarning=no
SetupMutex=NextTabletDriverSetupMutex

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#BuildDir}\next_tablet_driver.exe"; DestDir: "{app}"; Flags: ignoreversion restartreplace

[Icons]
Name: "{group}\Next Tablet Driver"; Filename: "{app}\next_tablet_driver.exe"
Name: "{commondesktop}\Next Tablet Driver"; Filename: "{app}\next_tablet_driver.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\next_tablet_driver.exe"; Description: "{cm:LaunchProgram,Next Tablet Driver}"; Flags: nowait postinstall skipifsilent
