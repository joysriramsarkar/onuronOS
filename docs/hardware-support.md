# 🔌 Onuron OS Hardware Support & HAL Architecture

To support a wide range of modern mobile hardware without requiring vendor-specific rewrites, Onuron OS utilizes a layered driver strategy combining upstream Linux LTS drivers, Android Treble modularity, and Halium/libhybris bridging.

Official architectural ecosystem:
> **"Onuron OS — powered by NilLang + Alap"**

---

## 1. Hardware Abstraction Topology

```
┌────────────────────────────────────────────────────────────┐
│                  Onuron OS Userspace Daemons               │
│         (audiod, camerad, inputd, powerd, nilshell)        │
└─────────────────────────────┬──────────────────────────────┘
                               │ Safe Rust Wrapper
                               ▼
┌────────────────────────────────────────────────────────────┐
│                       nilhal (Rust)                        │
│            Safe dlopen loader and device probing           │
└─────────────────────────────┬──────────────────────────────┘
                               │ C-ABI Dynamic Linkage
                               ▼
┌────────────────────────────────────────────────────────────┐
│                   NilHAL C Interface (hal/)                │
│    libnilhal_display.so · libnilhal_sensor.so · ...        │
└──────────────┬──────────────────────────────┬──────────────┘
                │ Native Linux Driver          │ Proprietary Vendor Blob
                ▼                              ▼
┌────────────────────────────┐ ┌─────────────────────────────┐
│ Upstream Linux Subsystems  │ │ Halium / libhybris Bridge   │
│ DRM/KMS · PipeWire · evdev │ │ Bionic → Glibc / Musl Shim  │
└──────────────┬─────────────┘ └──────────────┬──────────────┘
                │                              │
                ▼                              ▼
┌────────────────────────────────────────────────────────────┐
│                 Linux LTS Kernel + Android GKI             │
└────────────────────────────────────────────────────────────┘
```

---

## 2. Kernel Baseline & Single Reference Target Strategy

1. **Linux LTS Kernel (6.6 LTS)**:
   - Onuron OS maintains minimal defconfig fragments (`kernel/base_defconfig`, `kernel/arm64_defconfig`).
   - Android Generic Kernel Image (GKI) support ensures core kernel updates do not break vendor-specific loadable kernel modules (LKM).

2. **Phase 3 Single Reference Hardware Target**:
   - Rather than fragmenting efforts across multiple devices, Onuron OS targets **PinePhone (Allwinner A64 / Mali-400 MP2)** and **ARM64 QEMU (`-M virt`)** as the primary development platforms.
   - Hardware interfaces rely on mainline Linux drivers:
     - Display: Direct DRM/KMS (`sun4i-drm`)
     - Touch: Goodix GT917-based `evdev` touch controller
     - Power: AXP803 PMIC power supply subsystem (`/sys/class/power_supply/axp20x-battery`)

---

## 3. The NilHAL C-ABI Interface

The core HAL specification resides in `hal/`:
- `hal_display.h`: Mode setting, vsync callbacks, buffer presentation.
- `hal_sensors.h`: Accelerometer, gyroscope, proximity, ambient light.
- `hal_audio.h`: Audio routing, output volume, sample rate conversion.
- `hal_camera.h`: Camera sensor exposure, focus, frame streaming.

### Safe Rust Loader (`nilhal`)
The `runtime/nilhal` crate wraps these C interfaces in memory-safe Rust lifetimes:
```rust
use nilhal::display::DisplayDevice;

let display = DisplayDevice::load("/vendor/lib/nilhal/libnilhal_display.so")
    .expect("Failed to load display HAL module");
display.set_refresh_rate(60);
```

---

## 4. Proprietary Vendor Driver Bridging (Halium & libhybris)

When porting Onuron OS to existing Android devices where vendor blobs are compiled exclusively against Android Bionic C runtime:
- **libhybris**: Provides dynamic linker translation between host GNU/Musl libc and Android Bionic libraries (`/vendor/lib64/*`).
- Allows Onuron OS to run Qualcomm Adreno, ARM Mali, or MediaTek proprietary GPU drivers, camera HALs, and audio DSP blobs without requiring vendor source code.
