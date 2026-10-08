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

Do not use ADRs to silently override the canonical architecture. A decision that
changes a fixed architectural choice requires explicit maintainer approval.
