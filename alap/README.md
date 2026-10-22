# onuronOS Alap (Rust) — Mirror Implementation

This is the OS-side (Rust) mirror of the canonical Alap implementation at
`nilLang/pkg/alap` (Go). It is **not** the source of truth for the Alap API.

Drift between this crate and the canonical package is tracked in
`onuronOS/docs/LANGUAGE_AND_RUNTIME_CONTRACT.md`. New API additions must land in
`nilLang/pkg/alap` first, then be ported here.
