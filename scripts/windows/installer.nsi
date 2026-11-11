; The chess-master installer for Windows, built with NSIS by scripts/package-windows.sh.
; It installs for the current user only, so Windows doesn't ask for admin rights.
; The script passes three values: VERSION, SOURCE (the folder with the files) and OUTFILE.
; makensis runs in the project folder (-NOCD), so paths here are relative to it.

Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"

!define APP "chess-master"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}"

Name "${APP} ${VERSION}"
OutFile "${OUTFILE}"
RequestExecutionLevel user
InstallDir "$LOCALAPPDATA\Programs\${APP}"
; An update goes into the same folder as the version before it.
InstallDirRegKey HKCU "${UNINSTALL_KEY}" "InstallLocation"
SetCompressor /SOLID lzma

; The pages, in order.
!define MUI_ICON "assets/icon.ico"
!define MUI_UNICON "assets/icon.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\chess-master.exe"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "chess-master and Stockfish" SectionApp
  SectionIn RO ; always installed
  SetOutPath "$INSTDIR"
  File "${SOURCE}/chess-master.exe"
  File "${SOURCE}/stockfish.exe"
  File "${SOURCE}/README.txt"
  SetOutPath "$INSTDIR\licenses"
  File "${SOURCE}/licenses/*.txt"
  SetOutPath "$INSTDIR" ; the shortcut starts the app in this folder

  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortcut "$SMPROGRAMS\${APP}.lnk" "$INSTDIR\chess-master.exe"

  ; This makes chess-master show up in Settings > Apps, with an Uninstall button.
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\chess-master.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "EstimatedSize" $0
SectionEnd

; /o: shown on the components page, but not ticked.
Section /o "Desktop shortcut" SectionDesktop
  CreateShortcut "$DESKTOP\${APP}.lnk" "$INSTDIR\chess-master.exe"
SectionEnd

; Removes exactly what was installed. Your settings in %APPDATA%\chess-master stay.
Section "Uninstall"
  Delete "$INSTDIR\chess-master.exe"
  Delete "$INSTDIR\stockfish.exe"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\licenses\*.txt"
  RMDir "$INSTDIR\licenses"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR" ; only if it's empty now
  Delete "$SMPROGRAMS\${APP}.lnk"
  Delete "$DESKTOP\${APP}.lnk"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
SectionEnd
