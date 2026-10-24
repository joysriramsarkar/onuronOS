# ADR-0010 — Shell design system and the 1.7 screen implementation

- **Status:** accepted
- **Date:** 2026-10-22
- **Owning layer:** `nilshell` / `NilUI` (shell chrome and native screen composition).
  The implementation under `nilsim/` is the desktop rendering of that shell, not a
  new architecture layer.
- **Supersedes:** nothing.

## Context

The project holds a complete visual reference for the phone shell:
`অনুরণ ওএস — অ্যাপ UI ডিজাইন-png/` — 35 screens, each in a dark and a light
variant, 780×1592 px. Until now the simulator drew ad-hoc screens with hard-coded
colours, per-screen geometry and icons assembled inside each renderer. That made
"does the OS look like the design?" unanswerable and made every new screen a copy
of the previous one's mistakes.

The maintainer asked for the OS UI to follow those screens and for the matching
functionality, with two exceptions: **Terminal** and **Memories** keep their
existing implementation.

## Decision

1. **Geometric source of truth is the reference art.** The logical canvas is
   390×796, exactly half of the 780×1592 art, so every measured pixel in the PNG
   maps to one logical pixel after ÷2. Measurements are recorded in
   `docs/ui-design-spec.md`.
2. **One token + component layer.** `nilsim/src/ui.rs` owns palette, metrics,
   the vector icon set and the shared widgets (status bar, screen title, card,
   row, switch, slider, chip, keypad, primary button, FAB). Screens compose
   widgets; they do not redefine colours or spacing.
3. **Palette is sampled, not invented** (`#1E1236` / `#322748` / `#9B6BFF` …).
   Both themes derive from the same token struct, so a screen cannot drift
   between themes.
4. **Terminal and Memories are out of scope.** They keep their previous
   renderers, status bar and navigation bar. `paint_screen` branches explicitly
   for them (`legacy_chrome`), so redesign work cannot silently restyle them.
5. **Camera and Music have no tile on the designed home grid.** The designed
   home has exactly 12 native tiles. To keep the art intact while still making
   those screens reachable, an app drawer (`AppDrawer`, all 14 native apps) is
   reachable by scrolling up on Home, `F6`, or the shell shortcut list.
6. **Validation path.** Two headless modes exist so the result is checkable
   without a human clicking:
   - `nilsim --export-ui <dir>` renders every shell screen in both themes to BMP
     for comparison against the reference PNGs.
   - `nilsim --selftest` exercises routing, toggles, slider drag, the PIN gate,
     note persistence, backup, update check and preference round-tripping, then
     exits non-zero on failure.

## Consequences

- Adding a screen now means composing `ui::` widgets and registering hit areas;
  restyling means editing tokens once.
- The simulator's shell is verifiable against the art and by script; regressions
  in routing or persistence are caught by `--selftest`.
- Hardware-backed controls (Wi-Fi, Bluetooth, torch, VPN, camera sensor) remain
  **simulated state** in the desktop simulator. The screens are honest about
  what they do — capture writes a placeholder file and says so, update check
  reads a real channel manifest if present and reports when it is absent.
- The reference art is design input, not runtime data: no PNG is shipped or
  sampled at runtime, and the icon set is vector geometry in code.
- The lock PIN is persisted as a salted SHA-256 record in `/etc/prefs.json`
  (salt + digest, never the plaintext). A device without a PIN record boots into
  the first-boot flow; changing the PIN from Security re-enters that flow.
- Hardware-backed controls are simulator state and are described as such; the
  desktop shell does not claim driver-level Wi-Fi, Bluetooth, VPN or camera
  behaviour.

## Amendments (2026-10-23)

Five shell defects reported after the first pass, and their resolutions:

1. **Browser.** The embedded WebView2 was shown for the whole `AppBrowser`
   screen and painted over the redesigned start page. It is now created/visible
   only for an opened page and bounded to the content area; closing a page hides
   it again.
2. **Scrolling.** The settings list is now scrollable (11 rows, wheel + hairline
   scrollbar); Files, Notes, nilpkg, Android and SoftBus lists scroll too.
   Rows are clipped to their card.
3. **Notification status bar.** The pre-1.7 behaviour is restored: the left part
   of the status bar is the notification-panel handle.
4. **Gesture bar.** The back/home/recents capsule is drawn on every interactive
   screen, except lock/OOBE (security) and the two legacy apps.
5. **Media-only volume.** Slider drags captured only a thin strip near the
   track; the three sound cards now capture the pointer across their full
   height, so ringtone, media and alarm all respond.

The toast timer was also fixed: messages expired by wall-clock but never
decremented, so the last one stuck to every screen.
