# 🖥️ Onuron OS: DRM/KMS Compositor Architecture

This document specifies the display architecture for `nilshell`, evolving from an
ANSI console prototype into a direct Linux Direct Rendering Manager / Kernel Mode
Setting (DRM/KMS) display server and Wayland compositor.

---

## 0. Implementation status

What is implemented today, in `runtime/nilui-gpu`:

| Step | State |
| --- | --- |
| Card discovery over `/dev/dri/card*` | implemented |
| `GETRESOURCES` / `GETCONNECTOR` / `GETENCODER` / `GETCRTC` discovery | implemented |
| Preferred-mode selection (caller hint → driver preferred → largest) | implemented |
| `CREATE_DUMB` + `MAP_DUMB` + `ADDFB2` + `mmap` scanout buffers | implemented |
| `SETCRTC` modesetting | implemented |
| Page flip that blocks on `DRM_EVENT_FLIP_COMPLETE` | implemented |
| Software 2D rasterizer (rects, rounded rects, blit, bitmap text) | implemented |
| Wayland protocol IPC (`wl_surface`, `wl_shm`) | **implemented** (see §0.1) |
| `linux-dmabuf` GPU buffer import | **planned, not written** |
| Vulkan 2D renderer (`vkctx.rs`) | **stub — logs and returns** |

Three limits are worth stating plainly, because the code cannot be trusted past
them:

- **No physical panel has run this.** There is no DRM node in the development
  container and loading `vkms` is blocked there, so the ioctl sequence is
  verified against the kernel's own headers and request numbers but has never
  lit a real display. Treat the hardware path as unproven until a device boots.
- **A machine with no display is not presented to.** `DrmDevice` falls back to
  an offscreen backend that composes pixels in memory and reports
  `page_flip_success = false`; it never claims a frame reached a panel.
- **The compositor is CPU-only.** Client pixels are read back from shared memory
  and blended in software. There is no `linux-dmabuf` import, so a client that
  holds a GPU buffer must copy it to a `wl_shm` pool first. See §0.1.

## 0.1 Wayland server: what exists and what does not

The Wayland server is written directly against the wire protocol rather than on
top of `libwayland`. That is the same choice `drm.rs` makes for its ioctls, for
the same reasons: no `libwayland-server` headers are present, `deny.toml` gates
unmaintained crates as a release gate, and the musl release build statically
links.

**Implemented and tested**

| Area | State |
| --- | --- |
| Message framing, string/array padding, fd arguments | implemented, byte-layout tests |
| `wl_display`, `wl_registry`, `wl_compositor` | implemented |
| `wl_surface` (`attach`, `damage`, `frame`, `commit`, buffer scale/transform, offset) | implemented |
| `wl_shm`, `wl_shm_pool`, `wl_buffer` (ARGB8888, XRGB8888) | implemented, real memfd pools |
| `wl_callback`, `wl_output`, `wl_seat`, `wl_touch` event encoders | implemented |
| SCM_RIGHTS descriptor passing | implemented, round-tripped over a real socketpair |
| `wl_display.error` before connection close | implemented |
| Software compositing into the DRM present path | implemented, 14 tests |
| Eventfd commit wakeup (no polling) | implemented |
| Runnable `wayland_compositor` and `wayland_surface_producer` binaries | implemented |
| Cross-process test driving both binaries | implemented, negative-control checked |

**Not implemented — do not assume these work**

- `linux-dmabuf`: no GPU buffer import. A GPU client must copy to `wl_shm`.
- `wl_region` clipping: regions are accepted and tracked, never applied.
- Damage-limited redraw: `wl_surface.damage` is validated and discarded; the
  whole surface is recomposited each commit.
- `wl_pointer` and `wl_keyboard`: objects are accepted so clients that request
  them do not fail, but no input events are sent. Only `wl_touch` has encoders.
- Subsurfaces, `xdg-shell`, `wl_data_device`, and everything else outside the
  core `wayland.xml` subset listed above.
- `wl_shm_pool.resize`: refused with a protocol error rather than half-honoured.

The honest summary: a client can create a surface, hand over CPU pixels through
shared memory, and see them composited. Anything a modern toolkit needs beyond
that — `xdg-shell` in particular — is absent, so a stock GTK/Qt application will
not run against this yet.

## 0.2 Running it

```sh
export XDG_RUNTIME_DIR=/run/onuron
wayland_compositor --width 1080 --height 2400
```

In another terminal, push a few frames at it:

```sh
wayland_surface_producer --width 256 --height 256 --frames 5
```

The compositor prints each commit it receives and how many surfaces it drew. If
no DRM card is present it says so and stays offscreen, exiting with status 2
after the run; it never claims a frame reached a panel.

The ioctl request numbers and the UAPI struct sizes/offsets are pinned in
`runtime/nilui-gpu/src/ioctl.rs` and `runtime/nilui-gpu/src/drm_uapi.rs` against
values compiled from `/usr/include/drm/*.h`, so a layout drift fails the build.

---

## 1. Architectural Evolution

```
[ Prototype (Current nilshell) ]
Application ──> ANSI Escape Sequences ──> /dev/console (Terminal)

[ Implemented (nilui-gpu) ]
Application ──> Pixel Buffer ──> /dev/dri/card0 (DRM Dumb Buffer) ──> KMS CRTC ──> Hardware Display

[ Implemented: software Wayland compositor (GPU acceleration not yet written) ]
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
