//! Adafruit UF2 Bootloader 入口（Nice!Nano / ProMicro nRF52840）。
//!
//! 通过 `GPREGRET` 魔术字在下次复位时进入 UF2 盘符 + CDC 下载模式。

use embassy_nrf::pac;

/// Adafruit nRF52 Bootloader：`GPREGRET = 0x57` → UF2 + CDC
const UF2_GPREGRET: u8 = 0x57;

/// 写入引导标志并软件复位，不返回。
pub fn enter_uf2_bootloader() -> ! {
    defmt::info!("entering UF2 bootloader");
    defmt::flush();
    pac::POWER.gpregret().write(|w| w.set_gpregret(UF2_GPREGRET));
    cortex_m::peripheral::SCB::sys_reset();
}
