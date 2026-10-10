# ADR-0004: NilHAL Capability and Error Semantics

## Status
Accepted

## Context
Early prototypes often returned `Ok(())` or displayed hardcoded demo state (such as fake VoLTE bars or fixed battery percentages) even when hardware backends were absent or uninitialized. This masked integration failures and created false impressions of hardware readiness.

## Decision
NilHAL interfaces must distinguish between:
1. `Success(value)` — operation succeeded on actual backend.
2. `Queued(id)` — asynchronous operation accepted into bounded queue.
3. `PermissionDenied` — sandbox or user consent refused access.
4. `DeviceUnavailable` — hardware node or service endpoint absent.
5. `CapabilityUnsupported` — hardware physically lacks feature (e.g. no torch, no cellular modem).
6. `Timeout` / `IoError(error)` — hardware communication failure.

The UI shell and applications must never infer operational success solely from an event trigger or button tap.

## Consequences
- Prototype screens display explicit `[SIMULATED]` or `Unavailable` indicators until real drivers are evidenced.
- Fake/mock backends are strictly isolated to unit testing (`FakeBackend`) or demo profiles.
- Subsystem maturity in `docs/maturity.toml` cannot be elevated without concrete error/success telemetry.

## Validation
- `nilhal` test suite validates error propagation across Linux, Android, and Fake backends.
