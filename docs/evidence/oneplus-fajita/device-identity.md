# Device Identity Specification: OnePlus 6T (`fajita`)

## Hardware Profile
- **Marketing Name**: OnePlus 6T
- **Codename**: `fajita`
- **SoC**: Qualcomm Snapdragon 845 (SDM845)
- **CPU**: Octa-core Kryo 385 (4x 2.8 GHz Gold + 4x 1.7 GHz Silver)
- **GPU**: Qualcomm Adreno 630
- **RAM**: 6 GB / 8 GB LPDDR4X
- **Internal Storage**: 128 GB / 256 GB UFS 2.1
- **Display**: 6.41" Optic AMOLED, 2340x1080 (19.5:9), 60Hz
- **Touch Digitizer**: Synaptics Touch Controller (I2C)
- **Modem / Telephony**: Qualcomm Snapdragon X20 LTE (4G LTE, VoLTE, dual nano-SIM)
- **Audio**: Qualcomm WCD9341 audio codec
- **PMIC**: Qualcomm PM845 / PM8998 power management IC
- **Battery**: 3700 mAh Li-Po, Dash / Fast Charging (5V 4A)
- **Connectors**: USB-C 2.0 (OTG supported)

## Bootloader & Recovery Specifications
- **Unlock Status**: Factory bootloader unlockable via `fastboot oem unlock` without OEM token on standard non-carrier models.
- **Partition Layout**: A/B dual-slot seamless update scheme (`boot_a`, `boot_b`, `system_a`, `system_b`, `vendor_a`, `vendor_b`, `userdata`).
- **Emergency Recovery**: Qualcomm EDL (Emergency Download Mode, 9008) accessible via volume buttons / EDL cable with MSM Download Tool.
