@echo off
rem Skello WinUSB driver installer.
rem Binds the WinUSB driver to the bootloader ports the app opens
rem (MediaTek BROM / preloader / DA, Qualcomm EDL 9008, Unisoc BROM,
rem Google fastboot and adb) through pnputil. Windows 10 and newer.

setlocal
cd /d "%~dp0"

if not exist skello_winusb.inf (
    echo skello_winusb.inf not found next to this script.
    pause
    exit /b 1
)

rem pnputil requires an elevated shell; check and ask for elevation.
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo Requesting administrator rights...
    powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b %errorlevel%
)

echo Installing the Skello WinUSB driver package...
pnputil /add-driver "%~dp0skello_winusb.inf" /install

rem Note: the COM-port drivers (usbser for the MediaTek CDC ports, the
rem Qualcomm QDLoader for 9008) stop claiming these devices once this
rem package binds WinUSB. Skello never uses COM ports; other serial
rem tools do - roll back through Device Manager if one must keep working.

echo.
echo Done. Plug the phone into bootloader mode; Skello sees the ports.
pause
endlocal
