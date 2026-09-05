# 🎨 Onuron OS UI System: NilUI, Vulkan Renderer & Wayland Compositor

Onuron OS provides a fluid, hardware-accelerated user experience targeting 120Hz refresh rates with spring-physics motion and modern declarative styling.

Official architectural ecosystem:
> **"Onuron OS — powered by NilLang + Alap"**

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

## 3. Display Pipeline: DRM/KMS First, Vulkan Acceleration Second

To guarantee display output even when proprietary GPU drivers are unavailable:
1. **DRM/KMS Dumb Buffers**: Direct Linux `/dev/dri/card0` dumb-buffer framebuffer allocation and page-flipping.
2. **GPU Acceleration**: Vulkan 2D pipeline (`nilui-gpu`) rendering surfaces with SDF primitives, HarfBuzz text shaping, and 120Hz presentation timing.

---

## 4. `nilshell`: Wayland Compositor

The desktop and mobile surface manager is built on `wlroots`:
- **Touch Gesture Recognition**: 1-finger edge swipe (Back/Home navigation), 2-finger pinch (multitasking overview), 3-finger swipe (app switching).
- **Surface Routing**: Isolates client buffers and composites system overlays (status bar, navigation bar, lock screen, notifications).
- **Convergence Engine**: When connected to an external display (USB-C DisplayPort Alternate Mode), `nilshell` transitions seamlessly from single-app mobile layout to floating window desktop mode.
