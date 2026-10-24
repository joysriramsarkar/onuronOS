# অনুরণ ওএস: বুট থেকে প্রতিটি অ্যাপ পর্যন্ত বাস্তবায়ন, হার্ডওয়্যার পোর্টিং ও যাচাইয়ের পূর্ণাঙ্গ ব্লুপ্রিন্ট

**রিপো:** <https://github.com/joysriramsarkar/onuronOS>  
**প্রস্তুতির তারিখ:** ২৩ অক্টোবর ২০২৬  
**উদ্দেশ্য:** বর্তমান প্রোটোটাইপকে ধাপে ধাপে বাস্তব, পরীক্ষাযোগ্য, হার্ডওয়্যার-নির্দিষ্ট মোবাইল Linux OS-এ রূপান্তর করা।  
**অবস্থা:** এটি রিপো-পর্যবেক্ষণভিত্তিক বাস্তবায়ন-নির্দেশিকা। এই নথি নিজে থেকে OS-কে সম্পূর্ণ করে না; প্রতিটি ধাপের কোড, পরীক্ষা এবং নির্দিষ্ট ফোনে প্রমাণ দরকার।

---

## ০. আগে সবচেয়ে গুরুত্বপূর্ণ সত্যগুলো

“একেবারে বুট থেকে সব অ্যাপ, ব্রাউজার, টার্মিনাল ও Android bridge”—এটি একটি বড় operating-system engineering programme, একটি UI task নয়। কোনো একক কোড ব্লক বা AI prompt দিয়ে সব Android ফোনে নির্ভরযোগ্য বুট নিশ্চিত করা যায় না। Kernel, bootloader, DTB/DTBO, firmware, partition layout, display/touch, power, Wi-Fi, audio, camera, modem, suspend/resume—প্রতিটি নির্দিষ্ট SoC ও ডিভাইসের সঙ্গে যুক্ত।

**এই ব্লুপ্রিন্টের নিয়ম:**

1. **রিলিজ বিল্ডে কোনো fake provider, simulated data, placeholder success বা silent fallback চলবে না।** হার্ডওয়্যার-সুবিধা অনুপস্থিত হলে `Unsupported`/`DeviceNotFound`-এর মতো প্রকৃত error UI-তে দেখাতে হবে; কাল্পনিক “Connected”, “120 Hz”, “5G” বা ক্যামেরার ছবি দেখানো যাবে না।
2. **একটি করে নির্দিষ্ট target সমর্থন করতে হবে।** এখনকার নথিতে `qemu-x86_64`, `qemu-aarch64`, `android-host-arm64` এবং `oneplus-fajita` আলাদা target। `android-host-arm64` মানে Android অ্যাপের ভেতরে runtime; এটি bare-metal OS নয়।
3. **কোড আছে মানেই ফিচার implemented নয়।** সাফল্য মানে automated test + বাস্তব integration test + evidence/log; UI screenshot যথেষ্ট নয়।
4. **পরীক্ষাহীন বাস্তব ফোনে ফ্ল্যাশ নয়।** রিপোর `targets/oneplus-fajita/target.toml`-এ বর্তমানে `flashing_allowed = false`; এই gate বদলানো যাবে না যতক্ষণ না kernel/DTB/boot image এবং recovery পরীক্ষায় উত্তীর্ণ হয়।
5. **একটি স্থিতিশীল সিস্টেম তৈরি করতে পরিণত প্রযুক্তি ব্যবহার করতে হবে।** নিজস্ব NilLang/Alap/NilUI/`.nilax`-কে অনুরণের পরিচয় রাখা যায়; কিন্তু browser engine, kernel driver, TLS, audio codec, camera stack বা Android framework শূন্য থেকে পুনর্লিখন করা বাস্তবসম্মত নয়।

## ১. রিপো অডিট: সরাসরি দেখা বর্তমান ফাঁক

নীচের পর্যবেক্ষণগুলো রিপোর বর্তমান `main` branch-এর ফাইল থেকে করা হয়েছে। এগুলো “কখনো কাজ করেনি” এমন দাবি নয়; সংশ্লিষ্ট ফাইল/পথে প্রকৃত integration প্রমাণিত নয় অথবা কোডে fallback/stub স্পষ্ট আছে।

| ক্ষেত্র | রিপোতে দেখা অবস্থা | বাস্তবায়ন নির্দেশ |
|---|---|---|
| Build/RootFS | `build/build.sh` প্রয়োজনীয় binary না থাকলেও `if [ -f ... ]; then install ...; fi` দিয়ে এগিয়ে যায়। শেষে build complete বলে। | Release build-এ সমস্ত mandatory binary না থাকলে build fail করবে; incomplete rootfs কখনো সফল artifact নয়। |
| Build manifest | `build/build.sh` ও `build/build.py` `qemu_boot = planned_smoke`, `avb = not_implemented`, `hardware_validation = not_run`-এর পাশাপাশি source/architecture status-এ অতি-আশাবাদী value লিখতে পারে। | status অবশ্যই বাস্তব test run থেকে তৈরি হবে; কোনো test না হলে `not_run`। |
| `nilinit` / data | `nilinit/src/main.rs`-এ `/data` mount ও fallback logic আছে; `docs/maturity.toml`-এ persistent storage-র tmpfs fallback স্পষ্ট। | Release-এ encrypted/persistent `/data` mount ব্যর্থ হলে warning দিয়ে normal boot চালানো যাবে না। Recovery বা safe degraded mode আবশ্যক। |
| Shell/compositor | `shell/nilshell.c`-এর দৃশ্যমান `main()` server struct ও init message তৈরি করে, তারপর `sleep(60)` loop চলে। এতে বাস্তব wlroots Wayland backend, output, input seat বা surface loop তৈরি দেখা যায় না। | প্রকৃত compositor framework-কে সঠিকভাবে initialize, input route এবং surface composition করতে হবে; শুধুমাত্র “compositor running” log নিষিদ্ধ। |
| Terminal | `shell/src/simulated.rs` বলছে terminal-এর `modem`, `audio`, `camera` commands-এ simulated fallback রয়েছে। | বাস্তব terminal PTY/child process চালাবে; system commands বাস্তব sysfs/procfs/service API ব্যবহার করবে। অনুপস্থিত ডিভাইসে স্পষ্ট error। |
| Native browser | `apps/browser/src/main.rs`-এ URL/title/progress UI state বদলায়; Rust entrypoint-এ web engine/page renderer-এর integration দেখা যায় না। `apps/browser/src/main.nil`-এ `nil/web::WebView` API ব্যবহৃত, কিন্তু এই contract-কে বাস্তব platform adapter ও tests দিয়ে প্রমাণ করতে হবে। ArkTS `Index.ets`-এ স্থির bookmark/title/text থেকে “SSL/TLS 1.3”/privacy দাবি দেখানো হয়েছে। | বাস্তব browser engine embed করতে হবে; TLS/secure indicator engine-এর certificate/navigation state থেকে আসবে। static content-কে browser বলে দেখানো যাবে না। |
| Android Hosted JNI | `NativeBridge.java` native library না পেলে `isNativeLoaded=false`, protocol version `0` দিয়ে pure hosted mode-এ চলে। | Hosted fallback-কে view/demo mode হিসেবে পরিষ্কার label করতে হবে; JNI-enabled runtime-এর success বলা যাবে না। Production hardware bridge-এর required feature অনুপস্থিত হলে startup diagnostic fail/limited state হবে। |
| Android Hosted device metrics | `android-host/src/display.rs`-এ 1080×2340 ও 120 Hz default; `telephony.rs`-এ default SIM-ready/carrier value; camera module-এ test JPEG fixture রয়েছে। | Real values না এলে `Unknown/Unsupported`; test fixture কেবল `#[cfg(test)]` বা স্পষ্ট simulator feature-এ। |
| Android app compatibility | `shell/src/simulated.rs`-এ Android container status placeholder এবং “no LXC container is started” লেখা। `android/agent` container-এর ভেতরে `am` ব্যবহার করে command চালানোর agent; এটি নিজে Android container তৈরি করে না। | `nilandroidd`-কে সত্যিকারের container lifecycle, Android system image, Binder, graphics/input/audio bridge ও app launch পরিচালনা করতে হবে। Agent-কে container-এর প্রাথমিক পর্যায়ের বদলি ভাবা যাবে না। |
| Bare-metal mobile target | ADR-0007 OnePlus 6T (`fajita`) বেছে নিয়েছে; S25 Hosted Runtime আলাদা। কিন্তু target config-এ flash নিষ্ক্রিয় এবং hardware validation হয়নি বলে docs জানায়। `build/mkiso-fajita.py`-এর generated FDT-কে আসল device DTB হিসেবে ধরা যাবে না—এটি পূর্ণ SDM845/OnePlus hardware description-এর বিকল্প নয়। | community-validated kernel/device tree/firmware থেকে build system-এ reproducible target integration আনতে হবে। আসল DTB/DTBO এবং boot chain যাচাই না হওয়া পর্যন্ত flash নিষেধ। |
| Production readiness | `docs/maturity.toml`, `AGENTS.md`, `docs/reference-board.md` নিজেরাও বলে code presence hardware support-এর প্রমাণ নয়। | এই নিয়মই CI gate হবে; maturity উন্নীত হবে evidence দিয়ে, README লেখার মাধ্যমে নয়। |

প্রাসঙ্গিক ফাইল: [AGENTS.md](https://github.com/joysriramsarkar/onuronOS/blob/main/AGENTS.md), [README.md](https://github.com/joysriramsarkar/onuronOS/blob/main/README.md), [PLAN.md](https://github.com/joysriramsarkar/onuronOS/blob/main/PLAN.md), [boot-process.md](https://github.com/joysriramsarkar/onuronOS/blob/main/docs/boot-process.md), [reference-board.md](https://github.com/joysriramsarkar/onuronOS/blob/main/docs/reference-board.md), [maturity.toml](https://github.com/joysriramsarkar/onuronOS/blob/main/docs/maturity.toml), [simulated.rs](https://github.com/joysriramsarkar/onuronOS/blob/main/shell/src/simulated.rs), [browser main.rs](https://github.com/joysriramsarkar/onuronOS/blob/main/apps/browser/src/main.rs), [nilshell.c](https://github.com/joysriramsarkar/onuronOS/blob/main/shell/nilshell.c).

---

## ২. লক্ষ্য আর্কিটেকচার: স্থির রাখার নিয়ম

এই লেয়ারগুলো আলাদা রাখতে হবে:

```text
Firmware / unlockable bootloader
        │  verified, device-specific boot artifact
        ▼
Linux LTS kernel + real DTB/DTBO + matching firmware/modules
        │
        ▼
initramfs: /init → nilinit (PID 1)
        │  real block device; no silent /data tmpfs success
        ▼
base rootfs (/system or tested root image) + /vendor + persistent /data
        │
        ├── security: SELinux + namespaces + seccomp + UID/GID + permissions
        ├── services: nild, netd, inputd, powerd, audiod, camerad, telephonyd…
        ├── compositor: actual DRM/KMS + real evdev input + surfaces
        ├── runtime: nilrt + NilVM + Alap + NilUI
        ├── package manager: signed .nilax, atomic install/rollback
        ├── native apps: launcher, settings, files, notes, terminal, browser…
        └── optional Android compatibility: LXC/container + real Android image + Binder + graphics bridge
```

### Track A — QEMU native OS

প্রথমে `qemu-x86_64`; তারপর `qemu-aarch64 -M virt`। এই track-এ boot, root filesystem, supervision, persistent data, package manager, app runtime, IPC/security এবং UI behavior প্রমাণ করতে হবে। QEMU pass মানে শুধু QEMU pass—ফোনের kernel/driver support নয়।

### Track B — Android Hosted Runtime

`android-host-arm64`/Samsung S25-এ `.apk` host Activity, Surface, JNI, Android permissions ও Android API ব্যবহার করে Onuron runtime চালায়। এই track-এ Android framework ও host permission-এর সীমা থাকবে। এটি ফোনের Android OS বদলায় না। Native bridge অনুপস্থিত থাকলে status-এ তা জানাতে হবে।

### Track C — Native mobile OS

রিপোর ADR অনুযায়ী candidate হল OnePlus 6T `fajita`; এটি এখনও validated release target নয়। প্রথমে supported mainline kernel/device-tree/firmware combination থেকে device boot path তৈরি করতে হবে, তারপর Onuron userspace। S25-কে bare-metal target হিসেবে ধরবে না।

### Android APK compatibility আলাদা প্রকল্প

Bare-metal Linux-এ Android APK চালাতে চাইলে Android framework, ART, Binder, package manager, permissions, SurfaceFlinger/HWComposer path, audio, input, graphics, app lifecycle, storage ও SELinux mapping লাগে। `nilagent` এই Android environment-এর ভেতরের helper; এটি সম্পূর্ণ Android runtime নয়।

Waydroid-এর ডকুমেন্টেশন দেখায় যে Linux container-এ Android system চালানোর জন্য namespaces-এর পাশাপাশি kernel Binder এবং `ashmem` অথবা `memfd` support যাচাই করতে হয়; উপযুক্ত compositor/graphics integration-ও দরকার। আগে existing proven Android image/container architecture পরীক্ষা করো, “binder shim” নাম দিয়ে Binder protocol নতুন করে অনুমান করে লেখো না: [Waydroid official docs](https://docs.waydro.id/), [Waydroid kernel troubleshooting](https://docs.waydro.id/debugging/getting-essential-information).

---

## ৩. Phase 0 — স্থিতিশীল baseline, branching ও সত্য-ভিত্তিক maturity

### ৩.১ প্রথমে clean checkout-এ যা চালাবে

Linux build host বা WSL2/Linux VM ব্যবহার করো। Windows native Rust tests সম্ভব হলেও kernel/QEMU/device image toolchain-এর জন্য reproducible Linux environment সহজ হবে। repository-তে নির্দিষ্ট toolchain pin আছে; সেটি update করতে হলে আলাদা PR/ADR দরকার।

```bash
git clone https://github.com/joysriramsarkar/onuronOS.git
cd onuronOS
git status --short
rustc --version
cargo --version
python3 --version
cargo fmt --all -- --check
cargo test --workspace
python3 build/gen-maturity.py --check
python3 -m unittest discover -s build -p 'test_*.py'
```

পরের ধাপে QEMU tools ও system dependencies ইনস্টল করে existing `build/qemu-smoke.py`, `build/qemu-persistence-test.py` এবং সংশ্লিষ্ট CI workflow-এর exact command চালাতে হবে। যে test চালানো হয়নি, তার status `not_run`; test skip হয়েছে বলে pass নয়।

### ৩.২ Build script-এর fail-open আচরণ বদলাও

`build/build.sh`/`build/build.py`-তে **compile failure ছাড়া “সব ঠিক” বলে এগিয়ে যাওয়া** বন্ধ করতে হবে। বিশেষ করে optional `if [ -f ... ]; then install ...; fi` pattern core service binary-র ক্ষেত্রে চলবে না। Build output-কে temporary staging directory-তে বানিয়ে সব checks pass হলে তবেই final `out/<target>` replace করবে। পুরোনো ভালো image আগে থেকে মুছে দিয়ে মাঝপথে incomplete build রেখে দেওয়া যাবে না।

**প্রথম নতুন file:** `build/required-artifacts.toml`। target অনুযায়ী mandatory binaries, libraries, kernel, initramfs, config, firmware-manifest ও app packages তালিকাভুক্ত করো। `etc/nilos/services.toml`-এ যে service enable আছে, তার executable অবশ্যই final rootfs-এ থাকতে হবে অথবা target configuration-এ feature-টি explicit `unsupported`/disabled হতে হবে।

নিচে একটি minimal check script-এর ভিত্তি দেওয়া হল। Binary list-টি `required-artifacts.toml` থেকে পড়তে হবে; এখানে illustrative core list দেওয়া হয়েছে। Repo-র actual executable naming ও architecture checker-এর সাথে মিলিয়ে integrate করতে হবে।

```python
#!/usr/bin/env python3
"""Fail-closed check for a packaged Onuron rootfs."""
from __future__ import annotations
import argparse
import json
import subprocess
import sys
from pathlib import Path

CORE_BINARIES = (
    "nilinit", "nild", "nilkeyd", "nilbus", "netd", "powerd",
    "inputd", "audiod", "notifyd", "nilpkg", "nilrt-launch",
    "nilshell", "launcher", "settings", "nilrecovery",
)

EXPECTED_MACHINE = {
    "x86_64": ("Advanced Micro Devices X86-64", "Advanced Micro Devices X86-64"),
    "aarch64": ("AArch64", "AArch64"),
}

def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--rootfs", type=Path, required=True)
    p.add_argument("--arch", choices=EXPECTED_MACHINE, required=True)
    a = p.parse_args()

    root = a.rootfs.resolve()
    bin_dir = root / "usr/bin"
    missing = [name for name in CORE_BINARIES if not (bin_dir / name).is_file()]
    if missing:
        print("ERROR: missing mandatory executables:", ", ".join(missing), file=sys.stderr)
        return 2

    readelf = subprocess.run(["readelf", "-h", str(bin_dir / "nilinit")],
                             text=True, capture_output=True)
    if readelf.returncode != 0:
        print("ERROR: readelf could not inspect nilinit ELF", file=sys.stderr)
        return 3
    expected = EXPECTED_MACHINE[a.arch][0]
    machine_line = next((line for line in readelf.stdout.splitlines()
                         if line.strip().startswith("Machine:")), "")
    if expected not in machine_line:
        print(f"ERROR: target mismatch: expected {expected!r}; got {machine_line!r}",
              file=sys.stderr)
        return 4

    services_file = root / "etc/nilos/services.toml"
    if not services_file.is_file():
        print("ERROR: services.toml missing from image", file=sys.stderr)
        return 5

    # Further checks must parse TOML and verify every enabled service exec path,
    # runtime libs, SELinux policy, firmware manifest, and package signatures.
    # A successful file-presence check is NOT a boot test or hardware test.
    print(json.dumps({"rootfs": str(root), "arch_check": "passed",
                      "mandatory_binaries": len(CORE_BINARIES),
                      "qemu_boot": "not_run", "hardware_validation": "not_run"},
                     indent=2))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
```

এই script একা production audit নয়: এতে service manifest parsing, ELF check সব binary-তে, dynamic dependency, file modes, SELinux labels/policy, firmware hashes, required app packages এবং signature verification যোগ করতে হবে। চেক পাস হলেও QEMU/hardware status `not_run`-ই থাকবে যতক্ষণ আলাদা test না চলে।

### ৩.৩ Manifest-এ মিথ্যা pass নয়

Build manifest-এ আলাদা field রাখো:

```json
{
  "build_target": "qemu-aarch64",
  "git_revision": "<git rev-parse HEAD>",
  "build_status": "passed",
  "rootfs_validation": "passed",
  "elf_architecture_validation": "passed",
  "qemu_boot_test": "passed",
  "persistent_storage_test": "passed",
  "security_test": "not_run",
  "physical_hardware_test": "not_applicable",
  "release_signature": "not_signed"
}
```

এই JSON উদাহরণ; build script যেন `passed`-গুলো test command-এর exit code ও evidence artifact থেকে populate করে। কোনো status হাতে লিখে final artifact-এ দেওয়া যাবে না। Physical target-এর জন্য `physical_hardware_test` শুধু signed test report/evidence থাকলে `passed` হবে।

### ৩.৪ Maturity policy

- `IMPLEMENTED`: বাস্তব integration test আছে; fake backend ব্যবহার হয়নি।
- `FUNCTIONAL PROTOTYPE`: নির্দিষ্ট environment/target-এ tested কিন্তু production hardening বাকি।
- `EXPERIMENTAL`: interface/driver partial অথবা test coverage অপর্যাপ্ত।
- `STUB/SIMULATED`: UI demo, fake provider বা fabricated result।
- `PLANNED`: source path বা working implementation নেই।

CI যেন README-র maturity table শুধু evidence path আছে কি না দেখে নয়, test artifact/target identifier-ও যাচাই করে। Documentation-এ hardware-এ test না করা feature-কে “production-ready” করা যাবে না।

---

## ৪. Phase 1 — Kernel, boot chain, initramfs এবং RootFS

### ৪.১ Kernel download মানেই device kernel নয়

`targets/qemu-aarch64/target.toml`-এর `vmlinuz-virt` QEMU `virt` machine-এর জন্য; এটি OnePlus 6T kernel নয়। ফোন target-এ বাস্তব kernel source tree, matching `.config`, `Image`, DTB/DTBO, modules, firmware, boot image layout, command line এবং vendor integration-কে একসঙ্গে pin করতে হবে।

OnePlus 6T-এর community Linux port আছে; postmarketOS-এর device page বর্তমান hardware support-কে per-feature status হিসেবে বর্ণনা করে—যেমন screen/touch, Wi-Fi/Bluetooth, audio/camera/modem সব সমান maturity-তে নয়। সেটিকে reference হিসেবে পড়ো, “আমাদের OS-এ সব কাজ করবে” এমন প্রমাণ হিসেবে নয়: [OnePlus 6T postmarketOS page](https://wiki.postmarketos.org/wiki/OnePlus_6T_%28oneplus-fajita%29).

### ৪.২ Device configuration-এ explicit facts

`targets/oneplus-fajita/target.toml`-এ নিম্নলিখিত field যোগ/শক্ত করো:

- kernel source URL/repository + pinned commit/tag + license
- kernel config hash এবং config fragment list
- exact DTB/DTBO source path ও SHA-256
- module list ও `vermagic` compatibility
- firmware filenames + provenance/license/hash
- boot image header/version, page size, offsets ও tested command line
- exact device variant (A6010/A6013), active slot behaviour, bootloader state
- partition map validated on an owned test device
- feature matrix: display, touch, power, storage, Wi-Fi, BT, audio, camera, modem, GPS, suspend
- `flashing_allowed = false` যতদিন mandatory gates অসম্পূর্ণ

**একটি synthetic/minimal DTB আসল ফোনের DTB নয়।** `build/mkiso-fajita.py`-তে generated minimal FDT আছে; তাতে SDM845-এর সব bus, regulator, clocks, GPIO, panel, touch, PMIC, storage, modem, IOMMU, reserved-memory ও interrupt mapping নেই। এই generated FDT দিয়ে phone boot image তৈরি বা flash করা যাবে না। আসল device tree source থেকে reproducible DTB/DTBO build করো।

### ৪.৩ Initramfs-এর mandatory contract

Initramfs-এ থাকা দরকার:

- target architecture-এ valid executable `/init` (অথবা `/init` symlink/launcher) → `nilinit`
- প্রয়োজনীয় early kernel modules ও `modules.dep`/load order
- `/dev/console`, devtmpfs/proc/sysfs, `/run`, `/tmp`
- rootfs/data block device আবিষ্কারের logic
- ext4/যে filesystem নির্ধারিত তার driver; fscrypt ব্যবহারের জন্য kernel support এবং userspace tooling
- recovery executable, diagnostics, emergency console
- SELinux policy/policy version matching kernel/userspace
- system manifest, service configuration, trust roots/public signing keys
- device-specific firmware/module manifests
- logs/debug route (serial/ADB-equivalent recovery; secrets redaction)

Initramfs builder-এ network থেকে আসা kernel image SHA-256 pin করা থাকা প্রয়োজন, কিন্তু module compatibility-ও verify করতে হবে। “কোনও module download করা যায়নি—চলুক” নয়; target-এর required modules বাদ গেলে build fail।

### ৪.৪ `/data` কখনো নীরবে tmpfs হবে না

বর্তমান dev prototype-এ `/data`-র জন্য tmpfs fallback থাকতে পারে। Production phone-এ এর ফলে ব্যবহারকারীর contacts, notes, PIN state, package data ইত্যাদি reboot-এ হারিয়ে যেতে পারে এবং UI তবুও success দেখাতে পারে। Release mode-এ:

1. persistent block device শনাক্ত করো;
2. partition/metadata validate করো;
3. encryption policy ও key availability যাচাই করো;
4. filesystem check/recovery policy প্রয়োগ করো;
5. mount সফল হলে তবেই `data_ready` health signal দাও;
6. mount ব্যর্থ হলে `nilrecovery`-তে যাও বা স্পষ্ট safe limited mode-এ থাকো—নতুন ফাঁকা tmpfs-এ normal boot নয়।

Linux `libc::mount` দিয়ে mount করার একটি বাস্তব baseline নিচে। এটি **unencrypted ext4 mount-এর উদাহরণ**, পুরো encrypted-data lifecycle নয়; production-এ `nilkeyd`/fscrypt key provisioning, device identity, filesystem recovery policy ও boot-state verification আলাদাভাবে যুক্ত করতে হবে। সবচেয়ে গুরুত্বপূর্ণ বিষয়: error হলে function error ফেরায়—tmpfs fallback দিয়ে normal boot সফল দেখায় না।

```rust
use std::{
    ffi::CString,
    fs,
    io,
    os::unix::{ffi::OsStrExt, fs::FileTypeExt},
    path::Path,
};

#[derive(Debug)]
pub enum DataMountFailure {
    DeviceAbsent,
    NotBlockDevice,
    MountFailed(io::Error),
    VerificationFailed(String),
}

fn c_path(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))
}

pub fn mount_persistent_ext4(device: &Path, mountpoint: &Path) -> Result<(), DataMountFailure> {
    let meta = fs::symlink_metadata(device).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound { DataMountFailure::DeviceAbsent }
        else { DataMountFailure::MountFailed(e) }
    })?;
    if !meta.file_type().is_block_device() {
        return Err(DataMountFailure::NotBlockDevice);
    }
    fs::create_dir_all(mountpoint)
        .map_err(DataMountFailure::MountFailed)?;

    let src = c_path(device).map_err(DataMountFailure::MountFailed)?;
    let dst = c_path(mountpoint).map_err(DataMountFailure::MountFailed)?;
    let fs_type = CString::new("ext4").expect("static string contains no NUL");
    let options = CString::new("errors=remount-ro").expect("static string contains no NUL");

    // Requires CAP_SYS_ADMIN. Do not interpolate paths into a shell command.
    let rc = unsafe {
        libc::mount(
            src.as_ptr(), dst.as_ptr(), fs_type.as_ptr(),
            (libc::MS_NODEV | libc::MS_NOSUID) as libc::c_ulong,
            options.as_ptr().cast(),
        )
    };
    if rc != 0 {
        return Err(DataMountFailure::MountFailed(io::Error::last_os_error()));
    }

    // Verify the mount point and filesystem type appeared in the kernel's mount table.
    // /data is a canonical path with no spaces, so field 5 in mountinfo is comparable.
    let mountinfo = fs::read_to_string("/proc/self/mountinfo")
        .map_err(DataMountFailure::MountFailed)?;
    let target = mountpoint.to_string_lossy();
    let verified = mountinfo.lines().any(|line| {
        let Some((left, right)) = line.split_once(" - ") else { return false; };
        let fields: Vec<&str> = left.split_whitespace().collect();
        let right_fields: Vec<&str> = right.split_whitespace().collect();
        fields.get(4).copied() == Some(target.as_ref())
            && right_fields.first().copied() == Some("ext4")
            && right_fields.get(1).copied() == Some(device.to_string_lossy().as_ref())
    });
    if !verified {
        return Err(DataMountFailure::VerificationFailed(
            "mount syscall returned success but /proc/self/mountinfo did not match source/target/type".into()
        ));
    }
    Ok(())
}
```

এই helper ধরে নেয় target path canonical এবং mount source-রূপ `/dev/...`; production integration-এ target-কে fixed allowlist path (যেমন `/data`) বানাও, mountpoint-এর symlink/ownership যাচাই করো, `mountinfo`-এর escaped path সঠিকভাবে parse করো, device-এর major/minor identity যাচাই করো, mount সফল হলেও fscrypt policy/key state আলাদা করে যাচাই করো। কোনো কারণে mount verify না হলে service readiness আটকে recovery-তে যেতে হবে।

### ৪.৫ Boot success marker

`nilinit` boot start-এ persistent failed-boot counter বাড়াবে। সব critical checks পাস হওয়ার আগে counter reset করা যাবে না। “Ready” মানে কমপক্ষে:

- correct target kernel/architecture শনাক্ত;
- required rootfs mount ও `/data` persistent mount valid;
- SELinux mode expected state-এ;
- core system services healthy;
- DRM/KMS output এবং input device target-এর জন্য required হলে ready;
- launcher/runtime startup সফল;
- health daemon বা watchdog final success লিখেছে।

QEMU এবং phone আলাদা health profile পাবে। QEMU-র `virtio-gpu` capability-কে phone DRM capability ধরে নেওয়া যাবে না।

---

## ৫. Phase 2 — Service manager, IPC, health ও real HAL

### ৫.১ Service manifest-কে source of truth করো

`etc/nilos/services.toml`-এর সব enabled service-এর executable, required capability, dependency, socket, timeout, restart policy এবং health check থাকা চাই। Build system যেন enabled service-র binary বাদ পড়লে fail করে। Service crash/restart loop হলে exponential backoff, rate limit এবং recovery state দরকার। `restart = always` সব daemon-এর জন্য blind policy নয়—critical one-shot jobs, failed hardware service ও update service-এর আলাদা restart policy হওয়া উচিত।

প্রত্যেক service-এর জন্য:

- readiness handshake;
- process identity/UID/GID/domain;
- resource limits (cgroup CPU/memory/PIDs/FDs);
- IPC message version/size cap;
- structured logs, request ID ও error code;
- dependency timeout;
- watchdog/health response;
- clean shutdown ও restart recovery;
- hostile input/fuzz test।

### ৫.২ IPC protocol contract

`protocol/`-এ থাকা `ONUR` framed IPC-কে একটাই canonical protocol করো। JSON-line, random TCP localhost port ও Unix socket message variants যেন আলাদা undocumented protocol না হয়ে যায়। Frame minimum fields:

```text
magic="ONUR" | protocol_version | message_kind | request_id |
service_id | payload_length | payload | status/error_code
```

Rules:

- 1 MiB-সহ configured max size-এর বেশি frame reject;
- size validate করার আগে allocation নয়;
- request ID দিয়ে response correlate;
- timeout/cancel semantics;
- unsupported version হলে clear error;
- Unix socket peer credentials/SELinux policy verify;
- socket parent directory owner/mode verify;
- command names allowlist; raw command shell string গ্রহণ নয়;
- async hardware request-কে success বলা যাবে কেবল operation completion/ack পাওয়ার পর;
- queue full হলে event silently drop/pop-front নয়; backpressure/error/metrics;
- privileged operation-এর permission check service boundary-তেও করতে হবে—UI-তে বাটন hide করলেই security নয়।

### ৫.৩ HAL-এ `FakeHAL`-কে production থেকে বাদ দাও

`runtime/nilhal`-এ real Linux backend ও fake test backend-এর construction path আলাদা করো। `FakeHAL` CI/unit test-এ থাকবে; Release build-এ feature flag ভুল করে enable করলে compile failure হওয়া উচিত। `DeviceNotFound`, `UnsupportedOperation`, `PermissionDenied`, `TemporarilyUnavailable`, `HardwareFault` আলাদা করে return করবে। “Battery 100%”, “Wi-Fi connected”, “120 Hz” default value দিয়ে real hardware না থাকার বিষয় ঢেকে রাখা যাবে না।

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityState {
    Available,
    Unsupported { reason: String },
    PermissionDenied { reason: String },
    TemporarilyUnavailable { reason: String },
    Faulted { reason: String },
}

pub trait CapabilityProbe: Send + Sync {
    fn capability_id(&self) -> &'static str;
    fn probe(&self) -> CapabilityState;
}

// Fake implementations belong in test modules or a separately named simulator binary.
// Production constructors must select Linux/target-specific provider, and must return an
// explicit error if the provider cannot be constructed.
```

Release CI policy example:

```rust
#[cfg(all(feature = "sim-backend", not(test)))]
compile_error!("sim-backend must never be enabled in a production binary");
```

Cargo features অনুযায়ী `sim-backend` শুধুমাত্র আলাদা simulator target-এ রাখা আরও ভালো। Release artifact scanner শুধু `[SIMULATED]` লেখা খোঁজার বদলে fake provider types/config route-ও audit করবে।

### ৫.৪ Device capability manifest

`targets/<target>/capabilities.toml` যোগ করো। উদাহরণ:

```toml
[target]
id = "oneplus-fajita"
profile_version = 1
hardware_validation = "not_run"

[display]
provider = "linux-drm-kms"
required_for_boot = true
validation = "not_run"

[input]
provider = "linux-evdev"
required_for_boot = true
validation = "not_run"

[storage]
provider = "linux-block-ext4-fscrypt"
required_for_boot = true
validation = "not_run"

[network_wifi]
provider = "linux-wireless"
required_for_boot = false
validation = "not_run"

[camera]
provider = "linux-v4l2-device-specific"
required_for_boot = false
validation = "not_run"

[telephony]
provider = "device-specific-qmi-mbim-or-vendor-stack"
required_for_boot = false
validation = "not_run"
```

এটা উদাহরণ schema; provider names target অনুযায়ী বাস্তবে নির্ধারণ করতে হবে। `validation = "not_run"` যতক্ষণ test evidence না আসছে থাকবে। `required_for_boot = false` মানে UI-তে fake capability দেখানো নয়; “এই build/ডিভাইসে feature সমর্থিত নয়” দেখানো।

---

## ৬. Phase 3 — বাস্তব compositor, display, touch ও mobile shell

### ৬.১ C/Rust shell duplication নিষ্পত্তি

রিপোতে C `shell/nilshell.c`, `shell/Makefile`, Rust `shell/src/main.rs` এবং `nilsim` আছে। এগুলোর দায়িত্ব লিখিতভাবে আলাদা করো:

- **`nilshell`**: production Wayland compositor/window manager; actual DRM/KMS output, GPU renderer, evdev/libinput input, surface lifecycle, layer-shell/popup/IME, locking/launcher transition.
- **`nilsim`**: development UI simulator; production OS service নয় এবং কোনো hardware validation হিসেবে গণ্য হবে না।
- **Rust `shell/src/main.rs`**: যদি ANSI console diagnostic shell হয়, তার নাম/target আলাদা করো; production GUI compositor-এর বিকল্প নয়।

C ফাইলকে “Wayland Compositor” বলে রাখলেই wlroots compositor হয় না। দুইটি গ্রহণযোগ্য পথের একটিকে বেছে নাও:

**পথ A:** wlroots + libinput + libseat + DRM backend-এর প্রকৃত lifecycle implement; wlroots-এর supported API-র সাথে `shell/nilshell.c`-কে integrate করো।  
**পথ B:** Rust-first compositor framework (যেমন Smithay) দিয়ে production `nilshell` তৈরি করে আগের C stub retire করো।

দুই পথ একসঙ্গে অর্ধেক করে রাখবে না। `docs/adr/`-এ compositor choice, input stack, render API ও fallback policy লিখে দাও।

### ৬.২ Minimum compositor Definition of Done

- DRM device discovery; target-specific display connector/CRTC/plane selection;
- modeset, framebuffer allocation, page flip/vsync, hotplug/recovery;
- GPU/render path বাস্তবে device support করে; software renderer থাকলে তার actual usage/capability telemetry;
- evdev/libinput touch coordinates rotation/scaling;
- keyboard/input method, volume/power/back buttons;
- Wayland client protocol registry, surface commit/damage, frame callbacks;
- focus/input security; clipboard policy;
- notifications, lock screen, OOBE, launcher ও app windows;
- display off/on, suspend/resume, orientation, density/font scaling;
- disconnect/reopen DRM/input device test;
- supported device-এ physical latency/frame and battery measurements।

Kernel documentation-এ DRM/KMS-এর real pipeline (framebuffer → plane/CRTC → connector) বর্ণিত আছে: [Linux DRM/KMS docs](https://docs.kernel.org/gpu/drm-kms.html). Android host-এর Surface bridge আর Linux DRM/KMS compositor—দুটি আলাদা backend; একটিকে অন্যটির test evidence হিসেবে ব্যবহার করবে না।

### ৬.৩ “ready” signal-এর সত্যতা

`nilshell` readiness তখনই পাঠাবে যখন real display backend অন্তত একটি output configure করেছে এবং event loop/input seat চলছে। QEMU headless target-এ আলাদা `headless-test` backend allowed; phone release-এ সেই backend detect হলে fail/limited mode দিতে হবে।

---

## ৭. Phase 4 — NilLang, Alap, NilVM, `.nilax` ও অ্যাপ lifecycle

এটাই অনুরণের নিজস্ব প্ল্যাটফর্ম পরিচয়। তবে compiler/runtime/package flow end-to-end stable না হওয়া পর্যন্ত বেশি app লিখলে debugging জটিল হবে। বর্তমান `docs/LANGUAGE_AND_RUNTIME_CONTRACT.md`-কে canonical contract ধরে implementation/test matrix বানাও।

### ৭.১ Mandatory pipeline

```text
main.nil + resources
      ↓ nilc (syntax/type/capability validation)
.nib bytecode / supported executable format
      ↓ pack manifest + resources + checksums
.nilax package
      ↓ Ed25519 signature verification + trust policy
nilpkg install transaction
      ↓ isolated UID, app data dir, SELinux label, permissions
nilrt launch + bounded NilVM host calls
      ↓ Alap reactive component graph + NilUI renderer
nilshell surface
```

একটি package install/restart/update/uninstall path বাস্তব হতে হবে। UI-তে “installed” দেখানো যথেষ্ট নয়।

### ৭.২ Package Manager

`pkg/nilpkg`-এর জন্য test matrix:

- valid signature / tampered manifest / tampered binary;
- unknown, revoked, expired (যদি policy-তে থাকে) key;
- wrong architecture/minimum OS version;
- archive traversal (`../`), symlink/hardlink/device node, SUID/SGID;
- duplicate path, huge decompressed payload, zip/tar bomb;
- interrupted install / disk full / permission error;
- atomic rename, journaling, crash recovery, rollback;
- uninstall while running / stale process / orphan app-data policy;
- reproducible signing manifest এবং trust-store migration।

Signing private key source code/CI logs-এ থাকবে না; release signing offline/hardware-protected secret environment-এ হবে। Development key ও production trust root আলাদা।

### ৭.৩ Runtime isolation

`runtime/nilrt` শুধু `unshare()` করলে production sandbox হয় না। বাস্তব policy-তে:

- unique non-root UID/GID per app;
- mount namespace, read-only system tree, private writable `/data/user/<uid>`;
- `no_new_privs`, seccomp syscall allowlist, capability drop;
- SELinux domain/type enforcement;
- cgroup memory/process/CPU/file-descriptor limits;
- permission broker-এর default deny ও revocation;
- IPC broker permission verification; path traversal/symlink-safe filesystem APIs;
- per-app crash isolation/log redaction;
- escape attempts: `/proc`, `/sys`, `/dev`, ptrace, namespace, mount, inherited FD, symlink, `..`, hard link, IPC peer spoof tests।

`docs/security.md`, `docs/threat-model.md` ও `docs/adr/0004-nilhal-capability-and-error-semantics.md`-এর decisions-এর সাথে সব test যুক্ত করো।

### ৭.৪ UI framework contract

Alap/NilUI-তে production app-গুলোর জন্য stable contract দরকার: routing, widget lifecycle, input, focus/IME, accessibility tree, font shaping, Bengali conjunct/ligature, bidi, dynamic font scaling, dark/light theme, screen rotation, process restoration, errors, async cancellation। Custom UI engine সব browser HTML/CSS/JS render করবে—এমন design কোরো না; browser engine আলাদা।

---

## ৮. Phase 5 — প্রতিটি native অ্যাপকে সত্যিকারের কার্যকর করা

প্রথমে এক-দুইটি “vertical slice” শেষ করো; তারপর বাকি অ্যাপ। প্রতিটি app-এর একটি testable service contract ও real data source থাকা বাধ্যতামূলক।

| অ্যাপ/স্ক্রিন | প্রকৃত backend | মিথ্যা সাফল্যের উদাহরণ (নিষিদ্ধ) | acceptance test |
|---|---|---|---|
| OOBE/setup | persistent config, real locale/timezone, real user & credential setup | শুধু screen বদলালেই “setup complete” | reboot-এ profile persist; interrupted setup recover |
| Lockscreen | PAM/credential-like authenticated service; salted memory-hard hash; lockout/backoff | hard-coded PIN অথবা plain SHA loop-কে production security বলা | correct/incorrect PIN, lockout, reboot, data migration tests |
| Launcher/app drawer | verified installed-package registry + process state | static list-এ package নেই তবু launch success | install→visible→launch→close→uninstall lifecycle |
| Settings | actual sysfs/service capabilities + persisted settings | battery, screen refresh, SELinux state hardcoded | setting বদলালে backend বদলায়; unsupported হলে disabled + reason |
| Files | filesystem APIs, MIME/type, permissions, safe file operations | `/data` ছাড়া fake folder list | create/rename/copy/delete, storage error, permission tests |
| Notes | real atomic storage/SQLite or defined document store | UI state save না করেও “saved” toast | process kill/reboot persistence, conflict/partial write tests |
| Calculator | tested expression parser; locale/digit formatter | UI click count-কে calculation বলা | precedence, unary signs, trig domain, overflow, Bengali digits tests |
| Clock/alarm | wall clock + monotonic timer + alarm service / resume handling | UI timer app বন্ধ হলে থেমে যাওয়া | reboot, timezone, DST/locale, sleep/wake tests |
| Music player | actual file scanner + decoder + audio backend | ALSA absent অবস্থায় “playing” দেখানো | decode/play/pause/seek/route/error integration test |
| Camera | target driver/HAL; V4L2 or host Camera2 only in Hosted track | test JPEG/black/static frame production capture হিসেবে | real sensor preview/capture/torch and permission test; no sensor ⇒ error |
| Phone/dialer | target telephony service (QMI/MBIM/vendor route as applicable) | entered number-কে call state `ACTIVE` করা | dial/answer/hangup/permissions/SIM loss/emergency behavior on device |
| SMS/Messages | modem/SMS stack or explicitly Hosted Android SmsManager | local JSON-এ save করে “SMS sent” বলা | send receipt, incoming SMS, encoding, SIM/no signal tests |
| Wi-Fi | nl80211/wpa_supplicant/NetworkManager-equivalent target service | fake SSID/IP, static connected badge | scan, auth fail, reconnect, DHCP, DNS, airplane mode tests |
| Bluetooth | BlueZ or well-defined Linux Bluetooth service | UI toggle মাত্র | scan, pair, trust, disconnect, audio accessory tests |
| Display | real DRM/sysfs/display provider | 120Hz hardcoded | modes/brightness/orientation/blank/unblank measured |
| Security | kernel LSM state, verity/fscrypt/signature/permission services | settings page-এ সব toggle green | actual kernel state + denied operation test |
| VPN | WireGuard/OpenVPN backend and routing/DNS leak protection | VPN switch ON but route unchanged | route, DNS, kill-switch, reconnect, credential security tests |
| Backup/restore | encrypted backup format + key lifecycle + transactional restore | fake progress bar | checksum, wrong key, interrupted restore, version migration |
| System updater | signed manifest + A/B/rollback or explicit safe update model | update downloaded but not validated/committed | power-cut-at-every-stage, invalid signature, rollback test |
| SoftBus | actual service discovery + authenticated QUIC sessions + authorization | local in-memory peers while connected badge shows remote | two real processes/devices, auth failure, replay/drop/reconnect tests |
| Accessibility/language/IME/TTS | input method, screen reader, real fonts/TTS provider | toggle UI only | end-to-end keyboard, focus, announcements, Bengali shaping/IME tests |

**প্রথম vertical slice-এর অর্ডার:** boot → persistent data → service readiness → package install → NilLang app launch → screen/input → settings/files/notes → terminal → browser → camera/audio/network → telephony → Android compatibility। Phone-specific drivers এগোনোর সঙ্গে capability matrix update হবে।

### ৮.১ PIN security

README-তে salted/stretched SHA-256 loop-এর কথা আছে। PIN-এর মতো low-entropy secret-এর জন্য শুধু fixed-round SHA-256 production credential storage হিসেবে যথেষ্ট নয়। আলাদা security ADR করে vetted memory-hard password hashing scheme (Argon2id/scrypt/PBKDF2-এর মধ্যে platform-compatible, benchmark করা সিদ্ধান্ত), unique random salt, rate limiting, lockout, secure wipe, migration path ও hardware-backed key availability বিবেচনা করো। `nilkeyd`/fscrypt wiring সম্পূর্ণ না হওয়া পর্যন্ত “encrypted user data” দাবি কোরো না।

---

## ৯. Phase 6 — বাস্তব ব্রাউজার (NilZar) বানানো

### ৯.১ প্রথম সিদ্ধান্ত: engine নিজে লেখা যাবে না

HTML parser, CSS layout, JavaScript engine, JIT, HTTP cache, TLS verification, certificate store, media, font shaping, accessibility ও web platform security—একটি browser engine হলো বিশাল আলাদা প্রকল্প। `.nil` UI screen বা URL field কোনো browser নয়। বর্তমান `apps/browser/src/main.rs`-কে **browser chrome/controller** হিসেবে রাখতে পারো, কিন্তু প্রকৃত engine embed করতে হবে।

Native Linux target-এর জন্য WebKitGTK/WPE WebKit-এর মতো maintained engine বিবেচনা করো; GTK4/WebKitGTK API migration-এর source docs: [WebKitGTK 6.0 migration](https://github.com/WebKit/WebKit/blob/main/Source/WebKit/gtk/migrating-to-webkitgtk-6.0.md), [Rust `webkit6` API](https://docs.rs/webkit6/latest/webkit6/). `webkit6` ব্যবহার করলে device rootfs-এ native library/dependency-ও cross-build করে include করতে হবে; Rust crate একা engine নয়। Android Hosted APK-তে GeckoView বা Android WebView-এর dedicated adapter হতে পারে, কিন্তু তা native Linux browser engine-এর বিকল্প নয়। Mozilla-র docs GeckoView-কে Android browser embedding-এর জন্য আলাদা engine হিসেবে বর্ণনা করে: [GeckoView docs](https://firefox-source-docs.mozilla.org/mobile/android/geckoview/index.html).

Servo-ও Rust browser engine, কিন্তু এর নিজস্ব docs বলছে embedding/webview integration এখনও পরিবর্তনশীল এবং `servoshell`-সহ ব্যবহার/embedding path বিবেচনা করতে হয়; production browser-এর জন্য এটিকে বেছে নিলে integration risk আলাদাভাবে নথিভুক্ত করো: [Servo Book](https://doc.servo.org/).

### ৯.২ Browser architecture

```text
NilZar UI (address bar, tabs, history, bookmarks, downloads, permissions)
        │ typed BrowserController API
        ▼
WebEngineAdapter (native Linux WebKit/WPE OR Android Hosted GeckoView/WebView)
        │
        ├── Navigation/redirect/certificate/load progress callbacks
        ├── Process isolation and crash recovery
        ├── Cookies/site storage/cache per profile
        ├── Download manager + MIME + destination permission
        └── Media/camera/mic permission broker
```

একই UI controller-এর নিচে target-specific engine adapter রাখা যায়, তবে feature capability explicit হবে। Browser UI-এর “TLS secure” icon engine certificate/transport info না দিলে দেখানো যাবে না। HTTPS URL হওয়া বা toolbar-এ lock icon আঁকা TLS success-এর প্রমাণ নয়। HTTP/non-secure origin-এ warning লাগবে; certificate error bypass করার default button নয়।

### ৯.৩ Browser mandatory functions

- URL normalizer (scheme, host, IDNA, invalid URL, query vs domain parsing);
- load/stop/reload/back/forward, redirect, progress/title/error callback;
- tab lifecycle, private mode, session restore, crash restoration;
- bookmark/history local DB with atomic transactions and clear-all;
- downloads with destination picker, content length limit, resume policy, MIME and filename sanitization;
- cookie/site data permissions, clear site data, private context;
- permissions: camera, microphone, location, notifications—default deny, per-origin decisions;
- TLS certificate details from engine API; do not implement own certificate bypass;
- safe external protocol handling; downloads cannot silently launch arbitrary intent;
- sandboxed web process and content process; popup/permission abuse tests;
- Bengali fonts, IME/composition, selection/copy/paste, pinch zoom, accessibility;
- engine version/update/SBOM/licence tracking;
- fuzz + regression suite for URL parser, history DB migration, download path traversal and hostile content.

### ৯.৪ Browser controller-এর typed interface

এটি engine নয়; UI এবং actual engine adapter-এর stable seam:

```rust
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct NavigationRequest {
    pub url: String,
    pub tab_id: u64,
    pub user_gesture: bool,
}

#[derive(Debug, Clone)]
pub enum NavigationEvent {
    Started { tab_id: u64, url: String },
    Progress { tab_id: u64, fraction: f32 },
    Committed { tab_id: u64, url: String, title: String },
    Finished { tab_id: u64 },
    Failed { tab_id: u64, code: String, safe_message: String },
    CertificateWarning { tab_id: u64, host: String, description: String },
}

pub trait WebEngine: Send + Sync {
    fn navigate(&self, request: NavigationRequest) -> Result<(), String>;
    fn stop(&self, tab_id: u64) -> Result<(), String>;
    fn go_back(&self, tab_id: u64) -> Result<(), String>;
    fn go_forward(&self, tab_id: u64) -> Result<(), String>;
    fn reload(&self, tab_id: u64) -> Result<(), String>;
}

// Production rule: UI may display loading only after the engine accepts the request;
// title/progress/TLS badge/history are driven by real engine callbacks.
// A missing target adapter must return an explicit unsupported error, never draw a fake page.
```

**Acceptance test:** test server থেকে local HTTPS page (valid cert), invalid-cert page, redirect, JS/CSS, image, download, 404, offline, slow load, tab close during load, permission denial, private profile, malicious download filename, Bengali input—সব test engine-এ চলবে। Browser UI tests একা যথেষ্ট নয়।

### ৯.৫ বর্তমান ArkTS page-এ সংশোধন

`apps/browser/src/main/ets/pages/Index.ets`-এ bookmark চাপলে `urlInput` ও title বদলায়; প্রকৃত `WebView`/`WebEngine` load callback নেই এমন অংশকে বাস্তব browse হিসেবে গণ্য কোরো না। Static “SSL/TLS 1.3” / “privacy shield active” label সরিয়ে engine `get_tls_info`/TLS API থেকে status নাও; engine তা দিতে না পারলে lock icon বাদ দাও। Placeholder marketing text সরিয়ে actual webpage surface বসাও।

---

## ১০. Phase 7 — প্রকৃত Terminal, PTY, shell ও diagnostics

Terminal app শুধু command names-এর তালিকা বা fake `ps/services/net` output নয়। প্রকৃত terminal-এর জন্য:

- PTY master/slave;
- terminal emulator parser (ANSI/VT sequences, scrolling, resize, UTF-8, mouse/focus);
- per-user unprivileged process context;
- explicit environment (`HOME`, `PATH`, `TERM`, locale), safe working directory;
- signal forwarding, resize (`TIOCSWINSZ`), foreground process group, child exit collection;
- logs/diagnostic command-এ permission filtering;
- no arbitrary root shell in daily user mode;
- no command concatenation into `/bin/sh -c` from untrusted UI strings;
- capability-gated maintenance operations ও confirmation;
- real exit code/stderr; unavailable binary হলে “not installed”;
- process memory/CPU/FD limits and kill cleanup।

Rust-এ `portable-pty` বা system `openpty(3)`/`forkpty(3)`-ভিত্তিক vetted PTY layer ব্যবহার করে terminal UI-এর সাথে সংযোগ করো। Exact crate version Cargo.lock-এ pin করে current API অনুযায়ী integrate করতে হবে। Illustrative lifecycle:

```rust
// Illustrative API shape; pin a tested portable-pty release and adapt to its exact API.
use portable_pty::{native_pty_system, CommandBuilder, PtySize, PtySystem};

pub fn spawn_user_terminal(home: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pty = native_pty_system();
    let pair = pty.openpty(PtySize {
        rows: 28,
        cols: 90,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    // Must already have dropped root privileges/set UID/GID and sandbox policy
    // before spawning this command; this sample deliberately does not do that setup.
    let mut command = CommandBuilder::new("/usr/bin/sh");
    command.cwd(home);
    command.env("HOME", home);
    command.env("PATH", "/usr/bin:/bin");
    command.env("TERM", "xterm-256color");
    command.env("LANG", "bn_IN.UTF-8");

    let child = pair.slave.spawn_command(command)?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;

    // Integrate these handles with the UI event loop asynchronously:
    // read output -> terminal emulator; key bytes -> writer; resize -> PTY ioctl;
    // poll child exit -> render genuine exit status. Never fake command output.
    let _ = (&mut reader, &mut writer, child);
    Ok(())
}
```

এই code process-spawn-এর lifecycle বোঝায়; production terminal emulator নয়। Real implementation-এ blocking read UI thread-এ চালাবে না, child wait/reap করতে হবে, PTY window resize এবং Ctrl-C/foreground group সামলাতে হবে। Root privilege drop ও seccomp/namespaces ভুলে এই function production-এ ব্যবহার করা যাবে না।

### Terminal acceptance tests

- `pwd`, `ls`, `cat`, `ps`, `ip`, `dmesg`—যে command সত্যিই image-এ থাকবে, তার output live system থেকে;
- nonzero exit status, command not found, Ctrl-C, Ctrl-D, long output, resize, Unicode/Bengali, scrollback, process crash;
- user থেকে root escalation চেষ্টা, `/proc`/`/sys`/`/dev` সীমা, path traversal, environment injection;
- app close হলে child process reaped; abandoned PTY leak নয়;
- release image-এ scripted fake command provider নেই।

---

## ১১. Phase 8 — Android Hosted JNI bridge (S25) সম্পূর্ণ করা

### ১১.১ Track B-এর lifecycle

```text
Android permission state
   ↓
MainActivity/Surface lifecycle
   ↓ explicit NativeBridge.load + ABI/protocol handshake
Native Rust library loaded
   ↓ bounded bidirectional command/event protocol
Display/input/battery/network/audio/camera/sensors/telephony providers
   ↓ permission-checked Onuron HAL consumers
UI diagnostics reflects actual API result and lifecycle
```

Android platform permission/role restrictions-এর বাইরে bridge কোনো access পাবে না। Hosted mode-এ “native Onuron OS has direct hardware access” বলা যাবে না; Android Host API-র মাধ্যমে যেটুকু অনুমতি পাওয়া যায় সেটুকুই।

### ১১.২ Native library fail-open বন্ধ করো

বর্তমান `NativeBridge.java` library load fail হলে “pure hosted mobile mode”-এ চলে যায়। এতে UI চালানো সম্ভব হতে পারে, কিন্তু JNI real integration কাজ করছে এমন দাবি করা যাবে না। দুইটি explicit startup state রাখো:

- `HostedUiOnly`: native library unavailable; actual hardware bridge features disabled; diagnostic warning visible.
- `HostedNativeReady`: native library loaded, protocol version matches, JNI symbols exist, handshake response valid, required channels available.

Mandatory bridge release build-এ JNI loading failure হলে feature test fail করবে। Incompatible protocol-এ `0` version ফিরিয়ে silent continue নয়।

### ১১.৩ Wire protocol সংশোধন

`android-host/src/bridge.rs`-এর enums-এ `String` action/state/connection type খুব সহজে typo/incompatible value আনতে পারে। Versioned typed values এবং request ID ব্যবহার করো:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum BridgeRequest {
    Handshake { protocol_version: u32, runtime_version: String },
    GetCapabilities { request_id: u64 },
    GetBattery { request_id: u64 },
    SetBrightness { request_id: u64, percent: u8 },
    CapturePhoto { request_id: u64, camera_id: u32 },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeResponse {
    pub protocol_version: u32,
    pub request_id: u64,
    pub ok: bool,
    pub error_code: Option<String>,
    pub message: Option<String>,
    pub payload: Option<serde_json::Value>,
}
```

এটি contract-এর উদাহরণ; project-এর `nilprotocol` framing/serde version অনুযায়ী integrate করো। Message size bounded, queue bounded, timeout enforced, response IDs unique, stale request ignored/logged, queue saturation visible। Sensitive SMS/call/location contents debug log-এ যাবে না।

### ১১.৪ Host Android code

`android-host/app/src/main/java/org/onuron/mobile/MainActivity.java`, `NativeBridge.java`, `OnuronBridgeService.java`-এ:

- manifest permissions, runtime permissions ও Android 12+ restrictions explicit;
- Activity/Service lifecycle-এ surface recreated হলে stale native Surface reuse নয়;
- JNI method signatures generated/checked (JNI symbol test);
- native crash/unavailable state-এর diagnostic screen;
- `ConnectivityManager` callback, battery receiver, Camera2 `ImageReader`, AudioTrack/AudioRecord lifecycle, sensor registration—সব বন্ধ/মুক্ত করা হবে;
- Telephony API permission/role-নির্ভর; সব consumer-এ private phone/SMS data expose করা যাবে না;
- commands accept করার আগে allowlist, field range, origin permission ও user-consent check;
- command completion/permission denial correlated response হিসেবে ফিরবে;
- camera test JPEG, default signal, fixed screen refresh values শুধুমাত্র test mode; Hosted device-এ capability queried value ছাড়া hard-coded fallback নয়;
- bridge process crash হলে reconnection, in-flight request cancellation ও duplicate call/SMS prevention।

### ১১.৫ JNI bridge test gate

- APK install/start/stop/restart;
- `libandroid_host.so` correctly packaged in `arm64-v8a`;
- `getProtocolVersion()` returns exact expected version;
- bad ABI/missing symbol → explicit failed state;
- touch event goes host→Rust→NilHAL; app click/event in reverse path;
- actual resized surface dimensions/refresh rate;
- battery/network event changes only after Android API event;
- camera returns actual frame or explicit error; test JPEG never in release;
- real audio loopback/AudioRecord permission test;
- permission denied path and revocation;
- queue saturation/backpressure, malformed JSON/oversized event/fuzz;
- Android OS pause/resume/screen-off/on/rotation/low-memory restore;
- no telemetry/secret logging, lifecycle cleanup.

---

## ১২. Phase 9 — Native Onuron-এ Android APK compatibility

এই কাজকে S25 Hosted Runtime-এর সাথে এক করে ফেলবে না। Bare-metal Linux-এ Android runtime চালাতে প্রকৃত Android rootfs/system image, `init`, ART, Binder, package manager, framework services, graphics, audio/input এবং hardware integration দরকার। `android/agent/src/main.rs`-এর `am start` call শুধুমাত্র চলমান Android container-এর ভেতরে activity launch করতে পারে; এটি container start, Binder, SurfaceFlinger বা APK installation-এর সম্পূর্ণ ব্যবস্থা নয়।

### Implementation order

**A. Feasibility spike**

1. chosen target kernel-এ Binder driver/devices এবং `memfd`/প্রয়োজনীয় shared memory support আছে কি না যাচাই;
2. `CONFIG_ANDROID_BINDER_IPC`/device support এবং SELinux rules পরীক্ষা;
3. memory, disk, GPU/GBM/EGL, Wayland, cgroups ও namespaces compatibility পরীক্ষা;
4. trusted, licensed Android system image/build source ও architecture (arm64) নির্ধারণ;
5. upstream-supported LXC/container manager/Waydroid integration prototype; source, release cadence ও security update process pin;
6. target hardware-এ graphics/codec/vendor HAL support না থাকলে প্রথম milestone-এ feature explicitly unsupported রাখো।

**B. `nilandroidd` as real lifecycle manager**

- start/stop/status/restart Android container;
- verify system image checksum/signature and supported API/ABI;
- dedicated UID, mount/pid/ipc/user/network namespaces;
- read-only system image + writable bounded Android data volume;
- Binder device/pass-through according to kernel and architecture;
- cgroups limits; per-container SELinux domain; no host-root shell;
- health check must observe real Android services (e.g. system readiness), not “container state” placeholder;
- container crash → safe stop/diagnostics, no infinite restart storm;
- package install/uninstall/launch from trusted CLI/API and actual `pm`/Android package manager response;
- host/native package identity remains separate from Android package identity.

**C. Graphics/Input/Audio Integration**

- determine Android rendering path and compatible buffer transport;
- synchronize buffer format/modifier/fence correctly; never assume `ARGB8888` for every producer;
- SurfaceFlinger composition/virtual display and host compositor window mapping;
- input seat, coordinate transform/rotation, keyboard IME and pointer capture;
- audio stream routing, focus, mic permission, suspend/reconnect;
- clipboard/URI/file sharing narrow allowlist; no untrusted Android app access to host `/data`;
- verify latency, frame drops, app crash and memory lifecycle on target device.

**D. `nilagent` protocol hardening**

`android/agent`'s newline-delimited JSON server may remain a container-side helper, but add message version, request id, payload cap, authenticated peer policy, per-command allowlist and timeouts. `GET_INSTALLED` must ask real PackageManager/`pm list packages`; `START_ACTIVITY` must validate package/activity and capture command result; `PING` returns version/health. Do not parse arbitrary shell text. If an Android command is unsupported, return a typed error, not a mock response.

### Android compatibility Definition of Done

- target kernel supports required Android IPC/shared-memory facilities;
- Android init reaches a documented ready state from a clean start;
- package install→list→launch→draw actual surface→input→audio→stop works;
- permissions and per-app Android sandbox behave as expected;
- container cannot read/write native `/system` or another user’s data;
- no simulated status row/blank placeholder advertised as available apps;
- image updates/signatures/SBOM/license list/security patch policy tested;
- if a device cannot support the required graphics or Binder route, do not claim APK compatibility for that device.

---

## ১৩. Phase 10 — বাস্তব hardware bring-up: OnePlus 6T `fajita`

### ১৩.১ কেন ফোনটিকে আলাদা engineering target হতে হবে

রিপোর `docs/adr/0007-first-native-reference-phone.md` OnePlus 6T-কে candidate হিসেবে নির্বাচন করে; একই নথি Stage Gate A/B-এর আগে acquisition/flashing না করার কথা বলে। S25 Hosted Runtime একটি আলাদা track। OnePlus 6T-এর open mobile Linux community work useful reference, কিন্তু upstream page-এ feature status-ও অসম: camera/audio/calls/GPS-এর ক্ষেত্রে partial support থাকতে পারে। তার মানে “mainline kernel আছে” = “Onuron-এর সব hardware চলে” নয়। [Device page](https://wiki.postmarketos.org/wiki/OnePlus_6T_%28oneplus-fajita%29).

### ১৩.২ Hardware bring-up ladder

একই সময়ে camera, modem, browser, Android runtime সব চালু করার চেষ্টা করবে না। এই ক্রম অনুসরণ করো:

1. **UART/early console or reliable recovery log:** kernel/init crash হলে logs পাওয়া নিশ্চিত।
2. **Kernel only:** exact device kernel + exact device DTB/DTBO + matching firmware দিয়ে boot; Onuron userspace এখনো নয়।
3. **Initramfs shell:** rootfs mount, `/init`, console, modules, block devices; screen UI নয়।
4. **Persistent data:** partition discover + ext4 + fscrypt/key policy + two-boot persistence.
5. **Display:** DRM/KMS modeset; correct panel, brightness, suspend/resume.
6. **Touch/input:** real `/dev/input/event*`, coordinate mapping, rotation and gestures.
7. **Power/thermal:** battery, charging state, thermal throttling, suspend, wake.
8. **Wi-Fi/Bluetooth:** firmware, regulatory domain, authentication, DNS; no fixed status values.
9. **Audio:** speaker/earpiece/headphones/mic routing; correct UCM/calibration where necessary.
10. **Camera:** correct sensor/ISP pipeline, capture/preview/torch. V4L2 node found alone is not end-to-end camera support.
11. **Modem/data/SMS/calls:** QMI/MBIM/vendor interface and telephony stack; VoLTE/IMS is a separate support gate. Emergency-call claims need appropriate regulatory and physical validation.
12. **Location/sensors/NFC/USB/haptics:** capability-by-capability testing.
13. **Thermal/battery 24 h soak:** idle and interactive; verify no crash, memory/FD leaks or overheating.
14. **Update/recovery:** image signature, failed boot counter, recovery entry, rollback.

### ১৩.৩ Do not flash the current image

At the time this audit was prepared:

- `targets/oneplus-fajita/target.toml` sets `flashing_allowed = false`;
- physical hardware validation is not established in the reference-board docs;
- the generic QEMU Alpine `vmlinuz-virt` is not a phone kernel;
- the generated minimal FDT in `build/mkiso-fajita.py` is not a complete OnePlus device tree;
- the build script can package artifacts even if some expected binary files are absent.

Therefore no current artifact should be assumed safe for `fastboot flash`. Keep `flashing_allowed=false`. Only after separate maintainers sign off the exact phone model/revision, bootloader state, kernel commit, DTB/DTBO, boot image layout, partition backups, recovery path and verified `fastboot boot`/equivalent test should permanent flashing be considered. Bootloader unlock can wipe device data; backup first. A force flag must never override an unvalidated target profile.

### ১৩.৪ Hardware evidence report

For each device test, store redacted logs under `docs/evidence/oneplus-fajita/<test-id>/`:

```text
metadata.json          # exact device variant, kernel SHA, DTB SHA, OS commit, test date
boot-console.txt       # kernel + init logs, serials/phone numbers redacted
hardware-inventory.txt # sysfs/driver inventory
input-test.json        # event devices + touch coordinates
power-test.json        # battery, suspend/wake, temperature
network-test.json      # Wi-Fi/Bluetooth connect/DNS/reconnect
storage-test.json      # fscrypt state + write/reboot/read hash
ui-test.json           # display modes, frame drops, rotation
pass-fail.json         # test ID -> pass/fail/not-run and measured values
```

No IMEI, phone number, precise location, account tokens, Wi-Fi password, user content or encryption keys in evidence. Each result includes exact command, exit code, timestamp and build SHA.

---

## ১৪. Phase 11 — Security, Verified Boot, update, recovery ও privacy

### ১৪.১ Security is not a settings page

- Kernel config and LSM support are checked in the running kernel;
- SELinux policy compiles for the correct policy version and runs enforcing where target intends it;
- app services use distinct identities and least privilege;
- public keys are trust roots; private signing keys never ship in rootfs;
- fscrypt uses kernel-supported configuration and key lifecycle; `nilkeyd` must actually be wired into unlock/lock/recovery;
- seccomp filters are tested against real app syscalls; no “allow ~110 syscalls” number alone as proof;
- IPC access uses SELinux + Unix permissions + peer credential checks;
- logs redact messages, numbers, location, passwords and tokens;
- TLS, update metadata and packages verify real cryptographic signatures;
- threat model addresses malicious package, compromised app, container escape, boot image tampering, rollback, lost key, corrupted disk and update power loss.

### ১৪.২ Verified boot terminology

`build/mkvbmeta.py` producing a signed descriptor is not enough to claim Android Verified Boot. As ADR-0006 correctly states, the device bootloader must enforce OEM/public-key trust and actually verify the relevant metadata before loading the kernel. Until tested on a target with enforceable boot chain, call it an **integrity descriptor prototype**, not verified boot.

### ১৪.৩ Transactional updater

- signed manifest and metadata, version/target/architecture match;
- anti-rollback version/sequence policy;
- write inactive slot/image then `fsync` and verify;
- mark pending slot only after complete image validation;
- boot health confirmation before committing new slot;
- failed boot count and automatic rollback;
- power-cut test after every write/commit state;
- network/TLS errors do not leave system half-installed;
- preserve user data migration/versioning and explicit backup/recovery.

A/B updates depend on the specific device partition scheme. Do not assume all phones expose compatible `boot_a/system_a` partitions or that an unlocked bootloader accepts custom vbmeta. Exact partition map must come from the target device and be checked against the selected device’s documentation.

---

## ১৫. CI: “no stub in release” বাধ্যতামূলক gate

`/.github/workflows/linux-qemu.yml`, `security-audit.yml`, `code-quality.yml` এবং simulator workflow-কে আলাদা job-এ ভাগ করো। Simulator test pass কখনো native QEMU boot বা hardware integration test-এর বদলি হবে না।

Required gates:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings` (current workspace/toolchain অনুযায়ী staged rollout)
3. `cargo test --workspace`
4. Python build-script unit tests
5. target-specific build with pinned target triple
6. strict artifact/rootfs verification
7. `build/gen-maturity.py --check`
8. QEMU boot smoke test: PID 1, service readiness, compositor/headless target readiness
9. two-boot persistent-storage test
10. recovery/corrupt-disk/no-silent-tmpfs test
11. package tamper/signature/archive traversal tests
12. sandbox attack tests
13. browser integration tests (engine-specific; CI may use test server)
14. terminal PTY/child lifecycle tests
15. Android Hosted bridge APK + native ABI/protocol tests, separately
16. SBOM/license/security audit
17. final build manifest generated from actual outputs/tests

### Simulated backend guard

CI should fail a release build if:

- `FakeHAL`/`new_simulated()` is linked into production binary;
- `[SIMULATED]` badge registry remains enabled on production UI;
- static fake addresses/carrier/battery/display refresh values reach the release profile;
- camera fixture/test-pattern bytes are returned from production camera API;
- terminal command results are generated from screen state, not child process or real system API;
- browser secure/TLS status is set by UI constant;
- Android container readiness is asserted without real container/Android service health;
- a required binary/service is absent from rootfs;
- test silently skips due missing tool/permissions without marking target as unverified.

String grep can help but not enough; use type-level feature separation, integration tests, build profiles and runtime capability provenance.

---

## ১৬. Release gate — কখন বলা যাবে “এই ফোনে Onuron boot হয়”

Release label-এ exact target লিখবে, যেমন `OnuronOS <version> — QEMU ARM64 prototype` অথবা `OnuronOS <version> — OnePlus 6T hardware experimental`. Generic “Android-compatible phone OS” নয়।

### QEMU release-এর জন্য

- clean build;
- boot/recovery/persistence tests;
- mandatory services ready;
- real app install/launch;
- no fake providers in release build except explicitly named simulator artifact;
- test report + checksums + manifest;
- known limitations documented।

### Real phone release-এর জন্য

QEMU gate ছাড়াও:

- exact OnePlus 6T variant/bootloader/partition layout matched;
- kernel + DTB/DTBO + firmware source and licenses pinned;
- bootloader accepted tested image with recovery procedure;
- first boot → setup → lock/unlock → launcher → real touch/display;
- storage persists after two reboots; encryption state confirmed;
- basic networking, audio, power, thermal, suspend/resume validated;
- all unimplemented phone features show unsupported, not fabricated success;
- core services survive soak test; no critical kernel/userspace crash;
- update integrity and recovery verified;
- hardware evidence with build hash and test date;
- flashing instructions reviewed and `flashing_allowed` gate consciously opened only for the validated target.

### “সব অ্যাপ চলে” বলার জন্য

প্রতিটি app-এ acceptance test তালিকা পাস করতে হবে। Native NilLang app works মানেই arbitrary Android APK works নয়; Android container start মানেই GPU/camera/Google Play compatibility নয়; camera device node exists মানেই photo pipeline works নয়। Claim-কে exact test matrix-এর সঙ্গে মিলিয়ে লিখবে।

---

## ১৭. AI/কোড-এজেন্টকে কাজ দেওয়ার নিয়ম

একসঙ্গে “পুরো OS production-ready করো” prompt দিলে agent অনেক UI/manifest বানাতে পারে, কিন্তু end-to-end proof ছাড়া নতুন ভুল তৈরি হবে। **প্রতি সময়ে একটি subsystem, একটি branch, একটি acceptance test set।**

প্রতি implementation prompt-এ এই contract দাও:

```text
Read AGENTS.md, docs/architecture.md, docs/adr/* relevant to this subsystem,
docs/maturity.toml, and the implementation plus tests before editing.

Goal: implement one real end-to-end subsystem; do not add fake success paths.
Constraints:
- Preserve native Onuron vs Android-host vs Android-container boundaries.
- Reuse existing project abstractions and public contracts.
- No TODO, stub, mock or simulated backend in release path.
- Test doubles are allowed only under #[cfg(test)] or a separately named simulator target.
- Unsupported hardware must return a typed error and must be visible in diagnostics.
- Add unit, integration, failure-path, and (where possible) QEMU/hardware tests.
- Do not claim hardware support unless the target-specific test was actually run.
- Do not change flashing_allowed, fake firmware/DTB, signing policy, SELinux,
  namespaces, or seccomp to make tests pass.
- Do not mark any test pass unless you ran it and can provide exact command/output.
- Update docs/maturity.toml, evidence paths, ADR if needed, and README via generator.

Deliver:
1. concise design decision;
2. files changed and why;
3. full patch;
4. tests run and exact results;
5. tests not run + reason;
6. remaining risks and hardware dependencies.
```

### Suggested implementation order for separate coding tasks

1. **Build integrity:** strict artifact verifier, staged output, truthful manifests.
2. **Boot/rootfs:** required binary map, no-silent-tmpfs, recovery/no-data gate.
3. **IPC/health:** typed errors, service readiness graph, queue backpressure.
4. **HAL:** Linux DRM/evdev/sysfs providers, fake backend test-only.
5. **Compositor:** real wlroots or Smithay path; retire competing fake/sleep implementation.
6. **Native runtime:** install/launch/uninstall `.nilax` vertical slice with per-UID sandbox.
7. **Terminal:** PTY + real process + output decoder and sandbox.
8. **Browser:** select engine, add adapter, navigation/TLS/download integration tests.
9. **Apps:** service-backed vertical slice one app at a time.
10. **Android Hosted:** JNI ABI/protocol/lifecycle/real sensor/display/audio/camera tests.
11. **Android compatibility container:** kernel Binder feasibility, real Android system startup, app launch and surfaces.
12. **OnePlus hardware:** kernel/DTB/firmware bring-up ladder; no permanent flash before staged evidence.
13. **Security/update:** signed package, fscrypt, SELinux enforcement, update rollback/power-cut tests.

### PR policy

- One subsystem and a manageable diff per PR.
- CI must fail on new warning, mandatory test skip or false maturity claim (temporary staged exceptions documented).
- Update docs with real limitations; do not rewrite maturity table manually.
- Include at least one negative/failure-path test per new privileged operation.
- Avoid broad code generation that duplicates existing APIs.
- Require maintainer review for kernel, boot image, signing, SELinux, sandbox and target-flashing changes.

---

## ১৮. প্রথম ৩০ দিনের বাস্তব কাজ

এটি ৩০ দিনে পূর্ণ OS শেষ করার প্রতিশ্রুতি নয়। লক্ষ্য হলো project-এ measurable engineering discipline স্থাপন করা।

### সপ্তাহ ১: Truthful clean build

- clean Linux runner-এ workspace build/test repeat;
- current CI workflow-এর exact results record;
- strict artifact verifier;
- missing binary হলে build fail;
- manifest tests থেকে generate;
- target config conflict/duplicate docs reconcile: QEMU, S25 hosted, OnePlus native roles একরকমভাবে লেখা।

### সপ্তাহ ২: Boot/data/recovery

- `/data` silent tmpfs fallback production profile-এ বন্ধ;
- corrupt/unavailable data → explicit recovery;
- two-boot persistence, failed-boot counter, recovery test;
- services.toml vs packaged binaries consistency test;
- PID 1/service readiness সত্যভাবে verify।

### সপ্তাহ ৩: একটি end-to-end native app

- package `.nilax` sign/verify/install/launch/uninstall;
- app-এর real file/database persistence;
- UID/permission/sandbox enforcement;
- QEMU boot থেকে app launch পর্যন্ত test artifact।

### সপ্তাহ ৪: Terminal/compositor/browser feasibility decisions

- `nilshell.c`-এর বাস্তব compositor status মাপা; wlroots/Smithay ADR;
- terminal-এর real PTY spike;
- browser engine (WebKitGTK/WPE vs অন্য পথ) prototype build; শুধু static UI নয়;
- Android Hosted JNI handshake test;
- OnePlus target-এর kernel/DTB/firmware source-of-truth ও hardware test plan; flashing এখনও disabled।

৩০ দিনের শেষে measurable output হবে: “সব কাজ শেষ” নয়; বরং clean build, repeatable boot, persistent data, package install/launch test, truthful maturity, browser/compositor/terminal integration decisions এবং hardware bring-up-এর verified prerequisites।

---

## ১৯. Reference links

### OnuronOS repository

- [AGENTS.md — architectural invariants and maturity rules](https://github.com/joysriramsarkar/onuronOS/blob/main/AGENTS.md)
- [README — current maturity table](https://github.com/joysriramsarkar/onuronOS/blob/main/README.md)
- [PLAN.md — current gaps and next steps](https://github.com/joysriramsarkar/onuronOS/blob/main/PLAN.md)
- [Architecture](https://github.com/joysriramsarkar/onuronOS/blob/main/docs/architecture.md)
- [Boot process](https://github.com/joysriramsarkar/onuronOS/blob/main/docs/boot-process.md)
- [Reference board status](https://github.com/joysriramsarkar/onuronOS/blob/main/docs/reference-board.md)
- [Hardware support](https://github.com/joysriramsarkar/onuronOS/blob/main/docs/hardware-support.md)
- [Android compatibility plan](https://github.com/joysriramsarkar/onuronOS/blob/main/docs/android-compatibility.md)
- [S25 Hosted Runtime](https://github.com/joysriramsarkar/onuronOS/blob/main/docs/s25-hosted-runtime.md)
- [Language/runtime contract](https://github.com/joysriramsarkar/onuronOS/blob/main/docs/LANGUAGE_AND_RUNTIME_CONTRACT.md)
- [NilInit](https://github.com/joysriramsarkar/onuronOS/blob/main/nilinit/src/main.rs)
- [Build script](https://github.com/joysriramsarkar/onuronOS/blob/main/build/build.sh)
- [Build Python orchestrator](https://github.com/joysriramsarkar/onuronOS/blob/main/build/build.py)
- [Target registry](https://github.com/joysriramsarkar/onuronOS/blob/main/build/target_registry.py)
- [OnePlus 6T target config](https://github.com/joysriramsarkar/onuronOS/blob/main/targets/oneplus-fajita/target.toml)
- [Android JNI bridge](https://github.com/joysriramsarkar/onuronOS/blob/main/android-host/src/jni_bridge.rs)
- [Native browser code](https://github.com/joysriramsarkar/onuronOS/blob/main/apps/browser/src/main.rs)
- [Compositor C entry point](https://github.com/joysriramsarkar/onuronOS/blob/main/shell/nilshell.c)

### Upstream references

- [Linux DRM/KMS documentation](https://docs.kernel.org/gpu/drm-kms.html)
- [Waydroid documentation](https://docs.waydro.id/)
- [Waydroid kernel requirements/debugging](https://docs.waydro.id/debugging/getting-essential-information)
- [OnePlus 6T mainline Linux status](https://wiki.postmarketos.org/wiki/OnePlus_6T_%28oneplus-fajita%29)
- [GeckoView documentation](https://firefox-source-docs.mozilla.org/mobile/android/geckoview/index.html)
- [WebKitGTK 6 migration](https://github.com/WebKit/WebKit/blob/main/Source/WebKit/gtk/migrating-to-webkitgtk-6.0.md)
- [Rust webkit6 API](https://docs.rs/webkit6/latest/webkit6/)
- [Servo Book](https://doc.servo.org/)

---

## শেষ কথা

অনুরণকে সত্যিকারের মোবাইল OS করতে হলে প্রথমে “সব অ্যাপের স্ক্রিন আছে” এই মানদণ্ড থেকে “প্রতিটি critical path বাস্তব backend-এ কাজ করে এবং ব্যর্থ হলে সত্যি error দেয়” এই মানদণ্ডে যেতে হবে। সবচেয়ে জরুরি তিনটি কাজ হলো:

1. **Build-এ fail-closed ও সত্যভিত্তিক artifact/manifest।**
2. **QEMU-তে boot → persistent storage → service readiness → signed native app install/launch end-to-end test।**
3. **একটি নির্দিষ্ট ফোনের real kernel/DTB/firmware/driver path-কে evidence-সহ প্রমাণ করা—তার আগে flash নয়।**

Browser engine, terminal PTY ও Android container তার পরে বাস্তব backend হিসেবে যুক্ত করতে হবে। কোনো ফিচার সম্পূর্ণ না হলে সেটিকে disable/unsupported দেখানো প্রকৃত OS engineering; fake success দেখানো নয়।
