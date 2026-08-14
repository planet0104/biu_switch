//! 统一日志出口：RTT(defmt) 与 USB CDC 明文。

/// 打印一行 ASCII 日志（勿用中文，串口助手才能正常显示）。
pub fn line(msg: &str) {
    defmt::info!("{=str}", msg);
    #[cfg(feature = "usb-log")]
    crate::usb_log::write_line(msg);
}
