# 🔒 NilOS Security Architecture & Sandboxing

Security and user privacy are foundational requirements of NilOS. The architecture enforces defense-in-depth through mandatory access control, cryptographic verification, hardware-backed encryption, and process isolation.

---

## 1. SELinux Policy Engine (CIL)

NilOS implements Common Intermediate Language (CIL) SELinux policies in enforcing mode:

```
security/selinux/
├── 00-base.cil         # System types, roles, classes, permissions
├── 10-domains.cil      # Core daemon isolation & execution transitions
├── 20-app-mcs.cil      # Multi-Category Security (MCS) per-app isolation
├── 90-neverallow.cil   # Inviolable security constraints (neverallow rules)
└── file_contexts       # Cryptographic file label mappings
```

### Inviolable Invariants (`90-neverallow.cil`)
- **No Direct Hardware Access**: User applications can never read or write directly to raw device nodes (`/dev/block/*`, `/dev/kmem`, `/dev/mem`).
- **No Unconfined Daemons**: Every service operates within a strictly defined domain with minimal privileges.
- **Immutable System**: Read-only system image cannot be remounted read-write by any non-recovery process.

---

## 2. Process Sandboxing (`nilrt`)

Every native and third-party application runs inside an isolated sandbox managed by the `nilrt` runtime:

1. **Linux Namespaces**:
   - `CLONE_NEWPID`: Process cannot see other processes on the host.
   - `CLONE_NEWNS`: Private mount namespace with only required directories exposed.
   - `CLONE_NEWNET`: Network access restricted unless explicitly granted by permission broker.
   - `CLONE_NEWIPC`: Isolated inter-process communication queues.
2. **Seccomp-BPF Filters**:
   - Blacklists dangerous system calls (`ptrace`, `kexec_load`, `bpf`, `reboot`).
   - Limits file descriptor manipulations and raw socket creations.
3. **Permission Broker (`permbroker`)**:
   - Capabilities (Camera, Location, Microphone, Contacts, Network) must be brokered over UNIX socket requests to `nilrt`.
   - Permissions support one-time grants, per-session grants, and automatic revocation after inactivity.

---

## 3. Storage Encryption (`fscrypt v2` & `nilkeyd`)

User data partitions (`/data/user/<uid>`) are protected using native Linux `fscrypt v2`:
- Encryption cipher: AES-256-XTS for file contents, AES-256-CTS for filenames.
- Master keys are derived from user credentials (PIN, passphrase, biometric) and enrolled into a secure enclave / TEE keystore.
- Keys are managed at runtime by `nilkeyd`:
  - Enrolled on initial unlock.
  - Automatically evicted from memory on device lock or timeout.
  - Per-app directory separation ensures one compromised app cannot decrypt data belonging to another.

---

## 4. Zero-Telemetry Charter

NilOS guarantees complete telemetry-free operation:
- **No Diagnostics Call-Homes**: No analytical pings, unique device identifiers, or usage statistics are transmitted to remote servers.
- **Auditable Builds**: All binaries are built through reproducible pipelines so users can independently verify byte-for-byte fidelity with the source code.
- **Local-Only Crash Reporting**: `crashd` writes tombstone diagnostics strictly to local encrypted storage (`/data/log/crash/`).
