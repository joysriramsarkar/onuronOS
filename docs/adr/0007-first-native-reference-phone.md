# ADR-0007: First Native Reference Phone Selection

## Status
Accepted

## Context
Bringing up a native Linux mobile OS across multiple disparate smartphones simultaneously fragments engineering focus. An explicit reference hardware candidate must be chosen based on unlockability, mainline Linux community support, EDL emergency recovery paths, and hardware accessibility.

## Decision
1. **Reference Target Candidate**:
   - **OnePlus 6T (`fajita`)** (Snapdragon 845 SoC) is selected as the primary physical bring-up candidate for Track C (Native Mobile OS).
   - Rationale: Fully unlockable bootloader, mature mainline Linux kernel support (developed and validated within postmarketOS / Linux 6.x mobile initiatives), Qualcomm MSM Download Tool (EDL) recovery route preventing permanent bricks, and active developer documentation.
2. **Role of Samsung Galaxy S25**:
   - The Samsung Galaxy S25 remains the primary evaluation target for **Track B (Hosted Android Runtime)**.
   - It is not a candidate for native bare-metal flashing due to locked bootloaders in retail models, Knox security fuses, proprietary vendor tree constraints, and lack of mainline kernel availability.
3. **Stage-Gate Prerequisite**:
   - Physical device acquisition and flashing will occur only after Stage-Gates A (clean QEMU x86_64/ARM64 green baseline) and B (S25 hosted JNI bridge stability) are achieved.

## Consequences
- Engineering effort is focused on one single hardware bring-up path.
- The platform avoids unsupported device porting claims.

## Validation
- `docs/evidence/oneplus-fajita/` holds device identity specifications, partition layouts, and bring-up logs.
