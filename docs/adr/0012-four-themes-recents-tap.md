# ADR-0012 — Four themes in Display settings, a legible notification shade, and a recents rail that actually scrolls

- **Status:** accepted
- **Date:** 2026-11-15
- **Owning layer:** `nilshell` / NilUI — the shell chrome and native screen
  composition. `nilsim` remains the desktop rendering of that shell, not a new
  architecture layer.
- **Builds on:** ADR-0011 (visual language 2.0 "নীল-শাঁখা"). No geometry contract,
  token model or routing changes.

## Context

Three defects reported against the 2.0 build:

1. **Display settings offered only three theme choices** (লাইট / ডার্ক / অটো)
   and the cards were bare labels — no preview of what was being chosen.
2. **Notification shade glyphs were hard to read**: the Wi-Fi arcs fused into a
   solid fan at tile size, the aeroplane was a stroked outline that vanished, the
   torch and bell-off shapes did not survive 24px, and every app icon in a
   notification row rendered in one monochrome tone, so sources were
   indistinguishable at a glance.
3. **Recents did not scroll.** The input path updated `recents_scroll_offset`
   (wheel, keys, drag) and the rail used it — but a card's `rec:*` action fired
   on mouse-**down**, so any drag that started on a card opened the app instead
   of scrolling. On a touch surface the rail was effectively unscrollable.

## Decision

1. **Four themes, one of them AMOLED.** লাইট (paper) / ডার্ক (ink) / AMOLED
   (true-black ground, surfaces lifted by hairline only, same ivory/brass pair)
   / অটো (clock-resolved light↔dark, 06:00–18:00). The three resolved palettes
   are `Palette::Light|Dark|Amoled` in `nilsim/src/ui.rs`; `auto` is *not* a
   palette — it resolves at apply time in `SimState::set_theme_from_mode()`.
2. **Prefs migration, once.** Persisted key `theme_mode` renumbered to
   0/1/2/3 = light/dark/amoled/auto, written as `theme_scheme_v2`. On load, a
   file with `theme_scheme_v2` wins; a legacy file only gets its `theme_mode`
   mapped with `2` (old auto) → `3`, so an upgrading user keeps their auto
   behaviour instead of silently becoming AMOLED. Saving also writes a
   backward-compatible `theme_mode` (2→1 dark) so old builds boot to something sane.
3. **Theme picker with live previews.** Four 82×148 cards, each painting a
   miniature of its own palette (ground, raised card, brass bar, two text
   lines; অটো shows paper and ink side by side). Active = brass hairline and a
   bold label; no badge, since brass already encodes "selected".
4. **Notification shade glyphs redrawn on the 24px grid** — Wi-Fi with thinner
   separated arcs, a *filled* aeroplane silhouette, a torch with three rays,
   a bell-with-slash re-proportioned for 24px, cellular as four filled rounded
   bars, a two-stroke leaf, arc-plus-arrowhead rotation. Notification rows get
   **per-source tinted tiles** (chat `#3ECF8E`, phone `#4AA3FF`, system
   `#B79BFF`, fallback `#8FD3FF`; tile = theme tile blended 22% toward the
   tint, glyph = tint on dark, tint darkened 38% on paper) so sources are scannable without
   adding a new surface hue. A muted `N টি` count now sits on the notification
   section rule.
5. **Recents: tap opens, drag scrolls, dots show position.** `rec:*` presses
   are deferred to mouse-**release**; a release within 30px of the press opens
   the card, anything larger is a rail drag (±1 card per 25px) and opens
   nothing. The rail keeps its horizontal geometry (docs/ui-design-spec.md),
   gains a title ("সাম্প্রতিক") with a live count, and always draws position
   dots above the clear button (active page = brass pill).

## Consequences

- One extra palette means *all* themed rendering paths now branch three ways,
  not two: `t()`, `get_theme()`, the exporter (which writes 36 × 3 = 108 BMP)
  and the idle AMOLED state. There is no fourth "auto" screenshot because auto
  resolves to light or dark by clock.
- Old builds reading a new prefs file see AMOLED as dark (safest fallback).
  New builds reading an old file see auto mapped correctly — migration is
  one-directional and automatic on first load.
- The notification shade is more colourful than the rest of the 2.0 system by
  design; brass is *still* reserved for on/selected/primary — no tint uses it.
- Dragging on a card no longer opens it; opening requires a stationary tap or a
  wheel/keys. This is the standard recents-overview behaviour.

## Validation path

- `cargo build -p nilsim` — clean, no new warnings.
- `nilsim --selftest` — **21/21** (new: AMOLED resolves to the Amoled palette,
  auto resolves by clock and is never mistaken for AMOLED).
- `nilsim --export-ui <dir>` — 108 BMP; the display, notification-panel and
  recents screens were inspected in all three palettes.
- Interactive drag-scroll of the recents rail cannot be exercised headless;
  the tap/drag threshold and deferral are covered by inspection of the mouse
  up/down paths, and the rail's bounded scroll range remains selftest-covered.
- Not validated on any physical panel: the AMOLED tokens are derived from the
  measured ink palette (UI 2.0 §2), not measured on AMOLED glass. No hardware
  claim is made.

## Honesty notes

- AMOLED here means a true-black ground in the simulator, not a verified
  power/contrast measurement on an OLED panel.
- The `theme_scheme_v2` key is written alongside the legacy `theme_mode`; both
  are simulator prefs, not a platform ABI.
