; Curio's Windows installer (R-DEL-9).
;
; NSIS rather than MSI because every write this installer makes is per-user — an HKCU
; registry value per browser, a Run key, a directory under %LOCALAPPDATA%. None of them
; wants the elevation an MSI's per-machine default implies, and asking for admin would be a
; lie about what the installer touches. D29 dropped MSIX and left NSIS-or-MSI open; this is
; that choice being made.
;
; **Registration is not reimplemented here.** `curio-nmh --register` already writes the
; native-messaging manifest and points four browsers at it (R-EXT-20), and it is a binary
; this installer has just placed on disk. Restating its registry keys in NSIS would give a
; single fact a second home (R-OV-2) and guarantee the two drift.

Unicode true
ManifestDPIAware true

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"

; FileFunc's helpers are macros that must be declared before `${GetSize}` resolves; the
; !include alone only makes them available to declare.
!insertmacro GetSize

!define APP_NAME  "Curio"
!define APP_EXE   "curio.exe"
!define NMH_EXE   "curio-nmh.exe"
!define PUBLISHER "Rob Bags"
!define APP_URL   "https://github.com/rescueahero4/Curio"

; Stamped by CI from the one workspace version (R-DEL-12). The fallbacks exist so a bare
; `makensis curio.nsi` still produces something locally instead of dying on an undefined
; symbol.
!ifndef APP_VERSION
  !define APP_VERSION "0.0.0"
!endif
; SRC_DIR MUST be absolute. NSIS resolves every relative path against the *script's*
; directory rather than the working directory, so a caller who passes "stage" from the repo
; root gets packaging\windows\stage and a "file not found" that names a path nobody typed.
!ifndef SRC_DIR
  !define SRC_DIR "."
!endif

; Bare filename on purpose. NSIS resolves an icon path relative to the **script's** directory,
; not the working directory, so a caller-supplied "packaging\windows\curio.ico" resolves to
; packaging\windows\packaging\windows\curio.ico and fails to open. The icon sits beside this
; file; naming it alone is the only form that works from any CWD.
!ifndef ICON_FILE
  !define ICON_FILE "curio.ico"
!endif
; Matches the name release CI passes, so a local `makensis` run and a release produce the
; same file. That name is version-less on purpose (D36): the landing page links
; /releases/latest/download/curio-windows-x64-setup.exe, which only resolves while the asset
; filename stays byte-identical across releases.
!ifndef OUT_FILE
  !define OUT_FILE "curio-windows-x64-setup.exe"
!endif

!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
!define RUN_KEY    "Software\Microsoft\Windows\CurrentVersion\Run"

Name "${APP_NAME} ${APP_VERSION}"
OutFile "${OUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\${APP_NAME}"
InstallDirRegKey HKCU "Software\${APP_NAME}" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey "ProductName"     "${APP_NAME}"
VIAddVersionKey "CompanyName"     "${PUBLISHER}"
VIAddVersionKey "LegalCopyright"  "MIT"
VIAddVersionKey "FileDescription" "${APP_NAME} installer"
VIAddVersionKey "FileVersion"     "${APP_VERSION}"
VIAddVersionKey "ProductVersion"  "${APP_VERSION}"

!define MUI_ICON   "${ICON_FILE}"
!define MUI_UNICON "${ICON_FILE}"

!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Start ${APP_NAME}"

!insertmacro MUI_PAGE_LICENSE "${SRC_DIR}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; ---------------------------------------------------------------------------------------
; Installing over a running copy leaves a locked exe and a half-written install. Both paths
; check first and say so plainly rather than failing on a file-in-use error the user cannot
; interpret.
;
; The test is whether **this install's** executable can be opened for writing — not whether
; something named curio.exe exists anywhere. A developer running the app from `cargo run`
; has a curio.exe in the process list that has nothing to do with $INSTDIR, and blocking on
; that would refuse an install for a file the installer was never going to touch. Opening
; the target for append tests the actual condition: can this file be replaced.
; ---------------------------------------------------------------------------------------

!macro EnsureNotRunning UN
Function ${UN}EnsureNotRunning
  retry:
    ; Nothing there yet — a fresh install has nothing to collide with.
    ${IfNot} ${FileExists} "$INSTDIR\${APP_EXE}"
      Return
    ${EndIf}

    ClearErrors
    FileOpen $0 "$INSTDIR\${APP_EXE}" a
    ${IfNot} ${Errors}
      FileClose $0
      Return
    ${EndIf}

    ; A silent run has nobody to answer a dialog. Blocking on one would hang an unattended
    ; install forever, which is worse than failing loudly with a code the caller can act on.
    ${If} ${Silent}
      SetErrorLevel 2
      Abort "${APP_NAME} is running and its executable is locked. Quit it and re-run."
    ${EndIf}

    MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION \
      "${APP_NAME} is running.$\n$\nQuit it from the tray icon (right-click ${APP_NAME} \
then Quit) and press Retry. Quitting from the tray runs the full shutdown sequence; ending \
the process instead leaves a stale runtime.json behind." \
      IDRETRY retry
    Abort
FunctionEnd
!macroend

!insertmacro EnsureNotRunning ""
!insertmacro EnsureNotRunning "un."

Section "Install"
  Call EnsureNotRunning

  SetOutPath "$INSTDIR"
  SetOverwrite on

  File "${SRC_DIR}\${APP_EXE}"
  File "${SRC_DIR}\${NMH_EXE}"
  File "/oname=LICENSE.txt" "${SRC_DIR}\LICENSE"

  ; Tell the browsers the host exists. A machine with none of the four Chromium browsers
  ; installed is not an install failure — the extension falls back to the probe ladder
  ; (R-EXT-8) — so the exit code is reported, never fatal.
  DetailPrint "Registering the native-messaging host..."
  nsExec::ExecToLog '"$INSTDIR\${NMH_EXE}" --register'
  Pop $0
  ${If} $0 != 0
    DetailPrint "No browser was registered (exit $0). Pairing falls back to the in-app code."
  ${EndIf}

  WriteRegStr HKCU "Software\${APP_NAME}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\${APP_NAME}" "Version"    "${APP_VERSION}"

  ; Add/Remove Programs.
  WriteRegStr   HKCU "${UNINST_KEY}" "DisplayName"     "${APP_NAME}"
  WriteRegStr   HKCU "${UNINST_KEY}" "DisplayVersion"  "${APP_VERSION}"
  WriteRegStr   HKCU "${UNINST_KEY}" "Publisher"       "${PUBLISHER}"
  WriteRegStr   HKCU "${UNINST_KEY}" "URLInfoAbout"    "${APP_URL}"
  WriteRegStr   HKCU "${UNINST_KEY}" "DisplayIcon"     "$INSTDIR\${APP_EXE}"
  WriteRegStr   HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr   HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1

  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" "$0"

  CreateShortcut "$SMPROGRAMS\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"

  WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

; ---------------------------------------------------------------------------------------
; Uninstall is a feature (R-DEL-11).
;
; It removes the app, the NM manifests and their registry keys, the autostart entry, and
; runtime.json. It does NOT touch the data root — the database, screenshots, sidecars, and
; prompts under %USERPROFILE%\Curio stay exactly where they are. Deleting someone's library
; is their explicit act, never a side effect of removing an application.
; ---------------------------------------------------------------------------------------

Section "Uninstall"
  Call un.EnsureNotRunning

  DetailPrint "Unregistering the native-messaging host..."
  nsExec::ExecToLog '"$INSTDIR\${NMH_EXE}" --unregister'
  Pop $0

  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\${NMH_EXE}"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\com.curio.nmh.json"
  Delete "$INSTDIR\uninstall.exe"
  RMDir  "$INSTDIR"

  Delete "$SMPROGRAMS\${APP_NAME}.lnk"

  ; runtime.json and the quit-token lock. Their absence is how everything else knows the app
  ; is not running, so leaving them behind after an uninstall would be actively misleading.
  ;
  ; !! The RMDir below MUST stay non-recursive. !!
  ;
  ; This same directory holds `secrets.dpapi` — the DPAPI-encrypted API key (curio-server's
  ; secrets.rs resolves its vault to app_data_dir()/secrets.dpapi). Plain `RMDir` removes the
  ; directory only when it is empty, so the vault survives and the directory simply stays;
  ; that behaviour is verified, not assumed. Adding `/r` here would look like tidying up and
  ; would silently destroy the user's credential on every uninstall — the exact class of
  ; side-effect R-DEL-11 exists to forbid. If the leftover directory ever needs cleaning,
  ; delete the two files by name and leave the rest alone.
  Delete "$LOCALAPPDATA\${APP_NAME}\runtime.json"
  Delete "$LOCALAPPDATA\${APP_NAME}\curio.lock"
  RMDir  "$LOCALAPPDATA\${APP_NAME}"

  DeleteRegValue HKCU "${RUN_KEY}" "${APP_NAME}"
  DeleteRegKey   HKCU "Software\${APP_NAME}"
  DeleteRegKey   HKCU "${UNINST_KEY}"
SectionEnd
