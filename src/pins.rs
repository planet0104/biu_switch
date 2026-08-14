//! ProMicro nRF52840 / Nice!Nano V2 引脚分配（蓝牙音箱）。
//!
//! 板载固定引脚：
//! - P0.15 板载 LED（Nice!Nano 低电平点亮；ProMicro 红灯需 `led-active-high`）
//! - P0.04 电池电压 ADC（分压）
//! - P0.13 外部 VCC 截止（高=通，低=关；供 JDY-68A）
//!
//! 外接：
//! - P0.08 (D0) → XYF-1H6 IN+（经 330Ω），功放电源继电器
//! - P0.06 (D1) → JDY-68A TXD（nRF UART RX）
//! - P0.02 (D19) → JDY-68A RXD（nRF UART TX）
//! - P0.17 (D2) → JDY-68A STAT（高=已连接）
//! - P0.09 (D10) → 唤醒按钮（内部上拉，按下为低；SYSTEM OFF 唤醒；需 nfc-pins-as-gpio）

/// 唤醒按钮（P0.09）
pub const BUTTON_WAKE_PIN: u8 = 9;

/// UART TX → JDY RXD（P0.02）；关机前须拉低，防止倒灌
pub const JDY_UART_TX_PIN: u8 = 2;

/// 板载 LED（P0.15）
pub const BOARD_LED_PIN: u8 = 15;

/// 无活动自动关机（秒）
pub const AUTO_OFF_SEC: u64 = 600;

/// JDY-68A 上电后等待蓝牙初始化（毫秒）
pub const JDY_BOOT_DELAY_MS: u64 = 600;

/// 音频蓝牙广播名（手机「蓝牙音箱」列表里看到的名字，对应 AT+NAMA）
pub const AUDIO_BT_NAME: &str = "BiuSpeaker";

/// AT 查询/改名超时（毫秒）
pub const JDY_AT_TIMEOUT_MS: u64 = 800;

/// 轮询 AT+STAT / AT+PSTAT 间隔（毫秒）
pub const JDY_STAT_POLL_MS: u64 = 1500;

/// STAT 上电后等待 JDY 就绪再采样（毫秒），避免悬空被误判为已连接
pub const STAT_STARTUP_DELAY_MS: u64 = 1500;

/// STAT 消抖：连续稳定采样次数
pub const STAT_STABLE_SAMPLES: u8 = 5;

/// STAT 采样间隔（毫秒）
pub const STAT_SAMPLE_MS: u64 = 20;

/// 关机时功放断电后再切 JDY 的间隔（毫秒）
pub const AMP_OFF_SETTLE_MS: u64 = 50;

/// 按钮消抖（毫秒）
pub const BUTTON_DEBOUNCE_MS: u64 = 50;

/// 正常运行呼吸灯半周期：渐亮或渐灭各 3 s
pub const LED_BREATHE_HALF_MS: u64 = 3000;

/// 呼吸灯占空比更新间隔（毫秒）
pub const LED_BREATHE_STEP_MS: u64 = 10;

/// 呼吸灯 PWM 频率（Hz）
pub const LED_BREATHE_PWM_HZ: u32 = 20_000;

/// 低电量：每 N 秒快速闪一次
pub const LED_LOW_BAT_INTERVAL_SEC: u64 = 3;

/// 低电量单次快闪时长（毫秒）
pub const LED_LOW_BAT_FLASH_MS: u64 = 80;
