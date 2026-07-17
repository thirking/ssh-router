Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"

!ifndef VERSION
  !error "VERSION is required"
!endif
!ifndef VERSION_QUAD
  !error "VERSION_QUAD is required"
!endif
!ifndef SOURCE_DIR
  !error "SOURCE_DIR is required"
!endif
!ifndef OUTPUT_FILE
  !error "OUTPUT_FILE is required"
!endif

Name "SSH Router"
OutFile "${OUTPUT_FILE}"
InstallDir "$LOCALAPPDATA\SSH Router"
InstallDirRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetOverwrite on
Icon "..\crates\gpui-app\assets\icon.ico"
UninstallIcon "..\crates\gpui-app\assets\icon.ico"
VIProductVersion "${VERSION_QUAD}"
VIAddVersionKey "ProductName" "SSH Router"
VIAddVersionKey "FileDescription" "SSH Router Installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "CompanyName" "sshrouter"
VIAddVersionKey "LegalCopyright" "Copyright (c) SSH Router contributors"

Var PassiveMode
Var RestartAfterInstall
Var ParentProcessId

!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\ssh-router.exe"
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipInteractivePage
!insertmacro MUI_PAGE_WELCOME
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipInteractivePage
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipInteractivePage
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  StrCpy $PassiveMode 0
  StrCpy $RestartAfterInstall 0
  StrCpy $ParentProcessId ""
  ${GetParameters} $0
  ClearErrors
  ${GetOptions} $0 "/P" $1
  IfErrors +2 0
    StrCpy $PassiveMode 1
  ClearErrors
  ${GetOptions} $0 "/R" $1
  IfErrors +2 0
    StrCpy $RestartAfterInstall 1
  ClearErrors
  ${GetOptions} $0 "/PARENT=" $1
  IfErrors +2 0
    StrCpy $ParentProcessId $1
FunctionEnd

Function SkipInteractivePage
  StrCmp $PassiveMode 1 0 +2
    Abort
FunctionEnd

Section "SSH Router" SEC_MAIN
  StrCmp $ParentProcessId "" +2 0
    nsExec::ExecToStack '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -Command "Wait-Process -Id $ParentProcessId -Timeout 30 -ErrorAction SilentlyContinue"'
  SetOutPath "$INSTDIR"
  File /oname=ssh-router.exe "${SOURCE_DIR}\ssh-router.exe"
  SetOutPath "$INSTDIR\resources"
  File /oname=ssh-router-cli.exe "${SOURCE_DIR}\resources\ssh-router-cli.exe"

  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortcut "$SMPROGRAMS\SSH Router.lnk" "$INSTDIR\ssh-router.exe"

  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router" "MainBinaryName" "ssh-router.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router" "DisplayName" "SSH Router"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router" "DisplayIcon" '"$INSTDIR\ssh-router.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router" "Publisher" "sshrouter"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router" "NoRepair" 1

  StrCmp $RestartAfterInstall 1 0 +2
    Exec '"$INSTDIR\ssh-router.exe"'
SectionEnd

Section "Uninstall"
  ClearErrors
  Delete "$INSTDIR\ssh-router.exe"
  IfErrors 0 +3
    MessageBox MB_OK|MB_ICONSTOP "SSH Router is still running. Exit it from the tray and retry uninstalling." /SD IDOK
    Abort
  Delete "$SMPROGRAMS\SSH Router.lnk"
  Delete "$INSTDIR\resources\ssh-router-cli.exe"
  RMDir "$INSTDIR\resources"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSH Router"
SectionEnd
