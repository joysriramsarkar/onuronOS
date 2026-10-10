# OnePlus 6T (`fajita`) Hardware Bring-Up Capability Matrix

| Subsystem | Baseline Community Driver | OnuronOS Target Backend | Current Maturity Level | Acceptance Gate |
| --- | --- | --- | --- | --- |
| **Boot & CPU** | Mainline Linux 6.x (`sdm845`) | ARM64 Linux LTS + `nilinit` | PLANNED | Kernel boot to serial / bootsplash |
| **UFS Storage** | Linux standard UFS host controller | Ext4 `/data` with fscrypt | PLANNED | Read/write persistence across reboots |
| **Display Panel** | DRM/KMS DSI panel driver | Direct DRM dumb buffer (`nilui-gpu`) | PLANNED | 2340x1080 60Hz framebuffer rendering |
| **Touch Screen** | Evdev touch controller | `nilui-gpu` touch event loop | PLANNED | Multi-touch gesture hit testing |
| **Battery & Power** | PMIC battery fuel gauge | `powerd` sysfs reader | PLANNED | Battery percentage & charging state |
| **Wi-Fi & BT** | Ath10k WLAN / Bluetooth hci_qca | `netd` / `btd` | PLANNED | WPA2 association and IP assignment |
| **Audio** | ALSA Qualcomm sound card | `audiod` ALSA backend | PLANNED | Speaker output & microphone capture |
| **Modem (LTE/SMS)** | ModemManager / QMI / oFono | `telephonyd` QMI AT layer | PLANNED | Voice call & SMS dispatch |
| **Camera** | libcamera / Qualcomm CAMSS | `camerad` V4L2 backend | PLANNED | Still photo capture |

> **Maturity Note**: In accordance with the project maturity rule (AGENTS.md Section 4), all hardware subsystems on physical `fajita` remain strictly `PLANNED` until reproducible hardware test logs are recorded on an unlocked physical unit.
