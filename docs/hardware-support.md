# 🔌 NilOS Hardware Support & HAL Architecture

To support a wide range of modern mobile hardware without requiring vendor-specific rewrites, NilOS utilizes a layered driver strategy combining upstream Linux LTS drivers, Android Treble modularity, and Halium/libhybris bridging.

---

## 1. Hardware Abstraction Topology

```
┌────────────────────────────────────────────────────────────┐
│                    NilOS Userspace Daemons                 │
│         (audiod, camerad, nild, netd, nilshell)            │
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
│ DRM/KMS · PipeWire · iwd   │ │ Bionic → Glibc / Musl Shim  │
└──────────────┬─────────────┘ └──────────────┬──────────────┘
               │                              │
               ▼                              ▼
┌────────────────────────────────────────────────────────────┐
│                 Linux LTS Kernel + Android GKI             │
└────────────────────────────────────────────────────────────┘
```

---

## 2. Kernel Baseline & Treble Modularity

1. **Linux LTS Kernel (5.15 / 6.1 / 6.6)**:
   - NilOS maintains a minimal defconfig base (`kernel/nilos_defconfig`).
   - Android Generic Kernel Image (GKI) support ensures core kernel updates do not break vendor-specific loadable kernel modules (LKM).
2. **Device Tree Source (DTS)**:
   - Hardware topology is discovered purely via Device Tree or ACPI.

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
display.set_refresh_rate(120);
```

---

## 4. Proprietary Vendor Driver Bridging (Halium & libhybris)

When porting NilOS to existing Android devices where vendor blobs are compiled exclusively against Android Bionic C runtime:
- **libhybris**: Provides dynamic linker translation between host GNU/Musl libc and Android Bionic libraries (`/vendor/lib64/*`).
- Allows NilOS to run Qualcomm Adreno, ARM Mali, or MediaTek proprietary GPU drivers, camera HALs, and audio DSP blobs without requiring vendor source code.
