# 🎨 NilOS UI System: NilUI, Vulkan Renderer & Wayland Compositor

NilOS provides a fluid, hardware-accelerated user experience targeting 120Hz refresh rates with spring-physics motion and modern declarative styling.

---

## 1. UI Architecture Overview

```
┌───────────────────────────────────────────────────────────┐
│                    Application Layer                      │
│        NilUI Rust Declarative Widgets & State Stores      │
└─────────────────────────────┬─────────────────────────────┘
                              │ Draws UI tree
                              ▼
┌───────────────────────────────────────────────────────────┐
│                 nilui-gpu Render Backend                  │
│       SDF Primitive Drawing · HarfBuzz Text Shaping       │
│           Vulkan 2D Pipeline · Triple Buffering           │
└─────────────────────────────┬─────────────────────────────┘
                              │ Wayland client protocol (wl_surface)
                              ▼
┌───────────────────────────────────────────────────────────┐
│                 nilshell Wayland Server                   │
│       wlroots Compositor · Gesture Engine · Convergence   │
└─────────────────────────────┬─────────────────────────────┘
                              │ DRM / KMS / Direct Scanout
                              ▼
┌───────────────────────────────────────────────────────────┐
│                      Display Panel                        │
│                 120Hz Variable Refresh Rate               │
└───────────────────────────────────────────────────────────┘
```

---

## 2. NilUI Declarative Framework

NilUI adopts a declarative, reactive paradigm implemented natively in Rust:

- **State Management**: Reactive signals and observables trigger minimal subtree re-renders.
- **Physics Spring Animations**: Fluid transitions modeled on damped harmonic oscillators (mass, stiffness, damping) rather than rigid duration curves.
- **Design Tokens**: Centralized typography, spacing, corner radii, and color palettes defined in `/etc/nilos/tokens.json`.

---

## 3. `nilui-gpu`: Vulkan 2D Engine

Traditional mobile renderers frequently suffer from texture thrashing and font rendering bottlenecks. `nilui-gpu` addresses this via:

1. **Signed Distance Field (SDF) Rendering**:
   - Rounded rectangles, shadows, and vector paths are evaluated analytically in fragment shaders using SDF equations.
   - Zero tessellation overhead for corner radii and smooth anti-aliased edges.
2. **Text Shaping with HarfBuzz**:
   - Complex text layout, ligatures, and bidirectional text (including full Bengali Unicode support) powered by HarfBuzz.
   - Dynamic GPU glyph cache atlas populated on-demand.
3. **120Hz Presentation Timing**:
   - Vulkan swapchain configured with `VK_PRESENT_MODE_MAILBOX_KHR` or `FIFO_RELAXED_KHR` for tear-free 120Hz output with triple buffering.

---

## 4. `nilshell`: Wayland Compositor

The desktop and mobile surface manager is built on `wlroots`:
- **Touch Gesture Recognition**: 1-finger edge swipe (Back/Home navigation), 2-finger pinch (multitasking overview), 3-finger swipe (app switching).
- **Surface Routing**: Isolates client buffers and composits system overlays (status bar, navigation bar, lock screen, notifications).
- **Convergence Engine**: When connected to an external display (USB-C DisplayPort Alternate Mode), `nilshell` transitions seamlessly from single-app mobile layout to floating window desktop mode.
