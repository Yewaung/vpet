; VPet installer hooks (Tauri NSIS).
; The language model (Qwen3-4B, 2.5 GB, Apache-2.0) is too big to ship inside
; the installer, so it is downloaded while installing. Installing is the
; user's permission for this one download; the app itself stays offline.

!macro NSIS_HOOK_POSTINSTALL
  SetDetailsView show
  IfFileExists "$INSTDIR\models\Qwen3-4B-Q4_K_M.gguf" vpet_model_done
  IfFileExists "$INSTDIR\models\Qwen3-0.6B-Q8_0.gguf" vpet_model_done
  ; The light model is the default (and what silent installs get).
  StrCpy $1 "light"
  MessageBox MB_YESNO|MB_ICONQUESTION "Which language model should your pet think with?$\r$\n$\r$\nYes: FULL model (2.5 GB download). Best conversation.$\r$\nNo: LIGHT model (0.6 GB download). Quick and small; simpler talk.$\r$\n$\r$\nYou can switch later in Status > Privacy." /SD IDNO IDNO vpet_model_go
    StrCpy $1 "full"
  vpet_model_go:
  DetailPrint "Downloading resources: the $1 language model. This can take a while..."
  nsExec::ExecToLog '"$INSTDIR\${MAINBINARYNAME}.exe" --install-model "$INSTDIR\models" $1'
  Pop $0
  StrCmp $0 "0" vpet_model_done
    DetailPrint "The language model was not downloaded ($0)."
    MessageBox MB_ICONEXCLAMATION|MB_OK "The language model could not be downloaded.$\r$\n$\r$\nVPet works without it: your pet still lives, learns and uses the words it knows. You can download it later in Status > Privacy." /SD IDOK
  vpet_model_done:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$INSTDIR\models\Qwen3-4B-Q4_K_M.gguf"
  Delete "$INSTDIR\models\Qwen3-4B-Q4_K_M.part"
  Delete "$INSTDIR\models\Qwen3-0.6B-Q8_0.gguf"
  Delete "$INSTDIR\models\Qwen3-0.6B-Q8_0.part"
  RMDir "$INSTDIR\models"
!macroend
