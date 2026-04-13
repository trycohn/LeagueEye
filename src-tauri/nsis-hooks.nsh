; Kill running LeagueEye process before installing files
!macro NSIS_HOOK_PREINSTALL
  ; nsExec runs the command silently (no visible console window)
  nsExec::Exec 'taskkill /f /im "league-eye.exe"'
  ; Give Windows time to release the file lock after process termination
  Sleep 2000
!macroend
