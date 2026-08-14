@echo off
:: Release 烧录：不启用 usb-log（无 USB CDC 日志/HFCLK），适合量产与低功耗运行。
:: 日志走 defmt-rtt（需 SWD + probe-rs 调试；正常运行无 USB 串口）。

set "BUILD_CMD=cargo build --release --no-default-features --features no-battery,led-active-high,defmt-rtt"
set "FLASH_BANNER=nRF52840 biu_switch Release Flash (no usb-log)"

call "%~dp0flash.bat" %*
