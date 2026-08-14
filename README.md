# biu_switch — nRF52840 + JDY-68A 蓝牙音箱固件

基于 **ProMicro nRF52840**（兼容 **Nice!Nano V2**）的蓝牙音箱控制固件：通过 **JDY-68A** 接收手机蓝牙音频，经 **PAM8406** 功放驱动喇叭；支持自动关机、按钮唤醒、播放/连接状态检测。调试日志可选 `usb-log`（烧录脚本默认启用）。

详细硬件与时序见 [`蓝牙音箱固件设计文档.md`](蓝牙音箱固件设计文档.md)。

## 硬件接线（摘要）

| 功能 | 引脚 | 说明 |
|------|------|------|
| JDY VCC | 开发板 **VCC**（P0.13 内部控制） | 必须接 VCC，不能接常开 3.3V |
| 功放电源 | **P0.08** (D0) → 330Ω → XYF-1H6 IN+ | 高=功放上电；RAW → 继电器 → PAM8406 |
| UART RX | **P0.06** (D1) ← JDY TXD | 9600 8N1 |
| UART TX | **P0.02** (D19) → JDY RXD | 关机前拉低，防倒灌 |
| STAT | **P0.17** (D2) ← JDY STAT | 高=已连接 |
| 唤醒按钮 | **P0.09** (D10) → 按钮 → GND | 内部上拉；需 `nfc-pins-as-gpio` |
| 板载 LED | **P0.15** | ProMicro 红灯用 `led-active-high` |
| 电池 ADC | **P0.04** | Nice!Nano / 兼容板分压 |

## 行为说明

| 需求 | 实现 |
|------|------|
| 开机时序 | 按钮唤醒冷启动 → JDY 上电 → 等 600ms → 功放上电（防爆音） |
| 蓝牙名 | 开机查询 `AT+NAMA`；若不是 **BiuSpeaker** 则自动改名 |
| 关机时序 | TX 拉低 → 功放断电 → JDY 断电 → **SYSTEM OFF** |
| 自动关机 | 蓝牙断开或暂停后 **10 分钟**无活动 → 关机；连接/播放中取消倒计时 |
| 唤醒 | 按 P0.09 → GPIO SENSE 唤醒 → 冷启动 |
| 板载 LED | 呼吸灯（后续可改为状态指示） |
| 烧录 | 双击 RST 进 UF2；或 USB 串口发送 `BOOT` |

> 测极低功耗请 **拔掉 USB**，仅用电池供电。

## Feature

| Feature | 说明 |
|---------|------|
| `usb-log` | USB CDC **明文 ASCII** 日志（SSCOM 等串口助手可直接看；与 `defmt-rtt` 互斥） |
| `defmt-rtt` | defmt 经 SWD RTT 输出（默认 feature） |
| `led-active-high` | ProMicro 红灯高电平点亮 |
| `no-led` | 关闭板载 LED（调试） |
| `no-battery` | 跳过 SAADC（调试） |

## 编译

```powershell
rustup target add thumbv7em-none-eabi
cargo build --release --no-default-features --features usb-log,led-active-high
```

产物：`target/thumbv7em-none-eabi/release/biu_switch`

## 烧录

```powershell
.\flash.bat
```

量产/无 USB 日志：

```powershell
.\flash-release.bat
```

有 SWD 时可用脚本菜单 **[3]**，或：

```powershell
probe-rs run --chip nRF52840_xxAA target/thumbv7em-none-eabi/release/biu_switch
```

## 调试（USB 明文日志）

烧录 `flash.bat`（默认 `usb-log`）后，用任意串口助手打开 CDC 口即可，例如：

```
usb cdc ready
biu_switch boot
power on: complete
STAT: connected
BT connected
UART: play
music playing
UART: pause
music paused
STAT: disconnected
BT disconnected
```

发送 `BOOT` 并回车可进 UF2 Bootloader。

> 旧版 defmt 二进制日志在 SSCOM 里会显示为乱码；现已改为明文 ASCII。

## 模块结构

```
src/
├── main.rs           # 初始化与电源命令循环
├── power.rs          # P0.13/P0.08 时序 + SYSTEM OFF
├── jdy68a.rs         # UART 事件解析 + STAT 监控
├── audio_state.rs    # 连接/播放状态机
├── auto_shutdown.rs  # 10 分钟自动关机
├── button.rs         # 运行中按键（取消倒计时）
├── events.rs         # 事件通道
├── board_led.rs      # 板载 LED（保留）
├── battery.rs        # 电池采样（保留）
├── usb_log.rs        # USB CDC defmt（保留）
└── bootloader.rs     # UF2 软入口（保留）
```
