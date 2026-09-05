# 🖥️ Onuron OS: DRM/KMS Compositor Architecture

This document specifies the display architecture and transition plan for `nilshell`, evolving from an ANSI console prototype into a direct Linux Direct Rendering Manager / Kernel Mode Setting (DRM/KMS) display server and Wayland compositor.

---

## 1. Architectural Evolution

```
[ Prototype (Current) ]
Application ──> ANSI Escape Sequences ──> /dev/console (Terminal)

[ Phase 3 Target (DRM/KMS Dumb Buffers) ]
Application ──> Pixel Buffer ──> /dev/dri/card0 (DRM Dumb Buffer) ──> KMS CRTC ──> Hardware Display

[ Phase 4 Target (Accelerated Wayland Compositor) ]
NilLang Native App (.nil)
       │
       ▼
   NilUI Toolkit
       │
       ▼
  Wayland Protocol IPC (wl_surface, wl_shm, linux-dmabuf)
       │
       ▼
   nilshell Compositor
       ├── Software Blitter (Fallback)
       └── Vulkan 2D Renderer (nilui-gpu)
       │
       ▼
  DRM/KMS Modesetting & Page Flipping (Double / Triple Buffering)
       │
       ▼
Hardware Screen (PinePhone / QEMU VirtIO-GPU / Real Mobile Panel)
```

---

## 2. Direct DRM/KMS Initialization Pipeline

To bring up real displays without depending on a heavy desktop environment or X11, `nilshell` communicates directly with the Linux DRM subsystem:

1. **Card Discovery**:
   Open primary DRM device node:
   ```c
   int drm_fd = open("/dev/dri/card0", O_RDWR | O_CLOEXEC);
   ```

2. **Resource Enumeration**:
   Query DRM resources (`drmModeGetResources(drm_fd)`):
   - Iterate through connectors (`drmModeConnector`).
   - Find the connected connector (`DRM_MODE_CONNECTED`).
   - Select the preferred display mode (resolution & refresh rate, e.g. 720x1440 @ 60Hz).
   - Find the matching CRTC and encoder.

3. **Dumb Buffer Creation**:
   Allocate raw pixel memory in video/system RAM via ioctl:
   ```c
   struct drm_mode_create_dumb creq = {
       .width = mode.hdisplay,
       .height = mode.vdisplay,
       .bpp = 32, // XRGB8888
   };
   ioctl(drm_fd, DRM_IOCTL_MODE_CREATE_DUMB, &creq);
   ```

4. **Framebuffer Binding**:
   Register dumb buffer with the KMS subsystem:
   ```c
   uint32_t fb_id;
   drmModeAddFB(drm_fd, creq.width, creq.height, 24, 32, creq.pitch, creq.handle, &fb_id);
   ```

5. **Memory Mapping**:
   Map the framebuffer memory to userspace address space:
   ```c
   struct drm_mode_map_dumb mreq = { .handle = creq.handle };
   ioctl(drm_fd, DRM_IOCTL_MODE_MAP_DUMB, &mreq);
   void *pixels = mmap(0, creq.size, PROT_READ | PROT_WRITE, MAP_SHARED, drm_fd, mreq.offset);
   ```

6. **Modesetting & Page Flipping**:
   Set the CRTC mode with `drmModeSetCrtc`. Maintain two buffers (Front & Back) and flip asynchronously using `drmModePageFlip` synced to VBLANK interrupts for tear-free 60Hz/120Hz rendering.

---

## 3. Why DRM/KMS First, Vulkan Later?

Starting with DRM/KMS dumb buffers provides:
1. **Guaranteed Early Display**: Boot splash, lockscreen, and shell can render even before proprietary or complex GPU drivers (Mali, Adreno, PowerVR) are initialized.
2. **Deterministic Debugging**: Raw memory buffer writing allows simple pixel inspection and software test pattern output.
3. **Graceful Degradation**: If Vulkan hardware acceleration crashes or lacks driver support on a specific ARM64 device, the system automatically falls back to software-rendered DRM/KMS dumb buffers.
