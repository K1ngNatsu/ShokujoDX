; Inno Setup 6 script of the Windows installer (ADR 0043).
;
; tools\package-windows.ps1 -Installer runs it on the package folder:
;   ISCC /DVersion=0.1.0 /DSource=<folder> /DOutputDir=<dist> /DOutputName=<name> installer.iss
;
; The installer installs for the current user (no administrator rights), or
; for all users when the user selects it. It adds a Start menu entry, the
; .embproj file type and, as a choice, "Open with" for machine files.

#define AppName "ShokujoDX"
#define AppExe "ShokujoDX.exe"
#ifndef Version
  #define Version "0.0.0"
#endif
#ifndef Source
  #define Source "..\..\dist\stage"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif
#ifndef OutputName
  #define OutputName "ShokujoDX-setup"
#endif

[Setup]
; The AppId is the same for each version, so a new version updates the old one.
AppId={{4FDD400B-23A4-46D2-BC5E-2B6A425A358A}
AppName={#AppName}
AppVersion={#Version}
AppVerName={#AppName} {#Version}
AppPublisher=ShokujoDX project
VersionInfoVersion={#Version}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
LicenseFile=..\..\LICENSE
SetupIconFile=..\..\data\icon\app.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
OutputDir={#OutputDir}
OutputBaseFilename={#OutputName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes
CloseApplications=yes

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "de"; MessagesFile: "compiler:Languages\German.isl"
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "ja"; MessagesFile: "compiler:Languages\Japanese.isl"

[CustomMessages]
en.FileTypes=File types:
de.FileTypes=Dateitypen:
es.FileTypes=Tipos de archivo:
ja.FileTypes=ファイルの種類:
en.MachineFiles=Add {#AppName} to "Open with" for machine files (DST, PES, VP3, JEF and more)
de.MachineFiles={#AppName} zu „Öffnen mit“ für Maschinendateien hinzufügen (DST, PES, VP3, JEF und mehr)
es.MachineFiles=Añadir {#AppName} a «Abrir con» para archivos de máquina (DST, PES, VP3, JEF y más)
ja.MachineFiles=ミシンのファイル（DST、PES、VP3、JEF など）の「プログラムから開く」に {#AppName} を追加する
en.ProjectType=ShokujoDX project
de.ProjectType=ShokujoDX-Projekt
es.ProjectType=Proyecto de ShokujoDX
ja.ProjectType=ShokujoDX プロジェクト
en.MachineType=Machine embroidery file
en.Thumbnails=Show the pictures of designs in Explorer (projects, PES, PEC, VP3, DST, HUS, JEF)
de.Thumbnails=Bilder der Designs im Explorer zeigen (Projekte, PES, PEC, VP3, DST, HUS, JEF)
es.Thumbnails=Mostrar las imágenes de los diseños en el Explorador (proyectos, PES, PEC, VP3, DST, HUS, JEF)
ja.Thumbnails=エクスプローラーにデザインの画像を表示する（プロジェクト、PES、PEC、VP3、DST、HUS、JEF）
de.MachineType=Maschinenstickdatei
es.MachineType=Archivo de bordado de máquina
ja.MachineType=ミシン刺しゅうファイル

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "machinefiles"; Description: "{cm:MachineFiles}"; GroupDescription: "{cm:FileTypes}"
Name: "thumbnails"; Description: "{cm:Thumbnails}"; GroupDescription: "{cm:FileTypes}"

[Files]
Source: "{#Source}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; The project files of the program open with it.
Root: HKA; Subkey: "Software\Classes\.embproj"; ValueType: string; ValueName: ""; ValueData: "ShokujoDX.Project"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.embproj\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.Project"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\ShokujoDX.Project"; ValueType: string; ValueName: ""; ValueData: "{cm:ProjectType}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\ShokujoDX.Project\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"
Root: HKA; Subkey: "Software\Classes\ShokujoDX.Project\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""
; Machine files: only "Open with"; the program that opens them now stays.
Root: HKA; Subkey: "Software\Classes\ShokujoDX.MachineFile"; ValueType: string; ValueName: ""; ValueData: "{cm:MachineType}"; Flags: uninsdeletekey; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\ShokujoDX.MachineFile\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\ShokujoDX.MachineFile\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.dst\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.dsb\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.dsz\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.exp\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.hus\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.jef\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.pec\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.pes\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.sew\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.shv\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.vp3\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\.xxx\OpenWithProgids"; ValueType: string; ValueName: "ShokujoDX.MachineFile"; ValueData: ""; Flags: uninsdeletevalue; Tasks: machinefiles
; The thumbnail handler (ADR 0046): Windows runs it in its own process
; (it takes the file as a stream), so an error cannot stop Explorer.
Root: HKA; Subkey: "Software\Classes\CLSID\{{F1F7156E-ACB6-4891-9AC2-7E32E078B242}"; ValueType: string; ValueName: ""; ValueData: "ShokujoDX thumbnails"; Flags: uninsdeletekey; Tasks: thumbnails
Root: HKA; Subkey: "Software\Classes\CLSID\{{F1F7156E-ACB6-4891-9AC2-7E32E078B242}\InprocServer32"; ValueType: string; ValueName: ""; ValueData: "{app}\emb_thumbnail.dll"; Tasks: thumbnails
Root: HKA; Subkey: "Software\Classes\CLSID\{{F1F7156E-ACB6-4891-9AC2-7E32E078B242}\InprocServer32"; ValueType: string; ValueName: "ThreadingModel"; ValueData: "Apartment"; Tasks: thumbnails
Root: HKA; Subkey: "Software\Classes\.embproj\ShellEx\{{e357fccd-a995-4576-b01f-234630154e96}"; ValueType: string; ValueName: ""; ValueData: "{{F1F7156E-ACB6-4891-9AC2-7E32E078B242}"; Flags: uninsdeletekey; Tasks: thumbnails
Root: HKA; Subkey: "Software\Classes\.pes\ShellEx\{{e357fccd-a995-4576-b01f-234630154e96}"; ValueType: string; ValueName: ""; ValueData: "{{F1F7156E-ACB6-4891-9AC2-7E32E078B242}"; Flags: uninsdeletekey; Tasks: thumbnails
Root: HKA; Subkey: "Software\Classes\.pec\ShellEx\{{e357fccd-a995-4576-b01f-234630154e96}"; ValueType: string; ValueName: ""; ValueData: "{{F1F7156E-ACB6-4891-9AC2-7E32E078B242}"; Flags: uninsdeletekey; Tasks: thumbnails
Root: HKA; Subkey: "Software\Classes\.vp3\ShellEx\{{e357fccd-a995-4576-b01f-234630154e96}"; ValueType: string; ValueName: ""; ValueData: "{{F1F7156E-ACB6-4891-9AC2-7E32E078B242}"; Flags: uninsdeletekey; Tasks: thumbnails
Root: HKA; Subkey: "Software\Classes\.dst\ShellEx\{{e357fccd-a995-4576-b01f-234630154e96}"; ValueType: string; ValueName: ""; ValueData: "{{F1F7156E-ACB6-4891-9AC2-7E32E078B242}"; Flags: uninsdeletekey; Tasks: thumbnails
Root: HKA; Subkey: "Software\Classes\.hus\ShellEx\{{e357fccd-a995-4576-b01f-234630154e96}"; ValueType: string; ValueName: ""; ValueData: "{{F1F7156E-ACB6-4891-9AC2-7E32E078B242}"; Flags: uninsdeletekey; Tasks: thumbnails
Root: HKA; Subkey: "Software\Classes\.jef\ShellEx\{{e357fccd-a995-4576-b01f-234630154e96}"; ValueType: string; ValueName: ""; ValueData: "{{F1F7156E-ACB6-4891-9AC2-7E32E078B242}"; Flags: uninsdeletekey; Tasks: thumbnails
; The program in the "Open with" list of Windows.
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "{#AppName}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".embproj"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".dst"; ValueData: ""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".dsb"; ValueData: ""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".dsz"; ValueData: ""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".exp"; ValueData: ""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".hus"; ValueData: ""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".jef"; ValueData: ""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".pec"; ValueData: ""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".pes"; ValueData: ""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".sew"; ValueData: ""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".shv"; ValueData: ""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".vp3"; ValueData: ""; Tasks: machinefiles
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".xxx"; ValueData: ""; Tasks: machinefiles

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent
