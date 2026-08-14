@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

:: ============================================
:: nRF52840 biu_switch Flash Script
:: Nice!Nano V2 / ProMicro nRF52840 (UF2)
:: ============================================

set "SCRIPT_DIR=%~dp0"
set "PROJECT_DIR=%SCRIPT_DIR%"
set BIN_NAME=biu_switch
set TARGET=thumbv7em-none-eabi
set TARGET_DIR=target\%TARGET%\release
if not defined BUILD_CMD set BUILD_CMD=cargo build --release --no-default-features --features no-battery,usb-log,led-active-high
set UF2_FAMILY=0xADA52840
set UF2CONV=%SCRIPT_DIR%tools\uf2conv.py
set UF2FAMILIES=%SCRIPT_DIR%tools\uf2families.json

echo.
echo ============================================
if defined FLASH_BANNER (
    echo   !FLASH_BANNER!
) else (
    echo   nRF52840 biu_switch Flash Tool
)
echo ============================================
if defined BUILD_CMD echo [BUILD] !BUILD_CMD!
echo.

if not exist "%PROJECT_DIR%Cargo.toml" (
    echo [ERROR] Cargo.toml not found in: %PROJECT_DIR%
    echo Please keep this script next to the project Cargo.toml.
    exit /b 1
)

call :ensure_rust_target
if !errorlevel! neq 0 exit /b 1

call :ensure_tools
if !errorlevel! neq 0 exit /b 1

echo.
echo  [1] Build + UF2 Flash
echo  [2] Build Only
echo  [3] Build + Flash via probe-rs (SWD)
echo  [0] Exit
echo.
set /p choice="Enter option (0-3): "

if "%choice%"=="1" goto build_and_flash
if "%choice%"=="2" goto build_only
if "%choice%"=="3" goto probe_flash
if "%choice%"=="0" goto exit_script

echo [ERROR] Invalid option
exit /b 1

:build_and_flash
call :build_firmware "biu_switch.uf2"
if %errorlevel% neq 0 exit /b 1
call :flash_firmware "biu_switch.uf2"
exit /b %errorlevel%

:build_only
call :build_firmware "biu_switch.uf2"
exit /b %errorlevel%

:probe_flash
call :build_firmware ""
if %errorlevel% neq 0 exit /b 1
call :flash_probe_rs
exit /b %errorlevel%

:: ============================================
:: Build firmware and optionally generate UF2
:: Arg1: uf2 filename (empty = skip uf2)
:: ============================================
:build_firmware
set UF2_FILE=%~1

echo.
echo ----------------------------------------
echo [BUILD] biu_switch firmware...
echo ----------------------------------------

pushd "%PROJECT_DIR%"
%BUILD_CMD%
set BUILD_RESULT=!errorlevel!
popd

if !BUILD_RESULT! neq 0 (
    echo [ERROR] build failed!
    exit /b 1
)

if "%UF2_FILE%"=="" (
    echo [SUCCESS] ELF build complete
    exit /b 0
)

call :generate_uf2 "%UF2_FILE%"
exit /b %errorlevel%

:: ============================================
:: ELF -^> HEX -^> UF2
:: ============================================
:generate_uf2
set UF2_FILE=%~1
set ELF=%PROJECT_DIR%%TARGET_DIR%\%BIN_NAME%
set HEX=%PROJECT_DIR%%TARGET_DIR%\%BIN_NAME%.hex

if not exist "%ELF%" (
    echo [ERROR] ELF not found: %ELF%
    exit /b 1
)

echo [GENERATE] %UF2_FILE% ...
"%OBJCOPY%" -O ihex "%ELF%" "%HEX%"
if %errorlevel% neq 0 (
    echo [ERROR] objcopy failed
    exit /b 1
)

"%PYTHON%" %PYTHON_ARGS% "%UF2CONV%" "%HEX%" -c -f %UF2_FAMILY% -o "%UF2_FILE%"
if %errorlevel% neq 0 (
    echo [ERROR] UF2 conversion failed
    exit /b 1
)

echo [SUCCESS] %UF2_FILE% generated
exit /b 0

:: ============================================
:: Copy UF2 to Nice!Nano / Adafruit bootloader drive
:: ============================================
:flash_firmware
set UF2_FILE=%~1

if not exist "%UF2_FILE%" (
    echo [ERROR] UF2 file not found: %UF2_FILE%
    exit /b 1
)

echo.
echo ----------------------------------------
echo [FLASH] %UF2_FILE%
echo ----------------------------------------
echo.
echo Enter bootloader mode:
echo   1. Connect USB
echo   2. Within 0.5 s, short RST to GND twice
echo   3. A USB drive should appear (Nice!Nano / UF2 Bootloader)
echo   4. Press any key when ready...
echo.
pause >nul

set BOOT_DRIVE=
for %%d in (D: E: F: G: H: I: J: K: L: M: N: O: P: Q: R: S: T: U: V: W: X: Y: Z:) do (
    if exist "%%d\INFO_UF2.TXT" (
        findstr /I /C:"Raspberry Pi" "%%d\INFO_UF2.TXT" >nul 2>&1
        if !errorlevel! neq 0 (
            set BOOT_DRIVE=%%d\
            goto :found_boot_drive
        )
    )
)

echo [ERROR] UF2 bootloader drive not found!
echo [HINT] Double-tap RST, or try: python %UF2CONV% %UF2_FILE% -d
exit /b 1

:found_boot_drive
echo [INFO] Bootloader drive: %BOOT_DRIVE%
if exist "%BOOT_DRIVE%INFO_UF2.TXT" (
    echo [INFO] Bootloader INFO_UF2.TXT:
    type "%BOOT_DRIVE%INFO_UF2.TXT"
    echo.
)
echo [FLASH] Copying %UF2_FILE% ...

copy /Y "%UF2_FILE%" "%BOOT_DRIVE%" >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Copy failed. Close other programs using the drive and retry.
    exit /b 1
)

echo [SUCCESS] Flash complete! Board will reset automatically.
echo.
timeout /t 2 /nobreak >nul
exit /b 0

:: ============================================
:: probe-rs SWD flash (optional)
:: ============================================
:flash_probe_rs
where probe-rs >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] Installing probe-rs...
    cargo install probe-rs-tools --locked
    if %errorlevel% neq 0 (
        echo [ERROR] Failed to install probe-rs
        exit /b 1
    )
)

echo [FLASH] probe-rs run ...
probe-rs run --chip nRF52840_xxAA "%PROJECT_DIR%%TARGET_DIR%\%BIN_NAME%"
exit /b %errorlevel%

:: ============================================
:: Ensure rust target installed
:: ============================================
:ensure_rust_target
rustup target list --installed | findstr /C:"%TARGET%" >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] Installing Rust target %TARGET% ...
    rustup target add %TARGET%
    if %errorlevel% neq 0 (
        echo [ERROR] rustup target add failed
        exit /b 1
    )
)
exit /b 0

:: ============================================
:: Ensure objcopy, python, uf2conv.py
:: ============================================
:ensure_tools
set OBJCOPY=
set PYTHON=
set PYTHON_ARGS=

for /f "delims=" %%R in ('rustc --print sysroot 2^>nul') do (
    set "LLVM_OBJCOPY=%%R\lib\rustlib\x86_64-pc-windows-msvc\bin\llvm-objcopy.exe"
)
if exist "!LLVM_OBJCOPY!" set "OBJCOPY=!LLVM_OBJCOPY!"

if not defined OBJCOPY (
    where arm-none-eabi-objcopy >nul 2>&1
    if not errorlevel 1 set OBJCOPY=arm-none-eabi-objcopy
)
if not defined OBJCOPY (
    where llvm-objcopy >nul 2>&1
    if not errorlevel 1 set OBJCOPY=llvm-objcopy
)
if not defined OBJCOPY (
    rustup component list --installed 2>nul | findstr /C:"llvm-tools" >nul 2>&1
    if errorlevel 1 (
        echo [INFO] Installing llvm-tools-preview ...
        rustup component add llvm-tools-preview
    )
    for /f "delims=" %%R in ('rustc --print sysroot 2^>nul') do (
        set "LLVM_OBJCOPY=%%R\lib\rustlib\x86_64-pc-windows-msvc\bin\llvm-objcopy.exe"
    )
    if exist "!LLVM_OBJCOPY!" set "OBJCOPY=!LLVM_OBJCOPY!"
)
if not defined OBJCOPY (
    echo [ERROR] objcopy not found.
    echo [HINT] Run: rustup component add llvm-tools-preview
    echo [HINT] Or install GNU Arm Embedded Toolchain: arm-none-eabi-objcopy
    exit /b 1
)

where python >nul 2>&1
if not errorlevel 1 set PYTHON=python
if not defined PYTHON (
    where py >nul 2>&1
    if not errorlevel 1 (
        set PYTHON=py
        set PYTHON_ARGS=-3
    )
)
if not defined PYTHON (
    echo [ERROR] Python 3 not found - needed for uf2conv.py
    exit /b 1
)

if not exist "%SCRIPT_DIR%tools" mkdir "%SCRIPT_DIR%tools"
if not exist "%UF2CONV%" (
    echo [INFO] Downloading uf2conv.py ...
    curl.exe -fsSL -o "%UF2CONV%" "https://raw.githubusercontent.com/microsoft/uf2/master/utils/uf2conv.py"
    if errorlevel 1 (
        echo [ERROR] Failed to download uf2conv.py
        echo Manual: clone https://github.com/microsoft/uf2 and set UF2CONV path
        exit /b 1
    )
)
if not exist "%UF2FAMILIES%" (
    echo [INFO] Downloading uf2families.json ...
    curl.exe -fsSL -o "%UF2FAMILIES%" "https://raw.githubusercontent.com/microsoft/uf2/master/utils/uf2families.json"
    if errorlevel 1 (
        echo [ERROR] Failed to download uf2families.json
        exit /b 1
    )
)

echo [INFO] objcopy: !OBJCOPY!
echo [INFO] python:  !PYTHON! !PYTHON_ARGS!
echo [INFO] uf2conv: %UF2CONV%
exit /b 0

:exit_script
echo.
exit /b 0
