# ADR-0011 — Shell visual language 2.0 (“নীল-শাঁখা”)

- **Status:** accepted
- **Date:** 2026-11-14
- **Owning layer:** `nilshell` / NilUI — the shell chrome and native screen
  composition. `nilsim` remains the desktop rendering of that shell, not a new
  architecture layer.
- **Supersedes visually:** the palette and component styling of ADR-0010. The
  geometry contract (390×796, ÷2 of the 780×1592 art), the token/component
  layer, the routing and the legacy exceptions all stay in force.

## Context

ADR-0010 turned the reference art into one token + component layer, which made
“does the shell look like the design?” answerable. After using it, the visual
language itself became the limit:

- ground, card and leading tile were three steps of the same violet hue, so every
  surface carried the same weight and nothing receded;
- the accent was used for any active element, including a full-width primary bar
  at the bottom of a dense notification screen;
- the home screen gave eight Android letter tiles the same visual weight as the
  twelve native apps, which contradicts the project's own rule that Android
  compatibility is a separate subsystem;
- several screens had real defects rather than taste problems: the calculator
  keypad ran under the gesture bar, the camera previewed an invented purple
  scene, the battery settings row used a `+` icon, and the recents thumbnail was
  an empty rectangle.

## Decision

1. **Ink ground, ivory text, brass for state.** `bg #0B0F22` / `card #151B33` /
   `tile #1E2542` with warm ivory `#F4EEE2` in dark; paper `#F6F1E6` with ink
   `#101528` and an ink icon tone in light. Brass `#D9A441` (light `#8A5B12`) is
   reserved for *on / selected / primary*, never decoration. Every text pair is
   ≥ 4.5 : 1; the measurements live in `docs/design/ui-2.0.md` §2.
2. **Hairlines, not fills.** Cards separate from the ground by a 1px border at low
   contrast; radius rises to 18 (cards), 14 (tiles), 19 (app tiles). No shadows.
3. **One destructive style.** `ghost_button` — card fill, hairline, danger label.
   A destructive action never gets the solid accent bar.
4. **Android is a shelf, not a grid.** Home's eight coloured letter tiles are
   replaced by one “সামঞ্জস্য” card that opens the Android hub. Android screens
   keep their own implementation and are not coupled to native APIs (rule §3.9).
5. **Honesty in the pixels.** The camera viewfinder says “ক্যামেরা · সিমুলেটেড”
   instead of drawing a fake scene; recents show an app glyph where no live
   thumbnail exists; the home system strip reads real state (battery, storage,
   package count) and nothing else.
6. **Real defects are fixed in the same pass** (calculator overflow, battery row
   icon, scrollbar weight, terminal glyph overlap), because a visual system that
   leaves broken screens behind is not a system.

## Consequences

- Restyling is still one edit in `nilsim/src/ui.rs`; screens compose widgets.
- `docs/ui-design-spec.md` remains the measured geometry record for the reference
  art; its palette table is historical and points here.
- Terminal and Memories are still out of scope by the earlier maintainer
  instruction; they keep the old light renderer even in dark mode. That
  inconsistency is known and recorded, not hidden.
- No hardware claim changes: nothing here was validated on a panel, and the
  simulated controls are labelled as simulated.

## Validation path

- `cargo build -p nilsim` — clean.
- `nilsim --export-ui <dir>` — 36 screens × 2 themes render.
- `nilsim --selftest` — 19/19 (routing, toggles, sliders, PIN gate, notes,
  backup, update check, recents bounds, Android shortcut, prefs).
- `docs/design/onuron-ui-2.0-before-after.png` and the two gallery sheets are the
  visual evidence; they are generated from the exported BMPs, not hand-drawn.
