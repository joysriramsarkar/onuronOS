# 🤖 Onuron OS Android Application Compatibility Layer

Rather than maintaining a full reimplementation of the Android runtime, Onuron OS executes Android applications through a hardened, containerized isolation layer inspired by LXC and Waydroid.

Official architectural ecosystem:
> **"Onuron OS — powered by NilLang + Alap"**

---

## 1. Architecture Overview

```
┌────────────────────────────────────────────────────────┐
│                   Android Container                    │
│      Android Apps (APK) · Android Framework (AOSP)     │
│             Headless SurfaceFlinger & Audio            │
└───────────────────────────┬────────────────────────────┘
                            │ Wayland / PipeWire / Binder Socket
                            ▼
┌────────────────────────────────────────────────────────┐
│               Onuron OS Host Bridge Layer              │
│         nilandroidd · binder-shim · nilshell           │
└───────────────────────────┬────────────────────────────┘
                            │ Host IPC
                            ▼
┌────────────────────────────────────────────────────────┐
│               Onuron OS Native Userspace               │
│       nilinit · nild · NilUI Shell · Linux Kernel      │
└────────────────────────────────────────────────────────┘
```

---

## 2. Phased Integration Stages

To maintain stability, Android compatibility is approached in 4 disciplined stages (Phase 5):
1. **Stage 1**: Headless container bootstrap & shell environment.
2. **Stage 2**: Package management (APK installation inside isolated container storage).
3. **Stage 3**: Framework & binder translation (`binder-shim` to Onuron OS HAL).
4. **Stage 4**: Graphics, input passthrough (`wl_surface` to `nilshell`), audio, and camera.

---

## 3. Container Isolation & Security

Android applications run inside a dedicated, unprivileged Linux user namespace:
- **Mount Namespace**: The Android container receives an isolated rootfs (`/android/rootfs`). It cannot inspect or modify Onuron OS system partitions.
- **IPC Isolation**: Direct Binder access is mediated through `binder-shim`. Host services are invisible to the container.
- **Zero Host Telemetry**: Android applications have no access to host hardware identifiers, preventing cross-system tracking.

---

## 4. Surface & Audio Bridging

- **Graphics**: The containerized SurfaceFlinger passes rendered graphical buffers directly to `nilshell` using the standard Wayland protocol extension (`wl_surface`). Android applications appear as native windows on the Onuron OS desktop and launcher.
- **Input**: Touch and keyboard events are translated from `services/inputd` to Android input events via the Wayland input seat.
- **Audio**: Container audio streams are routed to the host PipeWire daemon via UNIX domain sockets.

---

## 5. GMS Alternatives (microG)

Onuron OS does not ship proprietary Google Mobile Services (GMS):
- Optional integration with **microG** provides open-source re-implementations of Google Play Services (push notifications via UnifiedPush, location providers, account authentication).
- Open app distribution through **F-Droid** and curated independent repositories.
