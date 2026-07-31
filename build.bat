@echo off
setlocal
where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust/Cargo was not found in PATH.
  echo Install Rust from https://rustup.rs and run this file again.
  exit /b 1
)

cargo build --release
if errorlevel 1 exit /b 1

copy /Y target\release\OpenCalc.exe OpenCalc.exe >nul

if exist hlp-viewer.exe (
  copy /Y hlp-viewer.exe target\release\hlp-viewer.exe >nul
)

for %%F in (CALC.HLP CALC.CNT CALP.HLP CALP.CNT CALS.HLP CALS.CNT) do (
  if exist target\release\%%F del /q target\release\%%F
)

if exist target\release\Help rmdir /S /Q target\release\Help
mkdir target\release\Help
if exist Help\CALC_EN.HLP copy /Y Help\CALC_EN.HLP target\release\Help\CALC_EN.HLP >nul
if exist Help\CALC_EN.CNT copy /Y Help\CALC_EN.CNT target\release\Help\CALC_EN.CNT >nul
if exist Help\CALC_PT-BR.HLP copy /Y Help\CALC_PT-BR.HLP target\release\Help\CALC_PT-BR.HLP >nul
if exist Help\CALC_PT-BR.CNT copy /Y Help\CALC_PT-BR.CNT target\release\Help\CALC_PT-BR.CNT >nul
if exist Help\CALC_ES.HLP copy /Y Help\CALC_ES.HLP target\release\Help\CALC_ES.HLP >nul
if exist Help\CALC_ES.CNT copy /Y Help\CALC_ES.CNT target\release\Help\CALC_ES.CNT >nul
if exist calc.tooltip copy /Y calc.tooltip target\release\calc.tooltip >nul

echo Built OpenCalc.exe with the embedded calc95.ico resource
if exist hlp-viewer.exe echo Included companion hlp-viewer.exe
if exist Help\CALC_EN.HLP echo Included English Help\CALC_EN.HLP
if exist Help\CALC_EN.CNT echo Included English Help\CALC_EN.CNT
if exist Help\CALC_PT-BR.HLP echo Included Portuguese Help\CALC_PT-BR.HLP
if exist Help\CALC_PT-BR.CNT echo Included Portuguese Help\CALC_PT-BR.CNT
if exist Help\CALC_ES.HLP echo Included Spanish Help\CALC_ES.HLP
if exist Help\CALC_ES.CNT echo Included Spanish Help\CALC_ES.CNT
if exist calc.tooltip echo Included calc.tooltip context-help catalog
if not exist Help\CALC_EN.HLP echo Note: English Help\CALC_EN.HLP is missing.
if not exist Help\CALC_PT-BR.HLP echo Note: Portuguese Help\CALC_PT-BR.HLP is missing; Portuguese Help will be unavailable.
if not exist Help\CALC_ES.HLP echo Note: Spanish Help\CALC_ES.HLP is missing; Spanish Help will be unavailable.
if not exist Help\CALC_EN.CNT echo Note: Help\CALC_EN.CNT is optional, but required for the English Help contents hierarchy.
if not exist Help\CALC_PT-BR.CNT echo Note: Help\CALC_PT-BR.CNT is optional, but required for the Portuguese Help contents hierarchy.
if not exist Help\CALC_ES.CNT echo Note: Help\CALC_ES.CNT is optional, but required for the Spanish Help contents hierarchy.
