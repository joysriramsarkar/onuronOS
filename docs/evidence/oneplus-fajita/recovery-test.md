# OnePlus 6T (`fajita`) Recovery and Rollback Procedure

## Emergency Recovery Architecture
1. **Fastboot Mode**:
   - Keys: Hold `Volume Up` + `Volume Down` + `Power` from power-off state.
   - Command: `fastboot reboot bootloader`
   - Role: Used for safe partition flashing via `build/flash-device.sh fajita`.
2. **EDL Mode (Qualcomm Emergency Download Mode 9008)**:
   - Hardware fail-safe built into Qualcomm SoC silicon boot ROM.
   - Trigger: Hold `Volume Up` + `Volume Down` while inserting USB-C cable to PC.
   - Tooling: OnePlus 6T MSM Download Tool (OxygenOS stock factory flash).
   - Recovery Guarantee: Restores factory partition tables, bootloader, modem, and radio partitions even if `boot` or `system` is fully corrupted.
3. **A/B Slot Switch Rollback**:
   - In case of boot failure on `boot_a`, the bootloader automatically rolls back to `boot_b` after 3 consecutive failed boots.
   - Manual slot switch command:
     ```bash
     fastboot --set-active=b
     fastboot reboot
     ```
