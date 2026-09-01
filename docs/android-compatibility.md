# 🤖 NilOS Android Application Compatibility Layer

Rather than maintaining a full reimplementation of the Android runtime, NilOS executes Android applications through a hardened, containerized isolation layer inspired by LXC and Waydroid.

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
│                NilOS Host Bridge Layer                 │
│         nilandroidd · binder-shim · nilshell           │
└───────────────────────────┬────────────────────────────┘
                            │ Host IPC
                            ▼
┌────────────────────────────────────────────────────────┐
│                NilOS Native Userspace                  │
│       nilinit · nild · NilUI Shell · Linux Kernel      │
└────────────────────────────────────────────────────────┘
```

---

## 2. Container Isolation & Security

Android applications run inside a dedicated, unprivileged Linux user namespace:
- **Mount Namespace**: The Android container receives an isolated rootfs (`/android/rootfs`). It cannot inspect or modify NilOS system partitions.
- **IPC Isolation**: Direct Binder access is mediated through `binder-shim`. Host services are invisible to the container.
- **Zero Host Telemetry**: Android applications have no access to host hardware identifiers, preventing cross-system tracking.

---

## 3. Surface & Audio Bridging

- **Graphics**: The containerized SurfaceFlinger passes rendered graphical buffers directly to `nilshell` using the standard Wayland protocol extension (`wl_surface`). Android applications appear as native windows on the NilOS desktop and launcher.
- **Input**: Touch and keyboard events are translated from `nilshell` to Android input events via the Wayland input seat.
- **Audio**: Container audio streams are routed to the host PipeWire daemon via UNIX domain sockets.

---

## 4. GMS Alternatives (microG)

NilOS does not ship proprietary Google Mobile Services (GMS):
- Optional integration with **microG** provides open-source re-implementations of Google Play Services (push notifications via UnifiedPush, location providers, account authentication).
- Open app distribution through **F-Droid** and curated independent repositories.
