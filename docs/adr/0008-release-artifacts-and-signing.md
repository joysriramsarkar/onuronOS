# ADR-0008: Release Artifacts and Cryptographic Signing Policy

## Status
Accepted

## Context
A clear boundary must separate development builds, test images, application packages, and official operating system releases. Private keys must never be committed to source control or exposed in build artifacts.

## Decision
1. **Key Hierarchy Separation**:
   - **OS Root of Trust Key**: Signs OTA update manifests and system verified boot descriptors. Managed out-of-band in secure HSM / CI secrets.
   - **App Publisher Keys**: Used by developers to sign `.nilax` packages. Verified by `nilpkg` during installation against the system publisher trust store.
   - **Development Test Keys**: Local, ephemeral Ed25519 keys generated for test fixtures and clearly labeled as `test_only`.
2. **Repository Cleanliness**:
   - Private keys, keystores (`.keystore`, `.jks`, `.pem`, `.key`), generated APKs, and `.gradle` caches are prohibited from git tracking.
3. **Artifact Integrity**:
   - Every official build produces a canonical `manifest.json` and `checksums.txt` containing target triple, git revision, build timestamp, and SHA-256 hashes of all component binaries.

## Consequences
- Leakage of signing credentials is eliminated by policy and CI checks.
- Build outputs are reproducible and tamper-evident.

## Validation
- CI security audit (`cargo audit`, git status check) ensures no tracked secrets or generated artifacts.
- Unit tests (`nilupd`, `nilpkg`) verify cryptographic signature enforcement and rejection of tampered packages.
