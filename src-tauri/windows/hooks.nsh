; Releases up to the egui era were shipped with an Inno Setup installer (per-machine, in
; Program Files). Remove it silently first so users do not end up with two installs and two
; "Apps & features" entries. Settings live in %APPDATA% and are not touched by the old uninstaller.
!macro NSIS_HOOK_PREINSTALL
  SetRegView 64
  ReadRegStr $0 HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Next Tablet Driver_is1" "UninstallString"
  StrCmp $0 "" legacy_done
  DetailPrint "Removing the previous (Inno Setup) installation..."
  ExecWait '$0 /VERYSILENT /SUPPRESSMSGBOXES /NORESTART'
legacy_done:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  MessageBox MB_YESNO "Do you want to delete all NextTabletDriver profiles and configuration files?" IDYES delete_config IDNO skip_config
delete_config:
  ; Remove the legacy/custom config folder used by the Rust backend
  RMDir /r "$APPDATA\NextTabletDriver"
  ; Remove the Tauri webview storage and Local AppData cache
  RMDir /r "$LOCALAPPDATA\com.nexttabletdriver.app"
skip_config:
!macroend
