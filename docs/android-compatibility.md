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

### 4.1 Graphics: what is built, and what is not

The compositor side of this bridge now exists. `nilui-gpu` contains a Wayland
server written directly against the wire protocol, with a reusable client API
(`nilui_gpu::wayland::client::SurfaceProducer`) for anything that has pixels to
show. See `docs/drm-kms-compositor.md` §0.1 for the full implemented/absent
breakdown.

**What works today, verified by tests:** a producer process connects to the
compositor, creates a `wl_surface`, allocates a `wl_shm` pool, hands over CPU
pixels, and sees them composited into the DRM present path. This is exercised
across two real processes over a real socket by
`runtime/nilui-gpu/tests/wayland_cross_process.rs`.

**What does not exist yet, and must not be assumed:**

- **No SurfaceFlinger integration.** Nothing in this repository talks to an
  Android SurfaceFlinger, and no AOSP rootfs is bundled. The bridge is a working
  *Wayland* side waiting for a guest that speaks it.
- **No `linux-dmabuf` import.** A gralloc buffer handed over by SurfaceFlinger
  is a GPU buffer. The compositor only accepts `wl_shm` CPU memory, so a bridge
  would have to copy every frame into a shared pool. That copy is a real
  per-frame cost on a phone and is the main reason this is not yet a complete
  graphics path.
- **No `xdg-shell`.** An Android bridge would define its own surface-placement
  convention rather than rely on a desktop shell protocol, so this is a smaller
  gap than it sounds, but it means stock toolkits will not run as-is.
- **No input path into Android.** Only `wl_touch` events can be encoded; nothing
  reads `services/inputd` and forwards it to a guest yet.

So the honest position is: the compositor can accept and display a surface from
any process that speaks the implemented Wayland subset. Turning a SurfaceFlinger
buffer into that surface still requires work that has not been written.

### 4.2 Planned design

- **Graphics**: a guest-side bridge would take buffers from SurfaceFlinger, copy
  them into a `wl_shm` pool (or, later, export them as dmabuf once
  `linux-dmabuf` is implemented), and push them through `SurfaceProducer`.
- **Input**: Touch and keyboard events translated from `services/inputd` into
  Wayland seat events. Not implemented; only the `wl_touch` event encoders exist.
- **Audio**: Container audio streams routed to the host PipeWire daemon via UNIX
  domain sockets. Not implemented.

---

## 5. GMS Alternatives (microG)

Onuron OS does not ship proprietary Google Mobile Services (GMS):
- Optional integration with **microG** provides open-source re-implementations of Google Play Services (push notifications via UnifiedPush, location providers, account authentication).
- Open app distribution through **F-Droid** and curated independent repositories.
