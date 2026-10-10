# Architecture Decision Records

Use an ADR for architectural decisions that are not already fixed by the
canonical OnuronOS context.

## When to write an ADR

Write one when deciding or changing items such as:

- GPU backend
- audio backend
- NilLang bytecode format
- package compression
- SoftBus transport
- Android container vs VM
- application-store strategy
- convergence/desktop UI strategy
- a major IPC/interface change
- a major security-boundary change

## Minimal ADR format

```markdown
# ADR-NNNN: Decision title

## Status
Proposed | Accepted | Rejected | Superseded

## Context
What problem are we solving?

## Decision
What are we choosing?

## Alternatives considered
What other options were evaluated?

## Consequences
What becomes easier, harder, faster, slower, safer, or riskier?

## Validation
How will the decision be tested or falsified?
```

## Index of Accepted ADRs

| ADR | Title | Status | Date |
| --- | ----- | ------ | ---- |
| [ADR-0001](./0001-native-os-vs-android-host.md) | Native OS vs Android-Host Boundary | Accepted | 2026-10-09 |
| [ADR-0002](./0002-target-identity-and-canonical-naming.md) | Target Identity and Canonical Naming | Accepted | 2026-10-09 |
| [ADR-0003](./0003-kernel-source-strategy.md) | Kernel Source Strategy & Pinned Checksums | Accepted | 2026-10-09 |
| [ADR-0004](./0004-nilhal-capability-and-error-semantics.md) | NilHAL Capability and Error Semantics | Accepted | 2026-10-09 |
| [ADR-0005](./0005-nillang-runtime-model.md) | NilLang Execution and Runtime Model | Accepted | 2026-10-09 |
| [ADR-0006](./0006-verified-boot-and-flashing-safety.md) | Verified Boot Trust Chain and Flashing Safety | Accepted | 2026-10-09 |
| [ADR-0007](./0007-first-native-reference-phone.md) | First Native Reference Phone Selection | Accepted | 2026-10-09 |
| [ADR-0008](./0008-release-artifacts-and-signing.md) | Release Artifacts and Cryptographic Signing Policy | Accepted | 2026-10-09 |
| [ADR-0009](./0009-alap-framework-role-and-boundaries.md) | Alap Framework Role, Boundaries, and Platform Integration | Accepted | 2026-10-10 |

Do not use ADRs to silently override the canonical architecture. A decision that
changes a fixed architectural choice requires explicit maintainer approval.

