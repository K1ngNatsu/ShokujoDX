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
; An older version is removed before the new one is installed (v0.3 plan, Stage 1).
en.Upgrade=Upgrade:
de.Upgrade=Aktualisierung:
es.Upgrade=Actualización:
ja.Upgrade=アップグレード:
en.OldVersion={#AppName} %1 (%2) is removed first. Your settings and your designs stay.
de.OldVersion={#AppName} %1 (%2) wird zuerst entfernt. Ihre Einstellungen und Ihre Designs bleiben.
es.OldVersion={#AppName} %1 (%2) se quita primero. Su configuración y sus diseños se conservan.
ja.OldVersion={#AppName} %1 (%2) を先に削除します。設定とデザインはそのまま残ります。
en.ForThisUser=for this user
de.ForThisUser=für diesen Benutzer
es.ForThisUser=para este usuario
ja.ForThisUser=このユーザー用
en.ForAllUsers=for all users
de.ForAllUsers=für alle Benutzer
es.ForAllUsers=para todos los usuarios
ja.ForAllUsers=すべてのユーザー用
en.RemoveFailed=The older version of {#AppName} (%1) could not be removed. Remove it in Settings > Apps, then start this installer again.
de.RemoveFailed=Die ältere Version von {#AppName} (%1) konnte nicht entfernt werden. Entfernen Sie sie unter Einstellungen > Apps und starten Sie dieses Installationsprogramm dann erneut.
es.RemoveFailed=No se pudo quitar la versión anterior de {#AppName} (%1). Quítela en Configuración > Aplicaciones y vuelva a iniciar este instalador.
ja.RemoveFailed=古いバージョンの {#AppName} (%1) を削除できませんでした。[設定] > [アプリ] で削除してから、このインストーラーをもう一度起動してください。
; The check for a new version (ADR 0109): the program reads the choice from
; install.ini next to it.
en.Updates=New versions:
de.Updates=Neue Versionen:
es.Updates=Versiones nuevas:
ja.Updates=新しいバージョン:
en.UpdateCheck=Look for a new version when {#AppName} starts (it asks GitHub for the newest version; it sends no data about you or your designs, and it installs nothing)
de.UpdateCheck=Beim Start von {#AppName} nach einer neuen Version suchen (fragt GitHub nach der neuesten Version; sendet keine Daten über Sie oder Ihre Designs und installiert nichts)
es.UpdateCheck=Buscar una versión nueva al iniciar {#AppName} (pregunta a GitHub por la versión más reciente; no envía datos sobre usted ni sobre sus diseños y no instala nada)
ja.UpdateCheck={#AppName} の起動時に新しいバージョンを確認する（GitHub に最新バージョンを問い合わせます。お客様やデザインのデータは送信せず、何もインストールしません）

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "machinefiles"; Description: "{cm:MachineFiles}"; GroupDescription: "{cm:FileTypes}"
Name: "thumbnails"; Description: "{cm:Thumbnails}"; GroupDescription: "{cm:FileTypes}"
Name: "updatecheck"; Description: "{cm:UpdateCheck}"; GroupDescription: "{cm:Updates}"

[Files]
Source: "{#Source}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[INI]
; The choice of the user for the check for a new version. The program
; applies it at its next start (update.rs).
Filename: "{app}\install.ini"; Section: "install"; Key: "update_check"; String: "yes"; Tasks: updatecheck
Filename: "{app}\install.ini"; Section: "install"; Key: "update_check"; String: "no"; Tasks: not updatecheck

[UninstallDelete]
Type: files; Name: "{app}\install.ini"

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

[Code]
// An older version is removed before the new one is installed (v0.3 plan,
// Stage 1). An install for this user and an install for all users have
// their own uninstall entries, so a change of the install mode left two
// copies before. The uninstaller removes only the files and the registry
// entries of the program: the settings (%APPDATA%\ShokujoDX) and the
// designs of the user stay.

const
  UninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{4FDD400B-23A4-46D2-BC5E-2B6A425A358A}_is1';

// The registry root of the installs for all users.
function MachineRoot: Integer;
begin
  if IsWin64 then
    Result := HKLM64
  else
    Result := HKLM;
end;

// The uninstaller and the version of the install in Root, if there is one.
function OldInstall(Root: Integer; var Uninstaller, Version: String): Boolean;
begin
  Result := RegQueryStringValue(Root, UninstallKey, 'UninstallString', Uninstaller);
  if Result then
  begin
    Uninstaller := RemoveQuotes(Uninstaller);
    if not RegQueryStringValue(Root, UninstallKey, 'DisplayVersion', Version) then
      Version := '?';
  end;
end;

// One line for the Ready page if there is an install in Root.
function OldLine(Root: Integer; Mode, NewLine: String): String;
var
  Uninstaller, Version: String;
begin
  Result := '';
  if OldInstall(Root, Uninstaller, Version) then
    Result := '      ' + FmtMessage(CustomMessage('OldVersion'), [Version, Mode]) + NewLine;
end;

function UpdateReadyMemo(Space, NewLine, MemoUserInfoInfo, MemoDirInfo, MemoTypeInfo,
  MemoComponentsInfo, MemoGroupInfo, MemoTasksInfo: String): String;
var
  Old: String;
begin
  Old := OldLine(HKCU, CustomMessage('ForThisUser'), NewLine)
    + OldLine(MachineRoot, CustomMessage('ForAllUsers'), NewLine);
  Result := '';
  if Old <> '' then
    Result := CustomMessage('Upgrade') + NewLine + Old + NewLine;
  if MemoDirInfo <> '' then
    Result := Result + MemoDirInfo + NewLine + NewLine;
  if MemoGroupInfo <> '' then
    Result := Result + MemoGroupInfo + NewLine + NewLine;
  if MemoTasksInfo <> '' then
    Result := Result + MemoTasksInfo + NewLine;
end;

// Runs the uninstaller of the install in Root with no questions, and waits
// until it is done. The uninstaller starts a copy of itself and stops, so
// the wait is for its uninstall entry to go. Returns '' or an error text.
function RemoveOld(Root: Integer): String;
var
  Uninstaller, Version: String;
  Code, Waited: Integer;
begin
  Result := '';
  if not OldInstall(Root, Uninstaller, Version) then
    Exit;
  Log('Removing the older version ' + Version + ': ' + Uninstaller);
  // ShellExec, so that the uninstaller of an install for all users can ask
  // for administrator rights.
  if not ShellExec('', Uninstaller, '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART', '', SW_HIDE,
    ewWaitUntilTerminated, Code) then
  begin
    Result := SysErrorMessage(Code);
    Exit;
  end;
  Waited := 0;
  while RegKeyExists(Root, UninstallKey) and (Waited < 60000) do
  begin
    Sleep(250);
    Waited := Waited + 250;
  end;
  if RegKeyExists(Root, UninstallKey) then
    Result := Version;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Error: String;
begin
  Error := RemoveOld(HKCU);
  if Error = '' then
    Error := RemoveOld(MachineRoot);
  Result := '';
  if Error <> '' then
    Result := FmtMessage(CustomMessage('RemoveFailed'), [Error]);
end;
