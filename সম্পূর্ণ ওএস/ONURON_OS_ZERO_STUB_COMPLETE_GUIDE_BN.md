# অনুরণ ওএস — বুট থেকে প্রতিটি অ্যাপ পর্যন্ত জিরো-স্টাব সম্পূর্ণ গাইড (কোড সহ)

> এই ফাইলটি এক জায়গায় পুরো ওএস-এর **বাস্তব ইমপ্লিমেন্টেশন নির্দেশনা** দেয়: বুট চেইন → কার্নেল/`nilinit` → IPC → কম্পোজিটর → টার্মিনাল → ব্রাউজার → প্রতিটি নেটিভ অ্যাপ → অ্যান্ড্রয়েড ব্রিজ → ফ্ল্যাশিং → যাচাই।
>
> **মূল পার্থক্য:** এই ডকে প্রতিটি দাবির সাথে আসল ফাইল-প্রমাণ (পাথ + লাইন) আছে, আর "স্টাব সরানোর" জন্য **চালানো-যোগ্য কোড** দেওয়া হয়েছে। যেখানে এখনও কাজ বাকি, সেখানে সৎভাবে "কী নেই" বলা হয়েছে — বানিয়ে "সম্পূর্ণ" বলা হয়নি।

সংস্করণ: ৫.০ (truth-first) · তারিখ: ২০২৬ · লাইসেন্স: AGPLv3

---

## সূচিপত্র

0. [এই ফাইল কী, আর কী নয় (সৎ ঘোষণা)](#০-এই-ফাইল-কী-আর-কী-নয়)
1. [সত্য অডিট — রেপোতে আসলে কী আছে (প্রমাণ সহ)](#১-সত্য-অডিট--রেপোতে-আসলে-কী-আছে)
2. [স্থির আর্কিটেকচার স্পাইন (ফ্রিজ)](#২-স্থির-আর্কিটেকচার-স্পাইন)
3. [Phase 0 — "No-Stub" বিল্ড গেট ও ম্যাটুরিটি সততা](#৩-phase-0--no-stub-বিল্ড-গেট)
4. [Phase 1 — বুট চেইন: {kernel → initramfs → nilinit} (বাস্তব + হার্ডেনিং)](#৪-phase-1--বুট-চেইন)
5. [Phase 2 — IPC কন্ট্রাক্ট ও সার্ভিস টপোলজি (বাগ ফিক্স সহ)](#৫-phase-2--ipc-কন্ট্রাক্ট-ও-সার্ভিস-টপোলজি)
6. [Phase 3 — আসল DRM/KMS কম্পোজিটর (স্টাব vkctx/atlas/set_crtc সরানো)](#৬-phase-3--আসল-drmkms-কম্পোজিটর)
7. [Phase 4 — inpald: আসল evdev ইনপুট ইঞ্জিন](#৭-phase-4--আসল-evdev-ইনপুট-ইঞ্জিন)
8. [Phase 5 — আসল টার্মিনাল: PTY + VT পার্সার + রিয়েল কমান্ড (নতুন `nilterminal`)](#৮-phase-5--আসল-টার্মিনাল)
9. [Phase 6 — আসল ব্রাউজার: WPE/WebKit (নিজে ইঞ্জিন লেখা যাবে না)](#৯-phase-6--আসল-ব্রাউজার)
10. [Phase 7 — প্রতিটি নেটিভ অ্যাপ সত্যিকারের করা](#১০-phase-7--প্রতিটি-নেটিভ-অ্যাপ)
11. [Phase 8 — অ্যান্ড্রয়েড ব্রিজ: আসল কন্টেইনার + Binder (স্টাব সরানো)](#১১-phase-8--অ্যান্ড্রয়েড-ব্রিজ)
12. [Phase 9 — স্ট্যাটাস বার ও সিস্টেম ইন্ডিকেটর (রিয়েল nilhal)](#১২-phase-9--স্ট্যাটাস-বার)
13. [Phase 10 — ফ্ল্যাশিং: QEMU, OnePlus 6 (fajita), Galaxy S25](#১৩-phase-10--ফ্ল্যাশিং)
14. [Phase 11 — যাচাই (Definition of Done) ম্যাট্রিক্স](#১৪-phase-11--যাচাই-ম্যাট্রিক্স)
15. [কাজের ক্রম (Task Ledger) ও PR নিয়ম](#১৫-কাজের-ক্রম)

---

## ০. এই ফাইল কী, আর কী নয়

**এটি কী:** এটা একটা *ইমপ্লিমেন্টেশন গাইড + রেফারেন্স কোড*। এখানে যেসব কোড দেওয়া হয়েছে সেগুলো সরাসরি `cargo build`/`gcc` দিয়ে বিল্ড করার মতো করে লেখা (libc/nix/ioctl ভিত্তিক), যাতে স্টাব ফাইলগুলো রিপ্লেস করা যায়।

**এটি কী নয়:** এটা "সব রেডি হয়ে গেছে" — এমন কোনো দাবি নয়। নিচের ১ নম্বর অডিট দেখুন: বর্তমানে `nilinit`, `nilprotocol`, `nilpkg`, `nilrt` sandbox, `nilkeyd`, `softbus`— এসব **আসল**। কিন্তু কম্পোজিটর, টার্মিনাল, ব্রাউজার, অ্যান্ড্রয়েড কন্টেইনার, স্ট্যাটাস বার — এগুলোতে **স্টাব/সিমুলেশন এখনও আছে**। এই ডক সেগুলো সরানোর পথটা কোড দিয়ে দেখায়।

**কঠোর নিয়ম (AGENTS.md ধারা ৪ মেনে):** কোনো ফিচারকে `IMPLEMENTED` বলা যাবে কেবল তখনই, যখন কোড + টেস্ট + বিল্ড পাথ তিনটাই মেলে এবং একটা reproducible প্রমাণ (boot log / dmesg / ioctl সফলতার ট্রেস) আছে। ডিরেক্টরি থাকা = ফিচার প্রোডাকশনে আছে — এটা ধরে নেওয়া নিষেধ।

---

## ১. সত্য অডিট — রেপোতে আসলে কী আছে

নিচের টেবিলটা ২০২৬-এর কোড পড়া থেকে তৈরি। "অবস্থা" কলামে সত্য — কোড পড়ে যা পাওয়া গেছে।

| Subsystem | ফাইল | আসলে অবস্থা | প্রমাণ / নোট |
|---|---|---|---|
| PID 1, mounts, cgroups, SELinux, fscrypt | `nilinit/src/{main.rs,supervisor.rs,activate.rs,recovery.rs,fscrypt.rs}` | 🟢 আসল | `mount_early_fs` (proc/sys/dev/run/tmp/cgroup2/selinuxfs), `setup_cgroups` cgroup2 subtree_control, `load_selinux` policy.33 → `/sys/fs/selinux/load` + enforce=1, `load_fscrypt_key` → keyd.sock → `FS_IOC_ADD_ENCRYPTION_KEY` |
| Supervisión + socket activation | `nilinit/src/supervisor.rs`, `activate.rs` | 🟢 আসল | `LISTEN_FDS=1`, `LISTEN_FDNAMES`, `dup2(fd,3)`, exponential backoff 250ms→30s, 60s healthy reset |
| সার্ভিস টপোলজি | `etc/nilos/services.toml` | 🟢 আসল (TOML, ২০ সার্ভিস) | `name/exec/restart/socket_activation` |
| Framed IPC | `protocol/src/lib.rs` | 🟢 আসল | 24-বাইট হেডার: `ONUR` + len(u32 BE) + ver(u16) + type(u16) + reqid(u64) + flags(u32); `MAX_PAYLOAD_SIZE = 1 MiB` |
| Unix socket auth | `runtime/nilsd/src/auth.rs`, `etc/nilos/ipc-policy.toml` | 🟢 আসল | `SO_PEERCRED`, fail-closed, unknown service = deny |
| .nilax প্যাকেজ + Ed25519 | `pkg/nilpkg/src/lib.rs` | 🟢 আসল | `Manifest` (JSON), `sign_manifest` → canonical bytes → Ed25519; `verify_manifest` SHA-256 + `verify_strict`; upgrade + journal + rollback |
| Sandbox (namespace + seccomp + permbroker) | `runtime/nilrt/src/{sandbox.rs,seccomp.rs,permbroker.rs}` | 🟢 আসল | `unshare(CLONE_NEWPID|NEWNS|NEWIPC|NEWUTS[|NEWNET])`, `PR_SET_SECCOMP` BPF allowlist, 7-day auto-revoke |
| SoftBus mesh | `softbus/src/{mdns.rs,quic.rs,ctl.rs}` | 🟢 আসল (লজিক) | mDNS `_nilbus._udp.local.`, Quinn QUIC/TLS1.3, port 7979 |
| **DRM/KMS কম্পোজিটর** | `runtime/nilui-gpu/src/{drm.rs,present.rs,vkctx.rs,atlas.rs}` | 🔴 **স্টাব** | `set_crtc()` শুধু `fb_id==0` চেক করে `Ok(())` ফেরায় — `drmModeSetCrtc` কখনো কল হয় না; `page_flip()` একই; `vkctx.rs::new()` শুধু print; `atlas.rs` শুধু print |
| **C কম্পোজিটর** | `shell/nilshell.c` | 🔴 স্টাব | `while(server.running){ sleep(60); }` — "120Hz event loop" আসলে ঘুম |
| **সারফেস/উইন্ডো ম্যানেজার** | (কেউ নেই) | 🔴 অনুপস্থিত | wlroots/Wayland কোড কোথাও নেই |
| **টার্মিনাল** | `shell/src/main.rs::handle_terminal_cmd` (l.1790–2122) | 🟡 মিশ্র | `ls/cat/mem/net/ps/services/uname/disk` আসল `/proc`,`/sys` পড়ে; কিন্তু `modem` fabricated; `python` explicitly refused; কোনো PTY নেই, shell spawn করে না; এটি একটি `Screen` enum ভ্যারিয়েন্ট |
| **স্ট্যাটাস বার** | `shell/src/main.rs::get_battery_display/get_network_display` | 🔴 সিমুলেটেড | battery fallback `"🔋 88%"`, signal fallback `"▂▄▆█ (sim)"` |
| nilui অ্যাপ রানটাইম | `runtime/nilui/src/lib.rs::App::run` | 🟡 আংশিক | for-loop `0..5` টিক → আসল event loop নয় |
| **ব্রাউজার (nilbrowser)** | `apps/nilbrowser/src/main.rs` | 🟡 হোস্টে কাজ করে | `wry 0.56` + `tao 0.34` — ডেস্কটপ উইন্ডো সিস্টেম দরকার; মোবাইলে (DRM-only) চলে না; workspace member নয় |
| ব্রাউজার (mock) | `apps/browser/src/main.rs` | 🔴 স্টাব | শুধু URL স্ট্রিং বদলায়, কোনো fetch/render নেই |
| ডেস্কটপ সিম | `nilsim/` | 🔴 সিমুলেটর | পুরোটাই host-side ভিজ্যুয়াল; `nilsim/src/nilbrowser.rs` Windows-only WebView2 |
| **nilandroidd (কন্টেইনার)** | `services/nilandroidd/src/{main.rs,isolation.rs}` | 🔴 **স্টাব** | `isolation.rs` শুধু ডেটা-স্ট্রাক্ট + validator; কোথাও `unshare`/`clone` নেই; non-unix-এ print "Simulated Android LXC runtime bridge active"; socket-এ সব রিকোয়েস্টে `Pong`/`OK` |
| android layer setup | `android/setup_android_layer.sh` | 🔴 স্টাব | শুধু `mkdir` + echo; LXC/Waydroid চালায় না |
| android client | `runtime/nilandroid-client/src/lib.rs` | 🔴 আংশিক | plain text `"START ..."` পাঠায়; ব্যর্থ হলে "Simulated Intent START" |
| nilhal android backend | `runtime/nilhal/src/backends/android.rs` | 🔴 সিমুলেটেড | battery 88%, SIM "+91XXXXXXXXXX", hard-coded Buds3 Pro |
| android-host camera | `android-host/src/camera.rs` | 🟡 আংশিক | `VALID_BASELINE_JPEG` test-pattern fallback, `is_simulated` ফ্ল্যাগ আছে |
| android-host JNI | `android-host/src/jni_bridge.rs` | 🟡 আংশিক | ১৬টি `Java_org_onuron_mobile_NativeBridge_*`; Surface callback no-op (Java Canvas আঁকে) |
| nilverify/nilfastbootd/nilinstall | `runtime/nilrt/src/bin/*` | 🔴 স্টাব | `nilverify` `/sys/fs/selinux` থাকলে SIGNATURE_OK ছাপে |

**উপরের যেগুলো 🔴/🟡 — এই ডকের বাকিটা ঠিক সেগুলোকে 🔵/🟢 করার নির্দেশনা।**

---

## ২. স্থির আর্কিটেকচার স্পাইন

এই স্পাইন বদলাবে না (AGENTS.md §2, §8):

```
Linux LTS kernel
  └─ nilinit (PID 1: mounts, cgroups v2, SELinux, fscrypt, supervision, socket activation)
       └─ নেটিভ সার্ভিস (nild, netd, audiod, powerd, inputd, camerad, btd, telephonyd, nilkeyd, notifyd, …)
            └─ IPC: nilprotocol (ONUR framed, Unix socket, SO_PEERCRED auth)
                 ├─ NilHAL  (Linux sysfs/evdev/ALSA/V4L2/DRM এর নিরাপদ আবরণ)
                 ├─ nilui-gpu (DRM/KMS 2D কম্পোজিটর, আসল page-flip)
                 └─ runtime/nilrt (sandbox: namespace+seccomp+cgroup+permbroker)
                      └─ NilLang/Alap/NilUI → .nilax (signed) → nilpkg → অ্যাপ
                           └─ nilshell / nilterminal / nilbrowser (নেটিভ UI)

অ্যান্ড্রয়েড সামঞ্জস্য = আলাদা সাবসিস্টেম (android/, android-host/) — নেটিভ স্পাইনের ভিত্তি নয়।
```

**নিষিদ্ধ (maintainer-এর স্পষ্ট অনুমতি ছাড়া):** AOSP-তে মাইগ্রেশন, systemd/OpenRC আনা, ওয়েব/PWA-কে নেটিভ প্ল্যাটফর্ম বানানো, স্যান্ডবক্স বাইপাস করা, "ডিরেক্টরি আছে = চলছে" ধরে নেওয়া।

---

## ৩. Phase 0 — "No-Stub" বিল্ড গেট

স্টাব প্রোডাকশনে যাওয়া আটকাতে বিল্ড-টাইম বাধা বসাও। এটা CI-তেও চলবে।

### ৩.১ গার্ড স্ক্রিপ্ট: `build/no-stub-gate.sh`

```bash
#!/usr/bin/env bash
# build/no-stub-gate.sh — release বিল্ডে সিমুলেশন প্রবেশ নিষিদ্ধ
# ব্যর্থ হলে exit 1। dev-only সিমুলেটর শুধু nilsim/ এ অনুমোদিত।
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCAN_DIRS="$ROOT/nilinit $ROOT/services $ROOT/runtime $ROOT/shell $ROOT/apps $ROOT/softbus $ROOT/pkg $ROOT/protocol"

# যে প্যাটার্নগুলো release-এ থাকা যাবে না
DENY='is_simulated *= *true|Simulated |simulated environment|SIMULATED\]|fallback when .* absent|VALID_BASELINE_JPEG|"🔋 88%"|\(sim\)'

fail=0
while IFS= read -r hit; do
  # nilsim/ এবং #[cfg(test)] বাদ (dev-only)
  case "$hit" in
    */nilsim/*|*/tests/*) continue ;;
  esac
  echo "STUB-FOUND: $hit"
  fail=1
done < <(grep -rInE "$DENY" $SCAN_DIRS --include='*.rs' --include='*.c' 2>/dev/null || true)

if [ "$fail" -ne 0 ]; then
  echo "[FATAL] release tree-তে সিমুলেশন/স্টাব পাওয়া গেছে — বিল্ড বন্ধ।"
  exit 1
fi
echo "[OK] no-stub gate পাস: release tree পরিষ্কার।"
```

`build/build.sh`-এর শুরুতে যোগ করো: `bash "$TOP/build/no-stub-gate.sh"` (এবং CI ওয়ার্কফ্লোতে)।

### ৩.২ ম্যাটুরিটি সততা

`build/gen-maturity.py` (বর্তমানে `shell/src/simulated.rs` পড়ে) প্রতিটি `simulated` স্ক্রিন ধরে `maturity.toml`-এ ব্যাজ বসায়। নতুন কোডে `simulated.rs::SIMULATED_SCREENS` থেকে এন্ট্রি সরালে ব্যাজও চলে যাবে — তাই টার্গেট হলো এই লিস্ট খালি করা, লুকানো নয়।

**DoD:** `grep -r "SIMULATED" nilinit services runtime shell apps` → release-এ **০ ম্যাচ**।

---

## ৪. Phase 1 — বুট চেইন

বর্তমান চেইন **আসল**, তবে কয়েকটা জায়গায় হার্ডেনিং দরকার। এখানে ধাপে ধাপে।

### ৪.১ কার্নেল

defconfig ফ্র্যাগমেন্ট রেফারেন্স মাত্র (`kernel/nilos_defconfig` base, `nilos_x86.defconfig`, `nilos-halium.defconfig`)। বাস্তবে বুট করা কার্নেল Alpine `vmlinuz-virt` (SHA-256 পিন করা `targets/*/target.toml`-এ)। **রিয়েল ডিভাইসের জন্য নিজের কার্নেল বিল্ড করতে হবে** — নিচের ফ্র্যাগমেন্ট baseline:

```ini
# kernel/nilos_defconfig (আসল বেস — অপরিবর্তিত রাখো)
CONFIG_NAMESPACES=y
CONFIG_UTS_NS=y
CONFIG_IPC_NS=y
CONFIG_USER_NS=y
CONFIG_PID_NS=y
CONFIG_NET_NS=y
CONFIG_CGROUPS=y
CONFIG_MEMCG=y
CONFIG_CGROUP_SCHED=y
CONFIG_SECCOMP=y
CONFIG_SECCOMP_FILTER=y
CONFIG_SECURITY_SELINUX=y
CONFIG_SECURITY_SELINUX_BOOTPARAM=y
CONFIG_FS_ENCRYPTION=y
CONFIG_EXT4_FS=y
CONFIG_EXT4_FS_ENCRYPTION=y
CONFIG_DRM=y
CONFIG_DRM_VIRTIO_GPU=y
CONFIG_DMABUF_HEAPS=y
CONFIG_DMABUF_HEAPS_SYSTEM=y
CONFIG_ANDROID_BINDER_IPC=y
CONFIG_ANDROID_BINDERFS=y
CONFIG_ASHMEM=y
CONFIG_WIREGUARD=y
```

ARM64 মোবাইলের জন্য যোগ করো: `CONFIG_DRM_MSM=y`, `CONFIG_INPUT_EVDEV=y`, `CONFIG_SND_SOC=y`, `CONFIG_VIDEO_V4L2=y`, `CONFIG_USB_GADGET=y` + device tree (`targets/oneplus-fajita/target.toml`-এ `dtb = "sdm845-oneplus-fajita.dtb"`)।

### ৪.২ initramfs

`build/mkinitramfs.py` আসল (pure-Python cpio.gz, `nilinit` → `/init`,`/sbin/init`,`/bin/nilinit` লিংক করে)। এটা রাখো।

### ৪.৩ nilinit — আসল বুট কোড (রেফারেন্স)

এটাই আসল পাথ (বদলাতে হবে না, শুধু পড়ে বুঝে নাও):

```rust
// nilinit/src/main.rs — আসল ক্রম (সারমর্ম)
fn main() {
    mount_early_fs();          // proc, sys, dev(devtmpfs), run, tmp, cgroup2, selinuxfs
    setup_cgroups();           // /sys/fs/cgroup/{system,onuron,nilos}.slice + subtree_control
    mount_data_partition();    // /dev/vda|vda1|... → ext4 → /data (বলপূর্বক, tmpfs-এ নীরব fallback নয়)
    load_selinux();            // policy.33 → /sys/fs/selinux/load, enforce=1
    load_fscrypt_key();        // /data/system/fscrypt/policy → keyd.sock → FS_IOC_ADD_ENCRYPTION_KEY
    let plan = decide_boot();  // Recovery{..} | Normal{failed_boots}
    let cfg  = load_services(); // /etc/nilos/services.toml (fallback: include_str!)
    // socket-activated সার্ভিস বাদ দিয়ে বাকিগুলো স্পন
    for s in cfg.services.iter().filter(|s| s.socket_activation.is_none()) { sup.spawn(s); }
    loop { sup.tick(); activator.check_pending(); handle_power_action(); sleep(500ms); }
}
```

**হার্ডেনিং (এই ডকে করার কথা):** নিচের নীতিগুলো আসল কোডে যোগ করো —
1. `/data` পেলেই **শুধু** সফল ধরে নেওয়া নয় — `/proc/mounts`-এ ext block mount যাচাই করো (কিছুটা আছে, আরও কড়া করো)।
2. core-health-এ `nilshell` ছাড়া সবগুলোর socket-probe বাধ্যতামূলক (আছে) — `NILRT_ALLOW_INSECURE_DEV` release-এ অকার্যকর করো।
3. boot-complete সিল করার আগে `nilverify` দিয়ে vbmeta hash যাচাই করাও (নিচে §১৩)।

**DoD:** `qemu-smoke.py` + `qemu-persistence-test.py` পাস; power-loss test-এ `/data` ASH-এ অটুট।

---

## ৫. Phase 2 — IPC কন্ট্রাক্ট ও সার্ভিস টপোলজি

### ৫.১ কন্ট্রাক্ট ফ্রিজ (আসল, বদলাবে না)

```rust
// protocol/src/lib.rs (আসল হেডার লেআউট)
pub const PROTOCOL_MAGIC: [u8; 4] = *b"ONUR";
pub const PROTOCOL_VERSION_1: u16 = 1;
pub const MAX_PAYLOAD_SIZE: u32 = 1024 * 1024; // 1 MiB
pub const HEADER_SIZE: usize = 24;
// BE ক্রম: magic[4] | payload_len u32 | version u16 | msg_type u16 | request_id u64 | flags u32
pub struct Frame { pub version: u16, pub message_type: u16, pub request_id: u64, pub flags: u32, pub payload: Vec<u8> }
```

`read_frame` magic/version/size যাচাই করে; ভুল হলে `IpcError::{InvalidMagic, UnsupportedVersion, PayloadTooLarge}`।

### ৫.২ সকেট পাথের বাগ — অবশ্যই সারাতে হবে

`etc/nilos/services.toml` বলছে `inputd → /run/nilos/input.sock` কিন্তু `services/inputd` আসলে `/run/onuron/input.sock` বাইন্ড করে; একইভাবে `netd/onuron/net.sock`, `powerd/onuron/power.sock`, `nilupd/onuron/update.sock`। socket-activation FD আর fallback পাথ তখন আলাদা → স্পন করা সার্ভিস কখনো ট্রাফিক পায় না।

**ফিক্স (এক প্যাটার্নে):** প্রতিটি ডেমন `nilsd::first_listener_or_bind` কে `services.toml`-এর পাথ দিক, অথবা `etc/nilos/services.toml` ও কোড দুটোতেই এক পাথ লেখো। উদাহরণ:

```rust
// services/inputd/src/main.rs — services.toml-এর পাথের সাথে এক
const INPUT_SOCK: &str = "/run/nilos/input.sock";
let listener = nilsd::first_listener_or_bind(INPUT_SOCK)?;
nilsd::notify_ready("inputd", Some(INPUT_SOCK));
```

একইভাবে `netd → /run/nilos/net.sock`, `powerd → /run/nilos/power.sock`, `nilupd → /run/nilos/update.sock`। `services.toml`-এও হুবহু এই পাথ লেখো, যাতে socket-activation আর fallback অভিন্ন হয়।

**DoD:** `ls -l /run/nilos/*.sock /run/onuron/*.sock` — declared আর live পাথ অভিন্ন; `nild` readiness probe থেকে `Pong` ফেরে।

---

## ৬. Phase 3 — আসল DRM/KMS কম্পোজিটর

সমস্যা: `runtime/nilui-gpu/src/drm.rs`-এ `set_crtc()`/`page_flip()` শুধু `fb_id` চেক করে `Ok(())` ফেরায়। `vkctx.rs`/`atlas.rs` শুধু print। **তাই এখন কোনোকিছু পর্দায় যায় না।** নিচে আসল ioctl-ভিত্তিক রিপ্লেসমেন্ট।

### ৬.১ আসল ioctl কনস্ট্যান্ট (`_IOC` ম্যাক্রো)

libdrm-এর `DRM_IOWR` হুবহু রিপ্রোডিউস করো:

```rust
// runtime/nilui-gpu/src/drm_ioctl.rs (নতুন)
#![allow(non_camel_case_types)]
use std::io;

const IOC_NRBITS: u32 = 8;
const IOC_TYPEBITS: u32 = 8;
const IOC_SIZEBITS: u32 = 14;
const IOC_DIRBITS: u32 = 2;
const IOC_NRSHIFT: u32 = 0;
const IOC_TYPESHIFT: u32 = IOC_NRSHIFT + IOC_NRBITS;          // 8
const IOC_SIZESHIFT: u32 = IOC_TYPESHIFT + IOC_TYPEBITS;      // 16
const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;       // 30
const IOC_NONE: u32 = 0;
const IOC_WRITE: u32 = 1;
const IOC_READ: u32 = 2;
const DRM_IOCTL_BASE: u32 = b'd' as u32; // 'd'

const fn ioc(dir: u32, ty: u32, nr: u32, size: u32) -> u64 {
    ((dir << IOC_DIRSHIFT)
        | (ty << IOC_TYPESHIFT)
        | (nr << IOC_NRSHIFT)
        | (size << IOC_SIZESHIFT)) as u64
}
const fn iowr<T>(nr: u32) -> u64 { ioc(IOC_READ | IOC_WRITE, DRM_IOCTL_BASE, nr, std::mem::size_of::<T>() as u32) }
const fn iow<T>(nr: u32) -> u64 { ioc(IOC_WRITE, DRM_IOCTL_BASE, nr, std::mem::size_of::<T>() as u32) }

#[repr(C)] #[derive(Default, Clone, Copy)]
pub struct drm_mode_create_dumb { pub height: u32, pub width: u32, pub bpp: u32, pub flags: u32, pub handle: u32, pub pitch: u32, pub size: u64 }
#[repr(C)] #[derive(Default, Clone, Copy)]
pub struct drm_mode_map_dumb { pub handle: u32, pub pad: u32, pub offset: u64 }
#[repr(C)] #[derive(Default, Clone, Copy)]
pub struct drm_mode_destroy_dumb { pub handle: u32 }
#[repr(C)] #[derive(Default, Clone, Copy)]
pub struct drm_mode_fb_cmd { pub fb_id: u32, pub width: u32, pub height: u32, pub pitch: u32, pub bpp: u32, pub depth: u32, pub handle: u32 }
#[repr(C)] #[derive(Default, Clone, Copy)]
pub struct drm_mode_crtc { pub set_connectors_ptr: u64, pub count_connectors: u32, pub crtc_id: u32, pub fb_id: u32, pub x: u32, pub y: u32, pub gamma_size: u32, pub mode_valid: u32, pub mode: drm_mode_modeinfo }
#[repr(C)] #[derive(Default, Clone, Copy)]
pub struct drm_mode_modeinfo { pub clock: u32, pub hdisplay: u16, pub hsync_start: u16, pub hsync_end: u16, pub htotal: u16, pub hskew: u16, pub vdisplay: u16, pub vsync_start: u16, pub vsync_end: u16, pub vtotal: u16, pub vscan: u16, pub vrefresh: u32, pub flags: u32, pub type_: u32, pub name: [u8; 32] }
#[repr(C)] #[derive(Default, Clone, Copy)]
pub struct drm_mode_crtc_page_flip { pub crtc_id: u32, pub fb_id: u32, pub flags: u32, pub reserved: u32, pub user_data: u64 }

pub const DRM_IOCTL_MODE_CREATE_DUMB:  u64 = iowr::<drm_mode_create_dumb>(0xB2);
pub const DRM_IOCTL_MODE_MAP_DUMB:     u64 = iowr::<drm_mode_map_dumb>(0xB3);
pub const DRM_IOCTL_MODE_DESTROY_DUMB: u64 = iowr::<drm_mode_destroy_dumb>(0xB4);
pub const DRM_IOCTL_MODE_ADDFB:        u64 = iowr::<drm_mode_fb_cmd>(0xAE);
pub const DRM_IOCTL_MODE_SETCRTC:      u64 = iowr::<drm_mode_crtc>(0xA2);
pub const DRM_IOCTL_MODE_PAGE_FLIP:    u64 = iowr::<drm_mode_crtc_page_flip>(0xB0);

pub fn drm_ioctl<T>(fd: i32, req: u64, arg: &mut T) -> io::Result<()> {
    let r = unsafe { libc::ioctl(fd, req as libc::c_ulong, arg as *mut T) };
    if r < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}
```

### ৬.২ আসল Dumb Buffer + FB + SetCrtc + PageFlip

```rust
// runtime/nilui-gpu/src/drm.rs — drm_ioctl মডিউল ব্যবহার করে আসল ইমপ্লিমেন্টেশন
use crate::drm_ioctl::*;
use std::os::unix::io::RawFd;

pub struct RealDrm {
    pub fd: RawFd,
    pub crtc_id: u32,
    pub connector_id: u32,
    pub mode: drm_mode_modeinfo,
}

pub struct RealDumb {
    pub handle: u32,
    pub fb_id: u32,
    pub pitch: u32,
    pub size: u64,
    pub mmap: *mut u8,
}

impl RealDrm {
    /// আসল dumb buffer অ্যালোকেট + mmap + FB রেজিস্টার
    pub unsafe fn create_framebuffer(&self, w: u32, h: u32) -> std::io::Result<RealDumb> {
        let mut dumb = drm_mode_create_dumb { width: w, height: h, bpp: 32, ..Default::default() };
        drm_ioctl(self.fd, DRM_IOCTL_MODE_CREATE_DUMB, &mut dumb)?;

        let mut map = drm_mode_map_dumb { handle: dumb.handle, ..Default::default() };
        drm_ioctl(self.fd, DRM_IOCTL_MODE_MAP_DUMB, &mut map)?;

        let mmap = libc::mmap(
            std::ptr::null_mut(), dumb.size as usize,
            libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED,
            self.fd, map.offset as libc::off_t,
        );
        if mmap == libc::MAP_FAILED { return Err(std::io::Error::last_os_error()); }

        let mut fb = drm_mode_fb_cmd {
            width: w, height: h, pitch: dumb.pitch, bpp: 32, depth: 24, handle: dumb.handle, ..Default::default()
        };
        drm_ioctl(self.fd, DRM_IOCTL_MODE_ADDFB, &mut fb)?;

        Ok(RealDumb { handle: dumb.handle, fb_id: fb.fb_id, pitch: dumb.pitch, size: dumb.size, mmap: mmap as *mut u8 })
    }

    /// আসল মোডসেট — এটাই যা আগে কখনো কল হত না
    pub unsafe fn set_crtc(&self, fb_id: u32) -> std::io::Result<()> {
        let mut connectors = [self.connector_id];
        let mut crtc = drm_mode_crtc {
            set_connectors_ptr: connectors.as_mut_ptr() as u64,
            count_connectors: 1,
            crtc_id: self.crtc_id,
            fb_id,
            x: 0, y: 0,
            mode_valid: 1,
            mode: self.mode,
            ..Default::default()
        };
        drm_ioctl(self.fd, DRM_IOCTL_MODE_SETCRTC, &mut crtc)
    }

    /// আসল ভি-ব্ল্যাংক-সিঙ্কড পেজ ফ্লিপ
    pub unsafe fn page_flip(&self, fb_id: u32, user_data: u64) -> std::io::Result<()> {
        let mut flip = drm_mode_crtc_page_flip {
            crtc_id: self.crtc_id, fb_id, flags: 0x01 /* PAGE_FLIP_EVENT */, user_data, ..Default::default()
        };
        drm_ioctl(self.fd, DRM_IOCTL_MODE_PAGE_FLIP, &mut flip)
    }
}
```

**ভি-ব্ল্যাংক ইভেন্ট গ্রহণ:** `DRM_IOCTL_MODE_PAGE_FLIP` `PAGE_FLIP_EVENT` ফ্ল্যাগে ইভেন্ট দেয়। `drm.rs`-এ `read()` লুপে `drm_event` পার্স করে ফ্লিপ কনফার্ম করতে হবে; নইলে ড্রপ ফ্রেম চোখে পড়বে। পরবর্তী ফ্রেম দিতে আগের ইভেন্ট পড়া বাধ্যতামূলক।

**পরীক্ষা (কোড ছাড়াই, শুধু ট্রেস দিয়ে):**
```bash
# রিয়েল ডিভাইসে: mode set সফল হলে kernel log-এ CRTC enable দেখবে
dmesg | grep -iE 'drm|msm|connector'
# fb_id != 0 এবং page flip ioctl 0 ফেরালেই সফল
```

**DoD (কম্পোজিটরের):**
- [ ] `/dev/dri/card0` খুলে connector/crtc/mode এনুমারেট করা হয়েছে (আসল `GETCONNECTOR`/`GETCRTC` ioctl)।
- [ ] `CREATE_DUMB → ADDFB → SETCRTC` সফল, পর্দায় সলিড রঙ।
- [ ] টানা ৬০ সেকেন্ড page-flip, dropped frame telemetry log হয়।
- [ ] `vkctx.rs`/`atlas.rs` থেকে print-only ফাংশন সরিয়ে হয় আসল Vulkan শেপিং, নয়তো `ash` ডিপেন্ডেন্সি বাদ দিয়ে ফাইল মোছা।

---

## ৭. Phase 4 — আসল evdev ইনপুট ইঞ্জিন

`services/inputd` (functional prototype) — এখানে সত্যিকারের evdev পড়া ও gesture state machine থাকা উচিত। নিচে রেফারেন্স রিডার:

```rust
// services/inputd/src/evdev.rs
use std::os::fd::RawFd;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct input_event { pub tv_sec: i64, pub tv_usec: i64, pub type_: u16, pub code: u16, pub value: i32 }

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_ABS: u16 = 0x03;
pub const SYN_REPORT: u16 = 0;
pub const ABS_MT_POSITION_X: u16 = 0x35;
pub const ABS_MT_POSITION_Y: u16 = 0x36;
pub const ABS_MT_TRACKING_ID: u16 = 0x39;

pub fn read_event(fd: RawFd) -> std::io::Result<input_event> {
    let mut ev = input_event::default();
    let n = unsafe { libc::read(fd, &mut ev as *mut _ as *mut libc::c_void, std::mem::size_of::<input_event>()) };
    if n < 0 { return Err(std::io::Error::last_os_error()); }
    if (n as usize) < std::mem::size_of::<input_event>() { return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "short read")); }
    Ok(ev)
}
```

**ডিভাইস ডিসকভারি:** `/dev/input/event*` স্ক্যান → `EVIOCGBIT(EV_ABS)` দিয়ে multi-touch যাচাই → `EVIOCGABS(ABS_MT_SLOT)` রেঞ্জ পড়া। এরপর SYN_REPORT-এ ফ্রেম শেষ করে `nilprotocol::Frame` (msg_type `Input*`) হিসেবে কম্পোজিটরে পাঠানো।

**DoD:** আসল টাচে প্রতিটি আঙুলের (x,y,tracking_id) আসল রেঞ্জ আসে; সিমুলেটেড কোনো কোঅর্ডিনেট নেই।

---

## ৮. Phase 5 — আসল টার্মিনাল

বর্তমান "Terminal" আসলে `shell/src/main.rs`-এর `Screen::AppTerminal` — কোনো PTY নেই, কোনো শেল স্পন হয় না, `modem` fabricated। এখানে আসল পথ।

### ৮.১ নতুন crate `apps/nilterminal`

```
apps/nilterminal/
  Cargo.toml
  src/main.rs        # UI + input
  src/pty.rs         # forkpty / termios
  src/vt.rs          # ANSI/VT100 পার্সার (state machine)
  src/proc.rs        # /proc,/sys ভিত্তিক আসল কমান্ড
```

`Cargo.toml`:
```toml
[package]
name = "nilterminal"
version = "0.1.0"
edition = "2021"

[dependencies]
libc = "0.2"
nilui = { path = "../../runtime/nilui" }

[[bin]]
name = "nilterminal"
path = "src/main.rs"
```
(workspace `members`-এ `"apps/nilterminal"` যোগ করো।)

### ৮.২ আসল PTY (`src/pty.rs`)

```rust
use std::os::fd::RawFd;

pub struct Pty { pub master: RawFd, pub child_pid: libc::pid_t }

/// forkpty() দিয়ে রিয়েল pseudoterminal + শেল প্রসেস
pub fn spawn_shell(shell: &str) -> std::io::Result<Pty> {
    let mut master: libc::c_int = -1;
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    ws.ws_row = 40; ws.ws_col = 100;
    ws.ws_xpixel = 0; ws.ws_ypixel = 0;

    let c_shell = std::ffi::CString::new(shell)?;
    let mut argv: [*const libc::c_char; 2] = [c_shell.as_ptr(), std::ptr::null()];

    let pid = unsafe {
        libc::forkpty(&mut master, std::ptr::null_mut(), std::ptr::null_mut(), &mut ws)
    };
    if pid < 0 { return Err(std::io::Error::last_os_error()); }
    if pid == 0 {
        // সন্তান: এইমাত্র নতুন PTY-এর slave stdin/stdout/stderr
        unsafe { libc::execvp(c_shell.as_ptr(), argv.as_mut_ptr()); }
        unsafe { libc::_exit(127); }
    }
    // master ফাইল-ডেস্ক্রিপ্টর non-blocking করো
    unsafe {
        let fl = libc::fcntl(master, libc::F_GETFL);
        libc::fcntl(master, libc::F_SETFL, fl | libc::O_NONBLOCK);
    }
    Ok(Pty { master, child_pid: pid })
}

pub fn pty_read(master: RawFd, buf: &mut [u8]) -> std::io::Result<usize> {
    let n = unsafe { libc::read(master, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    if n < 0 {
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() == Some(libc::EAGAIN) { return Ok(0); }
        return Err(e);
    }
    Ok(n as usize)
}

pub fn pty_write(master: RawFd, data: &[u8]) -> std::io::Result<()> {
    let mut off = 0;
    while off < data.len() {
        let n = unsafe { libc::write(master, data[off..].as_ptr() as *const libc::c_void, data.len() - off) };
        if n < 0 { return Err(std::io::Error::last_os_error()); }
        off += n as usize;
    }
    Ok(())
}
```

### ৮.৩ আসল VT পার্সার (`src/vt.rs`) — সংক্ষিপ্ত কিন্তু কাজ করা

```rust
#[derive(Default)]
pub struct VtParser { state: State, params: Vec<u32>, cur: u32, pub screen: Vec<Vec<char>>, pub row: usize, pub col: usize }
#[derive(Default, PartialEq)]
enum State { Ground, Esc, Csi, CsiParam }

impl VtParser {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self { screen: vec![vec![' '; cols]; rows], row: 0, col: 0, ..Default::default() }
    }
    /// PTY থেকে আসা বাইট স্ট্রিম পার্স করে screen buffer আপডেট করে
    pub fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            let c = b as char;
            match self.state {
                State::Ground => match b {
                    0x1B => self.state = State::Esc,
                    b'\n' => { self.row = (self.row + 1).min(self.screen.len() - 1); self.col = 0; }
                    b'\r' => self.col = 0,
                    0x08 => self.col = self.col.saturating_sub(1),
                    _ if c >= ' ' => { self.put(c); }
                    _ => {}
                },
                State::Esc => match b {
                    b'[' => { self.state = State::Csi; self.params.clear(); self.cur = 0; }
                    b'c' => { self.reset(); self.state = State::Ground; } // RIS
                    _ => { self.state = State::Ground; }
                },
                State::Csi => match b {
                    b'0'..=b'9' => { self.cur = self.cur * 10 + (b - b'0') as u32; self.state = State::CsiParam; }
                    b';' => { self.params.push(self.cur); self.cur = 0; self.state = State::CsiParam; }
                    _ => self.csi_dispatch(b),
                },
                State::CsiParam => match b {
                    b'0'..=b'9' => { self.cur = self.cur * 10 + (b - b'0') as u32; }
                    b';' => { self.params.push(self.cur); self.cur = 0; }
                    _ => self.csi_dispatch(b),
                },
            }
        }
    }
    fn put(&mut self, c: char) {
        if self.col >= self.screen[0].len() { self.col = 0; self.row = (self.row + 1).min(self.screen.len() - 1); }
        self.screen[self.row][self.col] = c;
        self.col += 1;
    }
    fn csi_dispatch(&mut self, final_byte: u8) {
        let mut p = self.params.clone(); p.push(self.cur);
        match final_byte {
            b'H' | b'f' => { // cursor position
                self.row = (p.first().copied().unwrap_or(1).max(1) as usize - 1).min(self.screen.len() - 1);
                self.col = (p.get(1).copied().unwrap_or(1).max(1) as usize - 1).min(self.screen[0].len() - 1);
            }
            b'J' => { // erase display
                for r in self.screen.iter_mut() { r.iter_mut().for_each(|c| *c = ' '); }
                self.row = 0; self.col = 0;
            }
            b'K' => { for c in self.screen[self.row].iter_mut().skip(self.col) { *c = ' '; } }
            b'A' => self.row = self.row.saturating_sub(p.first().copied().unwrap_or(1) as usize),
            b'B' => self.row = (self.row + p.first().copied().unwrap_or(1) as usize).min(self.screen.len() - 1),
            b'C' => self.col = (self.col + p.first().copied().unwrap_or(1) as usize).min(self.screen[0].len() - 1),
            b'D' => self.col = self.col.saturating_sub(p.first().copied().unwrap_or(1) as usize),
            _ => {}
        }
        self.params.clear(); self.cur = 0; self.state = State::Ground;
    }
    fn reset(&mut self) {
        for r in self.screen.iter_mut() { r.iter_mut().for_each(|c| *c = ' '); }
        self.row = 0; self.col = 0;
    }
}
```

### ৮.৪ আসল কমান্ড (`src/proc.rs`) — সিমুলেশন-মুক্ত

```rust
use std::fs;

/// ps → আসল /proc/*/comm  (কোনো fabricated লাইন নয়)
pub fn ps() -> String {
    let mut out = String::from("PID      COMMAND\n");
    if let Ok(entries) = fs::read_dir("/proc") {
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(cmd) = fs::read_to_string(format!("/proc/{}/comm", name)) {
                    out.push_str(&format!("{:<8} {}\n", name, cmd.trim()));
                }
            }
        }
    }
    out
}

/// net → আসল /sys/class/net/*/operstate
pub fn net() -> String {
    let mut out = String::from("IFACE        STATE\n");
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for e in entries.flatten() {
            let iface = e.file_name().to_string_lossy().to_string();
            let state = fs::read_to_string(format!("/sys/class/net/{}/operstate", iface))
                .unwrap_or_else(|_| "unknown".into());
            out.push_str(&format!("{:<12} {}\n", iface, state.trim()));
        }
    }
    out
}

/// services → আসল socket + /proc উপস্থিতি (কোনো fabricated নয়)
pub fn services() -> String {
    const SOCKS: &[(&str, &str)] = &[
        ("nild", "/run/onuron/ready/nild"), ("nilkeyd", "/run/nilos/keyd.sock"),
        ("nilbus", "/run/nilos/bus.sock"), ("netd", "/run/nilos/net.sock"),
        ("audiod", "/run/nilos/audio.sock"), ("powerd", "/run/nilos/power.sock"),
        ("inputd", "/run/nilos/input.sock"), ("btd", "/run/nilos/bt.sock"),
        ("telephonyd", "/run/nilos/telephony.sock"), ("camerad", "/run/nilos/camera.sock"),
    ];
    let mut out = String::new();
    for (name, path) in SOCKS {
        let mark = if std::path::Path::new(path).exists() { "ACTIVE" } else { "DOWN" };
        out.push_str(&format!("{:<14} {} ({})\n", name, mark, path));
    }
    out
}

/// modem → আসল modem node উপস্থিতি; না থাকলে সৎভাবে "not present", সিমুলেশন নয়
pub fn modem() -> String {
    for p in ["/dev/ttyUSB0", "/dev/cdc-wdm0", "/dev/wwan0"] {
        if std::path::Path::new(p).exists() { return format!("modem: hardware present at {}", p); }
    }
    "modem: no cellular modem detected on this device".into()
}
```

### ৮.৫ main রান-লুপ

```rust
use crate::pty::{spawn_shell, pty_read, pty_write};
use crate::vt::VtParser;

pub fn run_terminal() {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let pty = match spawn_shell(&shell) { Ok(p) => p, Err(e) => { eprintln!("pty: {e}"); return; } };
    let mut vt = VtParser::new(40, 100);
    let mut buf = [0u8; 4096];
    loop {
        // 1) শেলের আউটপুট → VT পার্সার
        match pty_read(pty.master, &mut buf) {
            Ok(0) => {}
            Ok(n) => vt.feed(&buf[..n]),
            Err(e) => { eprintln!("pty read: {e}"); break; }
        }
        // 2) কীবোর্ড টেক্সট (nilui ইভেন্ট) → PTY
        if let Some(input) = nilui::poll_keyboard_input() {
            let _ = pty_write(pty.master, input.as_bytes());
        }
        // 3) VT screen → nilui Element-এ রেন্ডার + present
        nilui::present_vt_screen(&vt.screen);
    }
}
```

**DoD:** গ্লাসে `sh` প্রম্পট আসে; `ls`, `cat /proc/uptime`, `ps`, `net`, `services` আসল ডেটা দেয়; `modem` সত্য কথা বলে (এলাকা অনুযায়ী); **কোনো `[SIMULATED]` ব্যাজ নেই**।

---

## ৯. Phase 6 — আসল ব্রাউজার

### ৯.১ সিদ্ধান্ত (অটল): নিজে HTML/CSS/JS ইঞ্জিন লেখা **অনুমোদিত নয়**

Chromium/WebKit লেখা একটি multi-year প্রকল্প। AGENTS.md §10 অনুযায়ী mature উপাদান ব্যবহার করো। মোবাইলে (Wayland-less, DRM-only) সবচেয়ে ব্যবহারযোগ্য পথ: **WPE WebKit** (`wpewebkit` + `libwpe`, DRM/KMS বেকএন্ড) অথবা ডিভাইসে একটা **Wayland** সার্ভার বসিয়ে `webkit2gtk-4.1`।

`apps/nilbrowser` বর্তমানে `wry`+`tao` ব্যবহার করে — এটা Windows/macOS/X11/Wayland হোস্টে কাজ করে, কিন্তু DRM-only মোবাইলে নয়। তাই:

- **ডেস্কটপ/dev (Track A):** `wry`+`tao` রাখো — WebView2/WebKitGTK ব্যবহার করে, এটা বৈধ।
- **মোবাইল (Track C):** WPE WebKit (`cog` প্যাটার্ন) কে `nilui-gpu`-এর DRM আউটপুটে চালাও।

### ৯.২ WPE ভিউ তৈরি (রেফারেন্স)

WPE C API ব্যবহার করে একটি ভিউ বানিয়ে তার buffers `nilui-gpu` কম্পোজিটরে ব্লিট করো। ন্যূনতম গঠন:

```rust
// apps/nilbrowser/src/wpe.rs — WPE WebKit ভিউ (Linux/ARM)
#![cfg(target_os = "linux")]
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};

extern "C" {
    fn wpe_view_backend_create(backend: *const c_void) -> *mut c_void;
    fn wpe_loader_init(impl_library_name: *const c_char) -> bool;
    fn cog_init(platform: *const c_char) -> bool; // cog প্ল্যাটফর্ম init (drm)
    fn cog_platform_drm_setup(device: *const c_char) -> bool;
}

pub fn start_drm_browser(url: &str, drm_card: &str) -> bool {
    let dev = CString::new(drm_card).unwrap();
    let plat = CString::new("drm").unwrap();
    let lib = CString::new("libWPEBackend-fdo-1.0.so.1").unwrap();
    unsafe {
        if !wpe_loader_init(lib.as_ptr()) { eprintln!("WPE loader init failed"); return false; }
        if !cog_init(plat.as_ptr()) { eprintln!("cog init failed"); return false; }
        if !cog_platform_drm_setup(dev.as_ptr()) { eprintln!("cog DRM setup failed"); return false; }
    }
    let _ = url;
    // এরপর cog DRM প্ল্যাটফর্ম KMS-এ সরাসরি রেন্ডার করে; nilui-gpu তাকে surface owner হিসেবে চালায়
    true
}
```

> **সম্মতি:** WPE/cog ডিভাইসে প্যাকেজ করতে হবে (`meta-wpe`, `libwpe`, `wpebackend-fdo`)। পুরো ক্রেডিট ও আপডেট WPE-র; অনুরণ শুধু **হোস্ট/সারফেস/UA/ভিগনেট** মালিক।

### ৯.৩ ডিভাইস-নিরপেক্ষ ফলব্যাক (আসল HTTP, fabricated নয়)

ইঞ্জিন না থাকলে টেক্সট মোড রিডার — কিন্তু **আসল** নেটওয়ার্ক ক্লায়েন্ট দিয়ে, আর **কোনো বানানো** `"✓ SSL TLS 1.3 | 200 OK"` স্ট্রিং নয় — আসল status কোড:

```rust
// apps/nilbrowser/src/reader.rs — আসল HTTP ফেচ (ureq), sত্যিকারের স্ট্যাটাস
pub fn fetch_text(url: &str) -> Result<(u16, String, String), String> {
    let resp = ureq::get(url)
        .set("User-Agent", "Mozilla/5.0 (Linux; Android 14; OnuronOS) NilBrowser/1.0")
        .timeout(std::time::Duration::from_secs(20))
        .call()
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let title = extract_title(&resp.into_string().map_err(|e| e.to_string())?);
    Ok((status, title, String::new()))
}

fn extract_title(html: &str) -> String {
    if let Some(s) = html.find("<title") { if let Some(gt) = html[s..].find('>') {
        if let Some(end) = html[s + gt..].find("</title>") {
            return html[s + gt + 1..s + gt + end].trim().to_string();
        }
    }}
    String::new()
}
```
(workspace `apps/nilbrowser/Cargo.toml`-এ `ureq = "2"` যোগ করো।)

**DoD:** রিয়েল URL লোড হয় (WPE dev-এ, WPE/Android-host-এ ডিভাইসে), অথবা ফলব্যাকে আসল HTTP status দেখায়; বানানো স্ট্রিং থাকে না।

---

## ১০. Phase 7 — প্রতিটি নেটিভ অ্যাপ

নেটিভ অ্যাপ গঠন (`runtime/nilui`): state struct + `update(&mut S, Ev)` + `view(&S) -> Element` + `App{..}.run(title)`। কিন্তু `App::run` শুধু ৫ টিক চালায় — **আসল ইভেন্ট লুপ লাগবে**। প্রথমে `runtime/nilui/src/lib.rs::App::run` ঠিক করো যাতে `render_element_and_collect_targets` + `hit_test` + আসল সারফেস present চলে, এবং ইভেন্ট (touch/ime) আসে ইনপুট সোর্স থেকে।

তারপর প্রতিটি অ্যাপ (আসল ডেটা-সোর্স):

| অ্যাপ | crate | আসল সোর্স | স্টাব সরানোর কাজ |
|---|---|---|---|
| Phone | `apps/phone` | telephonyd (`/run/nilos/telephony.sock`), AT/modem node | fabricated call fallback সরাও; modem না থাকলে সৎ "no modem" দেখাও |
| Messages | `apps/messages` | telephonyd SMS + `/data/sms/` | real PDU encode/decode; আসল thread ডেটা |
| Camera | `apps/camera` | camerad → V4L2 (`/dev/video*`) | `VALID_BASELINE_JPEG` সরাও; `is_simulated` হলে ত্রুটি দেখাও |
| Music | `apps/music` | audiod ALSA + `/data/music` | ALSA না থাকলে "no audio device" সত্য কথা |
| Files | `apps/files` | POSIX VFS (`/data,/system,/tmp`) | আসল read/write permissions |
| Settings | `apps/settings` | `/sys/class/power_supply`, rfkill, backlight, fscrypt ioctl | সিমুলেটেড SELinux/fscrypt রিপোর্ট সরাও; আসল `getenforce`-এর মতো পড়া |
| Calculator/Notes | `apps/calculator`,`apps/notes` | evaluator + `/data/notes/notes.json` | ইতিমধ্যে আসল — শুধু টেস্ট |
| Launcher/OOBE/Lockscreen | `apps/*` | `/data/config`, PIN hashing | event loop ঠিক হলেই ঠিক |
| WiFi/Bluetooth/Display/Sound | `apps/*` | `/sys/class/net`, rfkill, backlight, mixer | fabricated ইনডিকেটর → আসল sysfs |

**নিয়ম:** প্রতিটি `apps/<x>/src/main.rs` থেকে `is_simulated`/fabricated পাথ মুছে ফেলো; ডেটা না থাকলে **ত্রুটি/খালি অবস্থা** দেখাও, বানানো সংখ্যা নয়।

**Phone-এর জন্য আসল AT রেফারেন্স (সারমর্ম):**
```rust
// apps/phone বা telephonyd: আসল modem AT (কোনো fabricated নয়)
pub fn dial(dev: &str, number: &str) -> std::io::Result<()> {
    use std::io::{Read, Write};
    let mut f = std::fs::OpenOptions::new().read(true).write(true).open(dev)?; // /dev/ttyUSB2
    f.write_all(b"ATD")?; f.write_all(number.as_bytes())?; f.write_all(b";\r")?;
    let mut resp = String::new(); f.read_to_string(&mut resp)?; // "OK" বা "+CME ERROR"
    Ok(())
}
```

---

## ১১. Phase 8 — অ্যান্ড্রয়েড ব্রিজ

বর্তমান: `nilandroidd` শুধু প্রোফাইল যাচাই করে, কোনো কন্টেইনার চালায় না; non-unix-এ "Simulated Android LXC runtime bridge active" ছাপে; `isolation.rs` শুধু ডেটা। `android/setup_android_layer.sh` শুধু `mkdir`।

### ১১.১ আসল কন্টেইনার আইসোলেশন + লঞ্চ (`services/nilandroidd/src/container.rs`)

ডেটা-স্ট্রাক্টের বদলে **আসল** namespace/cgroup/mount/pivot:

```rust
// services/nilandroidd/src/container.rs — আসল আইসোলেশন ও লঞ্চ
use nix::sched::{unshare, CloneFlags};
use nix::unistd::{fork, ForkResult, sethostname, chdir};
use std::ffi::CString;

const ROOTFS: &str = "/data/android/rootfs";

pub fn launch_android_container() -> Result<(), String> {
    // ১) namespace আইসোলেশন ফ্ল্যাগ
    let flags = CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWPID
        | CloneFlags::CLONE_NEWIPC | CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWNET;
    unshare(flags).map_err(|e| format!("unshare: {e}"))?;

    // ২) কন্টেইনারের hostname
    sethostname("onuron-android").map_err(|e| format!("sethostname: {e}"))?;

    match unsafe { fork().map_err(|e| format!("fork: {e}"))? } {
        ForkResult::Child => {
            // ৩) mount namespace private
            nix::mount::mount(None::<&str>, "/", None::<&str>,
                nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_PRIVATE, None::<&str>)
                .map_err(|e| format!("mount private: {e}"))?;

            // ৪) কন্টেইনার rootfs bind করো
            let rootfs = std::path::Path::new(ROOTFS);
            for m in ["/dev", "/proc", "/sys"] {
                let tgt = format!("{ROOTFS}{m}");
                std::fs::create_dir_all(&tgt).ok();
                nix::mount::mount(Some(m), &tgt, None::<&str>, nix::mount::MsFlags::MS_BIND, None::<&str>)
                    .map_err(|e| format!("bind {m}: {e}"))?;
            }
            // ৫) pivot_root দিয়ে নতুন rootfs-এ যাও
            nix::unistd::pivot_root(rootfs, &rootfs.join("mnt")).map_err(|e| format!("pivot_root: {e}"))?;
            chdir("/").map_err(|e| format!("chdir: {e}"))?;

            // ৬) কন্টেইনার init চালাও (Waydroid/LXC এর init অথবা lxc-start)
            let init = CString::new("/sbin/lxc-start").unwrap();
            let args = [init.clone(), CString::new("-n").unwrap(), CString::new("android").unwrap(),
                        CString::new("-F").unwrap(), CString::new("--").unwrap(),
                        CString::new("/system/bin/init").unwrap()];
            let argv: Vec<*const libc::c_char> = args.iter().map(|a| a.as_ptr()).chain(std::iter::once(std::ptr::null())).collect();
            unsafe { libc::execv(init.as_ptr(), argv.as_ptr()); }
            unsafe { libc::_exit(127); }
        }
        ForkResult::Parent { child } => {
            // ৭) cgroup v2-তে কন্টেইনার সীমা লেখো
            let base = "/sys/fs/cgroup/onuron.slice/android";
            let _ = std::fs::create_dir_all(base);
            let _ = std::fs::write(format!("{base}/memory.max"), "3221225472");   // 3 GiB
            let _ = std::fs::write(format!("{base}/memory.swap.max"), "536870912"); // 512 MiB
            let _ = std::fs::write(format!("{base}/pids.max"), "4096");
            let _ = std::fs::write(format!("{base}/cgroup.procs"), child.to_string());
            Ok(())
        }
    }
}
```

### ১১.২ `android/setup_android_layer.sh` — আসল

```bash
#!/usr/bin/env bash
# android/setup_android_layer.sh — আসল Waydroid/LXC rootfs প্রস্তুতি (স্টাব নয়)
set -euo pipefail

ROOTFS=/data/android/rootfs
LXCNAME=android

mkdir -p /data/android/{rootfs,data,system}

if ! command -v lxc-start >/dev/null; then
  echo "[FATAL] LXC ইনস্টল নেই — apt install lxc lxc-templates" >&2; exit 1
fi
if ! command -v waydroid >/dev/null; then
  echo "[FATAL] waydroid ইনস্টল নেই — নিম্নে rootfs বুটস্ট্র্যাপ করুন" >&2; exit 1
fi

# ১) binderfs mount (kernel CONFIG_ANDROID_BINDERFS)
mkdir -p /dev/binderfs
mount -t binder binder /dev/binderfs

# ২) Waydroid image init (আসল system/vendor image)
if [ ! -f "$ROOTFS/system.img" ]; then
  waydroid init -s GAPPS || waydroid init
fi

# ৩) LXC কনফিগ (/etc/nilos/lxc/android.conf)
install -d /etc/nilos/lxc
cat > /etc/nilos/lxc/android.conf <<'EOF'
lxc.uts.name = android
lxc.net.0.type = none
lxc.mount.auto = proc:mixed sys:ro
lxc.cgroup2.memory.max = 3G
lxc.cgroup2.pids.max = 4096
lxc.apparmor.profile = unconfined
lxc.init.cmd = /system/bin/init
EOF

# ৪) কন্টেইনারে binder device pass করো
echo "b" > /dev/binderfs/binder-control 2>/dev/null || true

echo "[OK] Android compatibility layer প্রস্তুত (LXC: $LXCNAME)"
```

### ১১.৩ Binder শিম (`runtime/nilbinder`)

Kernel-এ `CONFIG_ANDROID_BINDER_DEVICES="binder,hwbinder,vndbinder"` আছে, `DeviceAccessPolicy` `/dev/binder*` allow করে; কিন্তু কোনো ইউজারস্পেস binder শিম নেই। **লক্ষ্য:** কন্টেইনার থেকে binder কল নিয়ে সেগুলো `nilprotocol` ফ্রেমে রূপান্তর করা।

```
runtime/nilbinder/           # নতুন crate
  Cargo.toml                 # libc, nilprotocol
  src/lib.rs                 # binder ioctl (BINDER_WRITE_READ) + ট্রান্সলেশন
```

রেফারেন্স: `/dev/binderfs/binder` খুলে `BINDER_WRITE_READ` ioctl-এর `binder_transaction_data` চক্র চালাও; প্যারসেল `flat_binder_object` দেখে Onuron সার্ভিসে (যেমন `audiod`, `powerd`) ম্যাপ করো। প্রোটোটাইপ বললেও এটা **আসল ioctl** — fabricated নয়।

### ১১.৪ nilagent-এর UI — সিমুলেটেড মার্কার সরানো

`android/agent/src/main.rs` আসল (`am start -n`, `am broadcast -a`, `pm list packages` শেল করে)। এটা রেখে শুধু non-Unix-এর "Not running on Unix" নোট রাখো — সেটাও সঠিক, কারণ এজেন্ট Linux-only।

### ১১.৫ `nilandroid-client` — সত্যিকারের ফ্রেম

```rust
// runtime/nilandroid-client/src/lib.rs — ফ্রেম পাঠাও, fabricated স্ট্রিং নয়
use nilprotocol::{Frame, MessageType};

pub fn start_activity(action: &str, uri: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        let mut s = UnixStream::connect("/run/nilos/android.sock").map_err(|e| e.to_string())?;
        let payload = serde_json::to_vec(&serde_json::json!({ "action": "START_ACTIVITY", "action_name": action, "uri": uri })).unwrap();
        let f = Frame::new(u16::from(MessageType::Custom(0x2100)), 1, payload);
        f.write_to(&mut s).map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(unix))]
    { Err("android bridge is Linux-only".into()) }
}
```
`nilandroidd`-এও `handle_ipc_request`-এ `Custom(0x2100)` হ্যান্ডল করে আসল `start_activity` কল করো — `Pong`/`OK` নয়।

### ১১.৬ android-host JNI — সিমুলেশন সরানো

- `android-host/src/camera.rs`: `VALID_BASELINE_JPEG` fallback **সরাও**; ফ্রেম না থাকলে `capture_real_frame_strict`-এর মতো ত্রুটি ফেরাও।
- `jni_bridge.rs`: System Surface callback-এ Rust frame buffer → আসল `ANativeWindow_lock/unlockAndPost` (AAudio/AHardwareBuffer) দিয়ে ব্লিট করো; বর্তমানে Java Canvas আঁকে — এটা সরিয়ে native frame path করো।
- `runtime/nilhal/src/backends/android.rs`: ৮৮% ব্যাটারি/hard-coded ডিভাইস সরাও; `AndroidBridgeClient` সত্যিই `/run/onuron/android_bridge.sock`-এ `nilprotocol` ফ্রেম পাঠাও।

**DoD (Android):** `lxc-ls -f` কন্টেইনার দেখায়; `nilandroid-client` দিয়ে `am start` আসল অ্যাপ নেয়; কন্টেইনার থেকে `/dev/input*`,`/dev/kmsg`,`/dev/mem` ব্লকড (`is_device_permitted` + আসল mount); `grep -r "Simulated Android" services runtime` → ০।

---

## ১২. Phase 9 — স্ট্যাটাস বার

বর্তমানে fabricated। NilHAL-এর Linux backend দিয়ে আসল সোর্স:

```rust
// shell/src/status.rs — আসল ইন্ডিকেটর
pub fn battery_percent() -> Option<u32> {
    let base = "/sys/class/power_supply";
    for e in std::fs::read_dir(base).ok()?.flatten() {
        let cap = e.path().join("capacity");
        if let Ok(s) = std::fs::read_to_string(&cap) {
            if let Ok(v) = s.trim().parse::<u32>() { return Some(v); }
        }
    }
    None // ডেটা না থাকলে None — বানানো ৮৮% নয়
}

pub fn charging() -> Option<bool> {
    let base = "/sys/class/power_supply";
    for e in std::fs::read_dir(base).ok()?.flatten() {
        let st = e.path().join("status");
        if let Ok(s) = std::fs::read_to_string(&st) {
            let s = s.trim().to_lowercase();
            return Some(s == "charging" || s == "full");
        }
    }
    None
}

pub fn signal_strength() -> Option<u32> {
    // modem absent হলে None; fabricated "▂▄▆█ (sim)" নয়
    for node in ["/sys/class/leds/wwan/brightness", "/dev/wwan0"] {
        if std::path::Path::new(node).exists() { /* modem-specific sysfs পড়ো */ }
    }
    None
}
```
UI-তে `None` হলে "—" দেখাও, fabricated মান নয়।

**DoD:** `grep -rn '"🔋 88%"\|(sim)' shell` → ০।

---

## ১৩. Phase 10 — ফ্ল্যাশিং

### ১৩.১ QEMU (dev/CI)

```bash
python build/mkdisk.py          # /data ext4 256 MB
python build/mkinitramfs.py --target qemu-x86_64
./build/qemu-boot.sh            # অথবা Windows: .\build\qemu-boot.ps1
python build/qemu-smoke.py      # headless boot smoke
python build/qemu-persistence-test.py
```

### ১৩.২ OnePlus 6/6T (`oneplus-fajita`) bare-metal

```bash
# ১) AArch64 release বিল্ড
cargo build --release --workspace --target aarch64-unknown-linux-musl
# ২) boot.img (DTB সহ, header v2)
python build/mkbootimg.py create \
  --kernel  out/oneplus-fajita/Image.gz \
  --ramdisk out/oneplus-fajita/initramfs.cpio.gz \
  --dtb     targets/oneplus-fajita/sdm845-oneplus-fajita.dtb \
  --cmdline "console=ttyMSM0,115200n8 root=/dev/ram0 rw init=/init" \
  --profile fajita -o out/oneplus-fajita/boot.img
# ৩) AVB (আসল Ed25519 signature)
python build/mkvbmeta.py create --image out/oneplus-fajita/system_a.img \
  -o out/oneplus-fajita/vbmeta_a.img --partition system
# ৪) flash (fastboot; বুটলোডার আনলক করা ডিভাইসে)
fastboot flash boot    out/oneplus-fajita/boot.img
fastboot flash vbmeta  out/oneplus-fajita/vbmeta_a.img
fastboot reboot
```

> **সতর্কতা (AGENTS.md §7):** একটাই রেফারেন্স ডিভাইস আগে স্থির করো (fajita)। "drivers/DTB আছে" মানে "hardware সাপোর্টেড" নয়। প্রতিটি bring-up ধাপে `dmesg`/UART লগ সংরক্ষণ করো।

### ১৩.৩ Galaxy S25 (Track B hosted lab)

```bash
cargo ndk -t arm64-v8a build --release -p android-host
cd android-host/app && ./gradlew assembleRelease
# OnuronOS.apk ইনস্টল → অ্যাপ চালালে native userspace Android surface-এ চলে
```

---

## ১৪. Phase 11 — যাচাই ম্যাট্রিক্স

| # | চেক | কমান্ড | প্রত্যাশিত |
|---|---|---|---|
| ১ | কোনো সিমুলেশন নেই | `grep -rInE 'SIMULATED\|is_simulated *= *true' nilinit services runtime shell apps` | release-এ ০ |
| ২ | বিল্ড গেট | `bash build/no-stub-gate.sh` | exit 0 |
| ৩ | ইউনিট/ইন্টিগ্রেশন | `cargo test --workspace` | সব পাস |
| ৪ | QEMU বুট | `python build/qemu-smoke.py` | boot-complete |
| ৫ | পার্সিস্টেন্স | `python build/qemu-persistence-test.py` | /data অটুট, reboot-এর পরেও টিকে |
| ৬ | রিকভারি | খারাপ বুট ৩× → `nilrecovery` মেনু | স্বয়ংক্রিয় recovery |
| ৭ | DRM | `dmesg \| grep drm` + পর্দায় সলিড রঙ | mode সেট+flip |
| ৮ | টার্মিনাল | nilterminal → `ps`, `cat /proc/uptime` | আসল PID/আপটাইম |
| ৯ | ব্রাউজার | আসল URL (WPE) | পেজ রেন্ডার |
| ১০ | Android কন্টেইনার | `lxc-ls -f`, `am start` | কন্টেইনার + অ্যাপ |
| ১১ | Binder | `/dev/binderfs/binder` ioctl | write/read সফল |
| ১২ | স্ট্যাটাস বার | battery/signal sysfs | আসল মান/— |
| ১৩ | সিকিউরিটি | sandbox+seccomp+permbroker | বাইরে যেতে পারে না |

**রিলিজ ঘোষণা করার শর্ত (AGENTS.md §16 ধাঁচে):** "এই ফোনে অনুরণ বুট হয়" বলতে হলে উপরের ১–৬ + ৭–১৩ ডিভাইসে পাস লাগবে, এবং UART/`dmesg` লগ সংযুক্ত করতে হবে। প্রমাণ ছাড়া দাবি নিষিদ্ধ।

---

## ১৫. কাজের ক্রম (Task Ledger)

1. **P0:** `build/no-stub-gate.sh` + CI + `maturity.toml` সততা।
2. **P1:** সকেট-পাথ বাগ সারানো (§৫.২) — সব ডেমন ট্রাফিক পায় কিনা যাচাই।
3. **P2:** `App::run` আসল ইভেন্ট লুপ (§১০)।
4. **P3:** DRM/KMS আসল ioctl (§৬) — পর্দায় প্রথম পিক্সেল।
5. **P4:** `inputd` আসল evdev (§৭)।
6. **P5:** `nilterminal` আসল PTY+VT (§৮)।
7. **P6:** `nilbrowser` WPE (§৯)।
8. **P7:** প্রতিটি অ্যাপের fabricated পাথ অপসারণ (§১০)।
9. **P8:** স্ট্যাটাস বার আসল (§১২)।
10. **P9:** `nilandroidd` আসল কন্টেইনার + binder শিম (§১১)।
11. **P10:** fajita bring-up (§১৩) + যাচাই ম্যাট্রিক্স (§১৪)।

**PR নিয়ম:** প্রতিটি PR = একটি সাবসিস্টেম; সাথে টেস্ট + একটি reproducible প্রমাণ (লগ/fioctl ট্রেস) + সংশ্লিষ্ট `docs/` আপডেট। "ডিরেক্টরি আছে" ≠ "কাজ করে"।

---

## শেষ কথা

প্রশ্ন: *"এই পরিবর্তন কি অনুরণ ওএসকে স্বাধীন Linux-ভিত্তিক মোবাইল OS হিসেবে শক্তিশালী করে — নাকি চুপিচুপি অন্য কিছু বানিয়ে ফেলছে (যেমন সিমুলেটর, বা AOSP ROM)?"* — যদি "অন্য কিছু" হয়, থামো এবং দ্বন্দ্ব ডকুমেন্ট করো।

এই ফাইলের প্রতিটি 🔴/🟡 এন্ট্রি 🔵/🟢 হওয়ার আগে "কোড আছে" কে "কাজ করে ও প্রমাণিত" ভাবা যাবে না।
