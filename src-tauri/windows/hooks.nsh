; Hooks del instalador NSIS de VRes.
;
; Antes de instalar (o actualizar) y antes de desinstalar, se le pide a la
; instancia en ejecucion que se cierre restaurando la pantalla. `--shutdown`
; nunca abre la interfaz: si VRes no estaba corriendo, termina al instante.

!macro NSIS_HOOK_PREINSTALL
  IfFileExists "$INSTDIR\${MAINBINARYNAME}.exe" 0 +2
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --shutdown'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  IfFileExists "$INSTDIR\${MAINBINARYNAME}.exe" 0 +2
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --shutdown'
!macroend
