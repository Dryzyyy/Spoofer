@echo off
setlocal
REM Lance GhostNet-GUI en admin (requis : MAC + TUN ; sans admin = proxy seul)
set "APP=%~dp0GhostNet-GUI.exe"
if not exist "%APP%" (
  echo [ERREUR] Introuvable : %APP%
  echo.
  echo Le fichier a probablement ete mis en quarantaine par Windows Defender
  echo ^(faux positif frequent sur les exe non signes qui touchent au reseau/registre^).
  echo.
  echo Solution : ajoute une exclusion Defender sur le dossier D:\Spoofer
  echo   1. Securite Windows -^> Protection contre les virus et menaces
  echo   2. Gerer les parametres -^> Exclusions -^> Ajouter : D:\Spoofer
  echo   3. Restaure le fichier depuis Historique de protection, ou rebuild :
  echo      cd D:\Spoofer\gui ^&^& vsenv.cmd npx tauri build
  echo.
  pause
  exit /b 1
)
powershell -NoProfile -Command "Start-Process '%~dp0GhostNet-GUI.exe' -WorkingDirectory '%~dp0' -Verb RunAs"
