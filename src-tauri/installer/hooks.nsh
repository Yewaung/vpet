; VPet installer hooks (Tauri NSIS).
; The pet's mind is a language model (Qwen3, Apache-2.0) and the pet can't
; live without one. It's too big to ship inside the installer (and every
; update would re-download it), so it is downloaded while installing, checked
; against its published SHA-256, and only then put in place. Installing is the
; user's permission for this one download; the app itself stays offline.
;
; The model lives in app data ($APPDATA\<bundle id>\models), the same place
; the app downloads one to, never in the install folder. Upgrading by hand
; runs the old version's uninstaller first, and that must not throw away a
; 2.5 GB download: the model is removed with the pet's other data, only
; when the user ticks "Delete the application data" while uninstalling.

!macro NSIS_HOOK_POSTINSTALL
  SetDetailsView show
  IfFileExists "$APPDATA\${BUNDLEID}\models\Qwen3-4B-Q4_K_M.gguf" vpet_model_done
  IfFileExists "$APPDATA\${BUNDLEID}\models\Qwen3-0.6B-Q8_0.gguf" vpet_model_done
  ; An in-app update doesn't ask: the app itself offers the download if the mind is missing.
  StrCmp $UpdateMode 1 vpet_model_done
  ; The light model is the default (and what silent installs get): it runs on almost any PC.
  StrCpy $1 "light"
  MessageBox MB_YESNO|MB_ICONQUESTION "Your pet feels, thinks and talks with a language model that runs on this PC. Which one should it have?$\r$\n$\r$\nYes: FULL mind (2.5 GB download, needs 8 GB RAM). Richest thoughts and talk.$\r$\nNo: LIGHT mind (0.6 GB download). Quick, runs on almost any PC.$\r$\n$\r$\nYou can switch later in Status > Privacy > Its mind." /SD IDNO IDNO vpet_model_go
    StrCpy $1 "full"
  vpet_model_go:
  DetailPrint "Downloading resources: your pet's $1 mind. This can take a while..."
  nsExec::ExecToLog '"$INSTDIR\${MAINBINARYNAME}.exe" --install-model "$APPDATA\${BUNDLEID}\models" $1'
  Pop $0
  StrCmp $0 "0" vpet_model_done
    DetailPrint "Your pet's mind was not installed ($0)."
    ; 3: no room, 4: the file didn't match what was published, else the download failed.
    StrCpy $2 "The download didn't finish. Check the internet connection."
    StrCmp $0 "3" 0 +2
      StrCpy $2 "There isn't enough free disk space. Free up some space (the light mind needs about 0.9 GB, the full one about 2.7 GB)."
    StrCmp $0 "4" 0 +2
      StrCpy $2 "The download was damaged (it didn't match the published file)."
    MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Your pet's mind could not be installed.$\r$\n$\r$\n$2$\r$\n$\r$\nRetry now, or Cancel: VPet will ask to download it when it starts, and your pet sleeps until then." /SD IDCANCEL IDRETRY vpet_model_go
  vpet_model_done:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Unfinished downloads are never worth keeping. Finished models stay with
  ; the pet's data, which Tauri's uninstaller deletes (models included) only
  ; when "Delete the application data" is ticked.
  Delete "$APPDATA\${BUNDLEID}\models\*.download"
  Delete "$APPDATA\${BUNDLEID}\models\*.part"
!macroend
