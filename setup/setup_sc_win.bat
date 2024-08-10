@echo off

set "EXEC_PATH="

SET file_not_found=false
if not exist "%~1" if not exist ".\librespeed-rs.exe" SET file_not_found=true
IF "%file_not_found%"=="true" (
  echo file not found
  exit /b 1
) else (
  if not "%~1"=="" (
    for %%F in ("%~1") do set "EXEC_PATH=%%~fF"
  )
  if exist ".\librespeed-rs.exe" (
    for %%F in (".\librespeed-rs.exe") do set "EXEC_PATH=%%~fF"
  )
)

REM init values
SET SERVICE_NAME=librespeed-rs
SET DISPLAY_NAME=Librespeed Rust Backend

REM Create the service
sc create %SERVICE_NAME% binPath= "%EXEC_PATH%" start= auto DisplayName= "%DISPLAY_NAME%"

REM Configure the service to restart automatically on failure
sc failure %SERVICE_NAME% reset= 0 actions= restart/5000

echo Service %SERVICE_NAME% created successfully.
