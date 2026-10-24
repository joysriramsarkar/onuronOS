# OnuronOS — সম্পূর্ণ OS নির্দেশিকা (বুট থেকে অ্যাপ পর্যন্ত)

এই একটি ফাইলেই সম্পূর্ণ নির্দেশনা: বুট চেইন, টার্মিনাল, ব্রাউজার, ইনপুট/টাচ,
অ্যান্ড্রয়েড ব্রিজ, বিল্ড, QEMU বুট, আসল ডিভাইসে বুট, এবং টেস্ট।

---

## ০. আগে সৎ কথা — কী বাস্তবে আছে, কী নেই

AGENTS.md-এর "Reality / maturity rule" অনুযায়ী প্রতিটি জিনিস সৎভাবে
শ্রেণীবদ্ধ করা বাধ্যতামূলক। তাই আগে এটা পড়ুন, তারপর কাজ শুরু করুন।

**এই সেশনে বাস্তবে লেখা ও টেস্ট করা হয়েছে (কোনো স্টাব নেই):**

| উপাদান | ফাইল | অবস্থা | টেস্ট |
|---|---|---|---|
| evdev টাচ/কী ইনপুট | `services/inputd/src/evdev.rs` | IMPLEMENTED | ১০ |
| টার্মিনাল কমান্ড এক্সিকিউশন | `shell/src/term.rs` | IMPLEMENTED | ১৪ |
| ব্রাউজার (HTTP/HTTPS) | `runtime/nilweb/src/lib.rs` | IMPLEMENTED | ১০ |
| অ্যান্ড্রয়েড কন্টেইনার লঞ্চ | `services/nilandroidd/src/container.rs` | IMPLEMENTED | ১৫ |
| অ্যান্ড্রয়েড ক্লায়েন্ট | `runtime/nilandroid-client/src/lib.rs` | IMPLEMENTED | — |
| Wayland সার্ভার (হাতে লেখা প্রোটোকল) | `runtime/nilui-gpu/src/wayland/` | IMPLEMENTED | ৭৩ |
| কম্পোজিটর + ক্লায়েন্ট বাইনারি | `runtime/nilui-gpu/src/bin/wayland_*.rs` | IMPLEMENTED | ২ (cross-process) |

মোট নতুন কোড ≈ ৫৫০০ লাইন। উপরের টেবিলের টেস্ট যোগফল ১২৪টি; সবগুলো পাস
(`cargo test -p nilui-gpu` → lib-এ ১০১ + cross-process ২)। সম্পূর্ণ workspace
মিলিয়ে ৪৭৭টি টেস্ট পাস।

**যা এখনও বাস্তবে নেই (এবং এই গাইড সেটা লুকাবে না):**

- **অ্যান্ড্রয়েড গেস্ট ইউজারস্পেস।** কন্টেইনার চালানোর ইঞ্জিন বাস্তব, কিন্তু
  Android-এর নিজের `system/` ইমেজ OnuronOS-এর সাথে আসে না। আপনাকে আলাদা করে
  একটি rootfs বসাতে হবে (§৭)। rootfs না থাকলে UI সৎভাবে `UNAVAILABLE` দেখাবে —
  ভুয়া `RUNNING` দেখাবে না।
- **ব্রাউজারে CSS/JavaScript/ছবি রেন্ডার হয় না।** এটা টেক্সট-মোড ব্রাউজার।
  এটা UI-তেই লেখা আছে। বাস্তব HTTP ফেচ, TLS, রিডাইরেক্ট, টাইটেল — এসব আসল।
- **Android UI স্ক্রিন।** কন্টেইনারের আসল অবস্থা দেখায়, কিন্তু Android-এর
  গ্রাফিক্স পাসথ্রু নেই, তাই Android-এর নিজের UI দেখা যায় না। তাই এই স্ক্রিনটির
  গায়ে `[SIMULATED]` ব্যাজ থাকে (`docs/maturity.toml`)।
- **`android-host/` (S25 হোস্টেড রানটাইম)** আলাদা সাবসিস্টেম, এটা এখানে
  পরিবর্তন করা হয়নি।
- **কম্পোজিটরে `linux-dmabuf`/GPU বাফার নেই, `xdg-shell` নেই।** Wayland সার্ভার
  এখন বাস্তব (§৬.৭), কিন্তু অ্যান্ড্রয়েডের SurfaceFlinger বাফার সরাসরি নিতে
  পারবে না — প্রতিটি ফ্রেম CPU-তে কপি করতে হবে। stock GTK/Qt অ্যাপও চলবে না।
  এটাই পরের বড় কাজ।

---

## ১. আর্কিটেকচার — কোন স্তর কার

OnuronOS AOSP fork নয়। এটি স্বাধীন Linux-ভিত্তিক মোবাইল OS।

```
Linux LTS 6.6  (kernel)
      ↓
nilinit        (PID 1, 서비스 supervision, mount, cgroup, SELinux)
      ↓
NilHAL         (হার্ডওয়্যার অ্যাবস্ট্রাকশন সীমানা)
      ↓
Onuron সিস্টেম সার্ভিস (services.toml-এ ১৮টি supervised daemon: nild, netd, audiod, inputd, camerad, ...)
      ↓
runtime/nilrt  (অ্যাপ isolation)
      ↓
NilLang + Alap + NilUI
      ↓
স্বাক্ষরিত .nilax অ্যাপ্লিকেশন
```

Android সামঞ্জস্য একটি **আলাদা, বিচ্ছিন্ন** সাবসিস্টেম (`android/`,
`android-host/`)। এটা কখনোই native platform-এর ভিত্তি নয়।

**সোনালি নিয়ম:** native অ্যাপ হার্ডওয়্যার সরাসরি স্পর্শ করে না। সে NilHAL ও
সার্ভিস সীমানা ব্যবহার করে।

---

## ২. প্রয়োজনীয় টুলচেইন

```bash
# Rust (Rust-first userspace)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# cross-compile targets (aarch64 ডিভাইসের জন্য)
rustup target add aarch64-unknown-linux-musl
rustup target add x86_64-unknown-linux-musl

# QEMU (x86_64 dev)
sudo apt-get install -y qemu-system-x86 qemu-system-arm

# মোবাইল ফ্ল্যাশিং টুল (আসল ডিভাইসের জন্য)
sudo apt-get install -y android-tools-adb android-tools-fastboot

# Python (বিল্ড স্ক্রিপ্ট)
python3 --version   # 3.8+
```

যাচাই:

```bash
rustc --version && cargo --version && qemu-system-x86_64 --version
```

---

## ৩. বুট চেইন — ধাপে ধাপে

### ৩.১ ফার্মওয়্যার → kernel → nilinit

1. **Bootloader** kernel ও initramfs লোড করে।
2. **kernel** initramfs আনপ্যাক করে `/init` চালায় — সেটি `nilinit`।
3. **nilinit** (`nilinit/src/main.rs`, ≈৮৭০ লাইন) এই কাজগুলো করে:
   - essential mount: `/proc`, `/sys`, `/dev`, `/run`
   - cgroup v2 চালু
   - SELinux policy লোড
   - `etc/nilos/services.toml` পড়ে সার্ভিস চালু করে
   - প্রতিটি সার্ভিস supervise করে (ক্র্যাশ হলে `restart = "always"` অনুযায়ী
     আবার চালায়)
   - সার্ভিস প্রস্তুত হলে তার socket-এ notification দেয়

### ৩.২ সার্ভিস টপোলজি

`etc/nilos/services.toml` হলো canonical তালিকা:

```toml
version = 1

[[services]]
name = "nild"
exec = "/usr/bin/nild"
restart = "always"

[[services]]
name = "inputd"
exec = "/usr/bin/inputd"
restart = "always"
socket_activation = "/run/nilos/input.sock"
```

নতুন কোনো daemon যোগ করলে **দুই জায়গায়** নাম দিতে হবে, নইলে সেটি ইমেজে
যাবে না:

1. `etc/nilos/services.toml` — supervisor চালু করবে
2. `build/build.sh` ও `build/mkinitramfs.py`-এর `BINS`/`bins` তালিকা — বাইনারি
   rootfs-এ কপি হবে

> **এই সেশনে ধরা পড়া আসল বাগ:** `build/build.sh`-এর তালিকায় `inputd` ছিল না,
> অর্থাৎ ডিভাইসে টাচ ইনপুট বাইনারিটিই যেত না। এটি ঠিক করা হয়েছে (২২টি অনুপস্থিত
> বাইনারি যোগ করা হয়েছে)।

### ৩.৩ বুট যাচাই

```bash
# emulator-এ বুট করে দেখুন
./build/qemu-boot.sh

# অথবা smoke test
python3 build/qemu-smoke.py
```

---

## ৪. টার্মিনাল — বাস্তব কমান্ড এক্সিকিউশন

আগে টার্মিনাল একটি `match` ব্লক থেকে বানানো উত্তর দিত — `ps`, `services`
ইত্যাদির আউটপুট কৃত্রিম ছিল। এখন প্রতিটি কমান্ড সত্যিকারভাবে চলে।

### ৪.১ কীভাবে কাজ করে

`shell/src/term.rs`:

- **builtin** (shell-এর নিজের state বদলায়, তাই প্রসেসে চলে):
  `cd`, `pwd`, `export`, `unset`, `echo`, `which`, `help`, `clear`, `exit`
- **বাকি সব** — `$PATH`-এ খোঁজা হয়, তারপর সত্যিকারের child process হিসেবে
  `Command` দিয়ে চালানো হয়, stdin/stdout/stderr সংযুক্ত করা হয়।

নিরাপত্তা সীমা:

```rust
/// Longest a single command may run before the terminal reclaims the prompt.
/// Without this a `cat` on a FIFO would freeze the whole UI.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_OUTPUT_BYTES: usize = 64 * 1024;
```

### ৪.২ উদ্ধৃতি ও escape পার্সিং

```rust
pub fn split_args(line: &str) -> Vec<String>
```

`echo "a b" c` → `["echo", "a b", "c"]`। `\ ` escape কাজ করে।

### ৪.৩ `cd`-এর সূক্ষ্ম দিক

`std::fs::canonicalize` একা ব্যবহার করা **যাবে না** — যে পথ এখনও নেই তার উপর
সেটি ব্যর্থ হয়, ফলে `/data/..` হিসাব করা যায় না। তাই আগে lexically `.`/`..`
সমাধান করা হয় (`normalize_logical`), তারপর পথটি সত্যিই থাকলে physical পথ
নেওয়া হয়:

```rust
let logical = normalize_logical(&joined);
let resolved = std::fs::canonicalize(&logical).unwrap_or(logical);
match std::fs::metadata(&resolved) { ... }
```

### ৪.৪ ব্যবহার

```
OnuronOS terminal — real command execution (see 'help')

> help
> ls /data
> cat /proc/version
> export FOO=bar
> sh -c "echo $FOO"
> which sh
> cd /tmp
> exit
```

শেষ কমান্ডের exit status verbatim দেখানো হয়; কমান্ড না থাকলে `127` ও
`command not found`।

### ৪.৫ সীমাবদ্ধতা (UI-তেও লেখা)

pipeline (`|`), redirection (`>`), globbing, substitution — এগুলো নেই।
`ls | wc` না লিখে সরাসরি `ls /data` লিখুন।

---

## ৫. ব্রাউজার — বাস্তব HTTP/HTTPS

ইঞ্জিন: `runtime/nilweb/src/lib.rs`। হোম স্ক্রিনে `[14] 🌐 Browser`।

একই ইঞ্জিন দুটি জায়গায় ব্যবহৃত হয় — শেলের Browser স্ক্রিন এবং আলাদা
`browser` অ্যাপ (`apps/browser/`)। দুটোই একই `nilweb` কোড চালায়, তাই
আচরণ সবসময় এক।

> **নোট (২০২৬-১০):** আগের `apps/nilbrowser` ক্রেটটি বাদ দেওয়া হয়েছে।
> সেটি `wry`/`tao` (WebKitGTK) ব্যবহার করত — ডেস্কটপ GTK-নির্ভর, মোবাইলে
> (DRM-only) চলে না, আর Linux/ARM-এ বিল্ডই হত না (glib-2.0 dev header লাগত)।
> এখন ব্রাউজার = `nilweb` টেক্সট-মোড ইঞ্জিন, যা আসল HTTP/HTTPS করে।

### ৫.১ আসলে যা করে

- **সত্যিকারের নেটওয়ার্ক রিকোয়েস্ট** — `ureq` + `rustls` (TLS)।
- আসল status code, content-type, byte দৈর্ঘ্য দেখায়।
- `<title>` থেকে শিরোনাম বের করে।
- HTML → টেক্সট: `<script>`/`<style>`-এর **ভেতরের লেখা বাদ** দেওয়া হয়,
  block ট্যাগ newline হয়, entity decode হয়।
- বাউন্ডেড রিডাইরেক্ট ফলো (HTTP client করে)।

### ৫.২ সীমা

```rust
const MAX_BODY_BYTES: usize = 512 * 1024;   // এক পাতায় মেমরি শেষ হবে না
const MAX_RENDER_LINES: usize = 400;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
```

### ৫.৩ ব্যবহার

```
url> example.com          → https://example.com (স্বয়ংক্রিয়ভাবে scheme যোগ)
url> https://example.com/x
url> rust programming     → DuckDuckGo সার্চ
url> d                    → নিচে স্ক্রল
url> u                    → উপরে স্ক্রল
url> top                  → একদম উপরে
url> back                 → হোম স্ক্রিনে ফেরা
```

পাতা খুলতে ব্যর্থ হলে UI সৎভাবে `✕ network error: ...` দেখাবে এবং netd
চালু আছে কিনা যাচাই করতে বলবে — ভুয়া "loaded" দেখাবে না।

### ৫.৪ যা রেন্ডার হয় না

CSS layout, JavaScript, ছবি। JavaScript-নির্ভর পাতা প্রায় খালি দেখাবে; তখন
ব্রাউজার নিজেই বলে দেয় "This page produced almost no text; it likely requires
JavaScript."

---

## ৬. ইনপুট / টাচ — evdev

আগে `inputd` শুধু `/dev/input`-এর ফাইল **তালিকা** করত, একটি event-ও পড়ত না।
অর্থাৎ আসল ফোনে টাচ কাজ করত না। এখন `services/inputd/src/evdev.rs` আসল
`struct input_event` পড়ে।

### ৬.১ গুরুত্বপূর্ণ kernel ABI তথ্য

- `struct input_event` **৬৪-বিটে ২৪ বাইট**, ৩২-বিটে ১৬ বাইট। ভুল হলে প্রতিটি
  event ভুলভাবে পড়া হবে — তাই এটা টেস্টে pin করা আছে।
- ioctl মান (আসল `<linux/input.h>`-এর সাথে মিলিয়ে দেখা হয়েছে):

```
EVIOCGBIT(0,8)  == 0x80084520
EVIOCGABS(0)    == 0x80184540
EVIOCGNAME(256) == 0x81004506
```

### ৬.২ ডিজাইন

- প্রতিটি ডিভাইস `O_RDONLY|O_NONBLOCK`-এ খোলা হয়, তারপর `poll(2)` — busy loop নেই।
- ডিভাইস চেনা হয় **EVIOCGBIT** দিয়ে, নাম দিয়ে নয়।
- axis range **EVIOCGABS** থেকে — তাই ০..৪০৯৫ রিপোর্ট করা digitizer সঠিকভাবে
  scale হয়, পিক্সেল ধরে নেওয়া হয় না।
- Multi-Touch Protocol B (`ABS_MT_SLOT`/`ABS_MT_TRACKING_ID`) ও legacy
  single-touch (`ABS_X/ABS_Y` + `BTN_TOUCH`) — দুটোই।

### ৬.৩ gesture

`Tap`, `DoubleTap`, `LongPress`, `Swipe`, `Drag` — `GestureRecognizer`।
চেনা event গুলো IPC queue-তে যায়, তাই `InputPollEvents` ক্লায়েন্ট আসল ডিভাইস
ইনপুট দেখে।

### ৬.৪ rotation

`TouchCalibration::transform` — Portrait / LandscapeLeft / UpsideDown /
LandscapeRight। raw coordinate → display coordinate।

---

## ৬.৫ ডিসপ্লে — DRM/KMS (আসল ioctl)

`runtime/nilui-gpu`। এটাই আসল পর্দার ভিত্তি — Wayland passthrough-ও এর উপর
দাঁড়াবে।

**আগে যা ভাঙা ছিল (গুরুতর):** `drm.rs` শুধু `/dev/dri/card0` খুলত, তারপর
**একটাও ioctl দিত না**। `set_crtc` আর `page_flip` শুধু `fb_id != 0` দেখে
`Ok` ফেরাত। অর্থাৎ `docs/maturity.toml`-এ "Direct /dev/dri/card0 modesetting,
dumb buffer allocation" লেখা থাকলেও পর্দায় কিছুই যেত না। AGENTS.md-এর নিয়ম
#৬ (প্রমাণ ছাড়া hardware দাবি নিষিদ্ধ) ভাঙা হচ্ছিল।

**এখন যা করে** (`drm.rs` ≈১১০০ লাইন, `ioctl.rs`, `drm_uapi.rs`):

| ধাপ | ioctl |
|---|---|
| card খোঁজা | `/dev/dri/card*` |
| resource | `GETRESOURCES` |
| connector / encoder / CRTC | `GETCONNECTOR`, `GETENCODER`, `GETCRTC` |
| master নেওয়া | `SET_MASTER` |
| buffer | `CREATE_DUMB` → `MAP_DUMB` → `ADDFB2` → `mmap` |
| mode সেট | `SETCRTC` |
| frame দেখানো | `PAGE_FLIP` + `DRM_EVENT_FLIP_COMPLETE`-এর জন্য block |

শেষ ধাপটাই tear-free করে: `PAGE_FLIP` শুধু queue করে, তারপর kernel যে
event দেয় সেটার জন্য অপেক্ষা না করলে "সফল" বলাটা মিথ্যা হত।

### যাচাই করা হয়েছে — কীভাবে

তিনটি স্বাধীন স্তর, প্রতিটিই টেস্টে pin করা:

1. **ioctl নম্বর** — `ioctl.rs`-এর টেস্টগুলো kernel header থেকে কম্পাইল করা
   C প্রোগ্রামের মানের সাথে মেলায় (`0xc04064a0` ইত্যাদি)।
2. **struct layout** — `drm_uapi.rs`-এ `const _: () = { ... }` দিয়ে size ও
   field offset compile-time-এ যাচাই (`drm_mode_crtc` = 104, `name` offset
   = 36 ...)। layout বদলালে build-ই ভাঙবে।
3. **ioctl সত্যিই যায় কি** — `opening_a_non_drm_file_reports_enotty_not_fake_success`
   টেস্টটি একটি সাধারণ ফাইলকে DRM node বলে খুলতে চায় এবং `ENOTTY` আশা করে।
   পুরোনো stub এখানে "সফল" বলত; এটিই সেই টেস্ট যা bug ধরত।

### যা এখনো যাচাই হয়নি — সৎভাবে

**কোনো আসল পর্দায় এটি চালানো হয়নি।** dev container-এ `/dev/dri` নেই,
আর `vkms` লোড করাও ব্লকড (`modprobe: Operation not permitted`)। তাই
discovery, `SETCRTC`, VBLANK wait — কোনোটা আসল driver-এর বিরুদ্ধে চলে না
দেখা। ডিভাইসে বুট করে একটা frame না দেখা পর্যন্ত এটাকে
"hardware-validated" বলবেন না।

ডিসপ্লে না থাকলে `DrmDevice` সৎভাবে `offscreen` backend রিপোর্ট করে,
`page_flip_success = false` দেয়, আর `present_demo` / `bootsplash` /
`touch_compositor` exit code **২** দিয়ে বেরোয় — ভুয়া "[SUCCESS] tear-free"
ছাপে না।

---

## ৬.৬ স্ট্যাটাস বার — হার্ডওয়্যার সরাসরি পড়া বন্ধ (সার্ভিস সীমান্ত)

**আগে যা ভাঙা ছিল (আর্কিটেকচারাল):** shell নিজেই
`/sys/class/power_supply/battery/capacity` আর `/sys/class/net` পড়ত। AGENTS.md
§৬ স্পষ্ট বলছে: *"Native applications should not access hardware directly.
They should use stable Onuron APIs and service boundaries."* অর্থাৎ UI স্তর
সরাসরি হার্ডওয়্যার পড়ছিল, আর interface নামকরণের নিয়ম (`wl*`, `rmnet*`) UI-এর
ভেতরে আটকে ছিল। ব্যাটারি ও নেটওয়ার্কের মালিক দুইটি ডেমন — `powerd` আর
`netd` — কিন্তু তারা ইতিমধ্যে IPC-তে ডেটা দিচ্ছিল, কেউ সেটা ব্যবহার করছিল না।

**তিনটি আসল সমস্যা:**

1. **contract crate-এ type ছিল না।** `BatteryInfo` আর `NetworkState` দুইটি
   binary crate-এর `main.rs`-এ ঘোষিত ছিল। Binary crate-এর ভেতরের type অন্য
   কেউ নাম ধরে ডাকতে পারে না, তাই shell-এর কাছে IPC ব্যবহার করার উপায়ই ছিল না।
2. **ডেমন ব্যর্থ হয়ে "সফল" বলত।** `powerd`/`netd` socket bind fail করলেও
   `[  OK  ] ... daemon active` ছাপত। bind-টা worker thread-এ হয়, আর তার
   ফল কেউ দেখত না। ফলে `/run/onuron` না থাকলে ডেমন "চলছে" বলে মনে হত, অথচ
   কোনো client পৌঁছাতে পারত না।
3. **সিমুলেটেড payload চিহ্নিত ছিল না।** হার্ডওয়্যার না থাকলে ফেরত দেওয়া
   মানকে "আসল পড়া" বলে দেখানোর কোনো বাধা ছিল না।

**এখন যা করে:**

| স্তর | কী |
|---|---|
| contract | `protocol/src/lib.rs`-এ `BatteryInfoPayload`, `NetworkStatePayload`, `NetworkInterfacePayload`, `ConnectionType` |
| ডেমন | `powerd` ও `netd` ওগুলো re-export করে; bind ব্যর্থ হলে `[ FAIL ]` + `notify_ready` বন্ধ |
| client | `runtime/nilpower` (`PowerClient`), `runtime/nilnet` (`NetClient`) |
| UI | shell কেবল client ডাকে; `/sys/class/power_supply` ও `/sys/class/net` পড়া পুরো বাদ |

দুইটি accessor আছে, ইচ্ছাকৃতভাবে:

- `battery_info()` / `state()` — **infallible**। ডেমন না থাকলে flagged
  simulated মান দেয়, যাতে UI সবসময় কিছু আঁকতে পারে। **কিন্তু** UI-কে অবশ্যই
  `is_simulated` দেখিয়ে `[SIMULATED]` ব্যাজ বসাতে হবে।
- `try_battery_info()` / `try_state()` — **strict**। ডেমন না থাকলে `Err`।
  যেখানে placeholder চলবে না (boot check, টেস্ট) সেখানে এটাই ব্যবহার করুন।

`nilnet`-এর fallback ইচ্ছাকৃতভাবে **নৈরাশ্যবাদী**: `NetworkStatePayload::default()`
`is_connected = false` দেয়। যে সংযোগ নেই সেটা "আছে" বলা দেখানোর চেয়ে খারাপ
কিছু নেই।

### যাচাই করা হয়েছে — কীভাবে

এখানে mock নেই। চারটি integration টেস্ট **আসল ডেমন বাইনারি চালু করে**,
তারপর **আসল client** দিয়ে পড়ে:

```bash
# nilpower: আসল powerd spawn করে socket-এ পড়ে
cargo test -p nilpower --test powerd_ipc -- --nocapture
# → OK: capacity=100% status=Full simulated=false voltage=4000mV temp=25C

# nilnet: আসল netd spawn করে host-এর আসল interface পড়ে
cargo test -p nilnet --test netd_ipc -- --nocapture
# → OK: connected=false active=None type=None interfaces=2 dns=1
```

টেস্ট নিজেই ব্যক্তিগত socket বানায় (env `ONURON_POWER_SOCKET` /
`ONURON_NET_SOCKET` দিয়ে), তাই parallel টেস্টে সংঘর্ষ হয় না; `DaemonGuard`
drop-এ প্রসেস kill করে socket মুছে দেয়। বাইনারি build না থাকলে টেস্ট সৎভাবে
SKIP বলে, fail করে না। এছাড়া "ডেমন নেই" অবস্থায় strict accessor `Err` দেয়
কি, আর fallback flagged simulated হয় কি — আলাদা টেস্টে pin করা।

**যা এখনো যাচাই হয়নি:** এই dev container-এ আসল ব্যাটারি/মডেম নেই, তাই
`powerd` যা ফেরত দেয় তা `nilhal`-এর QEMU path। আসল ব্যাটারিতে পড়া যায় কি
না তা ডিভাইসে বুট করে দেখতে হবে।

---

## ৬.৭ Wayland কম্পোজিটর — হাতে লেখা প্রোটোকল (nilui-gpu)

আগের অবস্থা: DRM/KMS দিয়ে সরাসরি পিক্সেল স্ক্যানআউট হতো, কিন্তু **বাইরের কোনো
প্রসেস তার নিজের জানালা পাঠাতে পারত না**। `wl_surface` ছিল শুধু কাগজে — ডকে
"planned, not written" লেখা। এখন সেটি বাস্তব।

### ৬.৭.১ কেন libwayland ব্যবহার করা হয়নি

তিনটি কারণ, একটাই সিদ্ধান্ত:

1. `libwayland-server`-এর হেডার এই পরিবেশে নেই, আর সেগুলো থাকলেও `pkg-config`
   দিয়ে build গাঁথতে হতো।
2. `deny.toml`-এ `unmaintained = "all"` একটি release gate — বাইরের Wayland
   ক্রেট যোগ করলে সেটি ভাঙার ঝুঁকি।
3. release build musl দিয়ে static link হয়; C লাইব্রেরি টানলে সেটি জটিল হয়।

একই কারণে `drm.rs` নিজেই ioctl হাতে লেখে। তাই এখানেও ওয়্যার প্রোটোকল সরাসরি
লেখা হয়েছে — কোনো বাইরের Wayland ক্রেট নেই, শুধু `std` + `libc`।

### ৬.৭.২ ফাইল কাঠামো

| ফাইল | কাজ |
|---|---|
| `wayland/wire.rs` | মেসেজ ফ্রেমিং, স্ট্রিং/অ্যারে প্যাডিং, fd আর্গুমেন্ট |
| `wayland/protocol.rs` | অবজেক্ট মডেল, opcode টেবিল, ইভেন্ট বিল্ডার |
| `wayland/shm.rs` | `memfd_create`, read-only mmap, পুল/বাফার যাচাই |
| `wayland/server.rs` | Unix socket সার্ভার, per-client থ্রেড, SCM_RIGHTS |
| `wayland/compose.rs` | কমিট করা সারফেস → পিক্সেল বাফার (স্কেল, alpha blend) |
| `wayland/client.rs` | `SurfaceProducer` — যে কেউ পিক্সেল পাঠাতে চায় তার API |
| `bin/wayland_compositor.rs` | চালানোর মতো কম্পোজিটর বাইনারি |
| `bin/wayland_surface_producer.rs` | চালানোর মতো ক্লায়েন্ট বাইনারি |

### ৬.৭.৩ ওয়্যার ফরম্যাট — যে তিনটি ফাঁদে সবাই পড়ে

Wayland মেসেজে তিনটি সূক্ষ্ম নিয়ম আছে; ভুল করলে ক্লায়েন্ট চুপচাপ ভুল পড়ে:

```rust
// ১. স্ট্রিং সবসময় NUL-terminated এবং ৪ বাইটে প্যাডেড।
//    "ab" → দৈর্ঘ্য-হেডার 3 (a, b, \0), তারপর ১ বাইট প্যাড।
// ২. অ্যারে দৈর্ঘ্য বাইটে, তারপর ৪ বাইটে প্যাডেড।
// ৩. fd ওয়্যার বাইটে শূন্য (0) — আসল fd যায় SCM_RIGHTS দিয়ে,
//    অর্থাৎ মেসেজের সাথে নয়, socket-এর control buffer-এ।
```

শেষ নিয়মটি সবচেয়ে বেশি ভুল হয়। `put_fd` তাই বাইটে 0 লেখে, আর fd-টি আলাদা
কিউতে রাখে যা `sendmsg`-এর `SCM_RIGHTS`-এ যায়:

```rust
// wayland/wire.rs (সারাংশ)
pub fn put_fd(&mut self, fd: RawFd) -> &mut Self {
    // ওয়্যারে fd-এর জায়গা শূন্য; আসলটি control message-এ যাবে।
    self.put_uint(0);
    self.fds.push(fd);
    self
}
```

### ৬.৭.৪ কমিট → কম্পোজিট → DRM: পুরো পথ

```
ক্লায়েন্ট                        সার্ভার
  |  attach(buffer) + commit()      |
  |------------------------------->|  সারফেসের পিক্সেল পড়ে (mmap)
  |                                |  SurfaceFrame বানায়
  |  <--- wl_buffer.release -------|  (অর্থাৎ পড়া শেষ)
  |                                |
  |                     কম্পোজিটর: composite() → PixelBuffer
  |                                |  KmsPresenter.present_frame()
  |                                |  → SETCRTC / page flip (আসল কার্ডে)
```

`wl_buffer.release` পাঠানোটা **জরুরি**, শুধু ভদ্রতা নয়: ক্লায়েন্ট একটি বাফার
বারবার ব্যবহার করে, তাই release না পেলে সে জানতেই পারে না কম্পোজিটর পড়া শেষ
করেছে কি না। এই সিকোয়েন্সটি টেস্টে pin করা আছে।

### ৬.৭.৫ ব্লক করা, পোল করা নয় — eventfd

প্রথম সংস্করণে কম্পোজিটর প্রতি ১৬ms-এ ঘুমিয়ে জেগে দেখত "নতুন কিছু আছে কি"।
এটি একটি **আসল বাগ** ছিল, এবং cross-process টেস্টে ধরা পড়েছে: ক্লায়েন্ট কমিট
করে সাথে সাথে সারফেস ধ্বংস করে দিলে ফ্রেমটি কখনো আঁকা হতো না — ১৬ms উইন্ডোতে
সেটি হারিয়ে যেত।

সমাধান: `eventfd`। প্রতি commit-এ সার্ভার একটি কাউন্টার বাড়ায়; কম্পোজিটর
`poll()` করে সেটির উপর ব্লক করে:

```rust
// server.rs — সারফেস commit হলে presenter-কে জাগানো
if wake_pending {
    wake.signal();   // eventfd-তে 1 লেখে; কখনো ব্লক করে না
}
```

```rust
// wayland_compositor.rs — ১০০ms টাইমআউটে poll, যাতে Ctrl-C কাজ করে
let rc = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 100) };
if fds[0].revents & libc::POLLIN != 0 {
    wake.wait()?;    // কাউন্টার শূন্য করে, পরের poll আবার ব্লক করবে
}
```

### ৬.৭.৬ একটি বাস্তব প্রোটোকল বাগ — বাফার টিকে থাকা

টেস্ট একটি সূক্ষ্ম ভুল ধরেছে যা হাতে ধরার উপায় ছিল না। Wayland-এ `attach` করা
বাফার **পরবর্তী commit-গুলোতেও কার্যকর থাকে**; কেবল নতুন `attach` (বা null
attach) সেটি বদলায়। সার্ভার প্রথমে `mem::take` দিয়ে বাফার তুলে নিত, ফলে যে
ক্লায়েন্ট একবার attach করে বহু ফ্রেম commit করে সে কেবল **প্রথম ফ্রেমটিই**
আঁকা হতো:

```rust
// ভুল — প্রতিটি commit-এ বাফার খালি হয়ে যায়
let pending = std::mem::take(&mut s.pending_buffer);

// ঠিক — বাফার বর্তমান থাকে, কেবল attach বদলায়
let pending = self.surfaces.get(&surface).and_then(|s| s.current_buffer);
```

এটি ঠিক করার আগে `a_second_commit_replaces_the_first` টেস্ট fail করছিল, এখন
পাস করে। এজন্যই টেস্ট লেখা — নয়তো ডিভাইসে গিয়ে "কেন শুধু প্রথম ফ্রেম দেখছি"
খুঁজতে হতো।

### ৬.৭.৭ চালানো

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export XDG_RUNTIME_DIR=/run/onuron
mkdir -p $XDG_RUNTIME_DIR

# কম্পোজিটর (টার্মিনাল ১)
cargo run -p nilui-gpu --bin wayland_compositor -- --width 1080 --height 2400

# ক্লায়েন্ট (টার্মিনাল ২) — ৫টি ফ্রেম পাঠায়
cargo run -p nilui-gpu --bin wayland_surface_producer -- \
    --width 256 --height 256 --frames 5
```

কম্পোজিটরের আসল আউটপুট এইরকম:

```
[ wayland_compositor ] Onuron Wayland compositor 1080x2400
[   OK   ] listening on /run/onuron/wayland-0
[   OK   ] global wl_compositor v4 (1)
[   OK   ] global wl_shm v1 (2)
[  INFO  ] display backend: offscreen_fallback (/dev/dri/card0: DRM device not found)
[  INFO  ] commit from client 1 surface 5: 256x256
[  INFO  ] frame 2: 1 surface(s) | 0.02ms | offscreen_fallback
```

`offscreen_fallback` মানে: ফ্রেম কম্পোজিট হয়েছে, কিন্তু কোনো প্যানেল জ্বলেনি।
বাইনারি তখন exit code `2` দেয় — এটা ইচ্ছাকৃত, যাতে স্ক্রিপ্ট "প্যানেল জ্বলেছে"
আর "শুধু মেমরিতে আঁকা হয়েছে" আলাদা করতে পারে। **আসল কার্ড থাকলে
`page_flip_success = true`** হয় এবং backend নাম পাল্টে যায়।

### ৬.৭.৮ যাচাই — mock নেই

তিন স্তরে যাচাই, প্রতিটি আসল কোড পথ ছোঁয়:

```bash
# ১. ইউনিট (৭৩টি): framing, opcode, shm, সার্ভার
cargo test -p nilui-gpu --lib wayland

# ২. cross-process: দুটি আসল বাইনারি, আসল socket, আসল SCM_RIGHTS
cargo test -p nilui-gpu --test wayland_cross_process -- --nocapture
# → compositor_composites_a_surface_from_a_separate_client_process ... ok
# → compositor_refuses_to_start_on_a_taken_socket ... ok

# ৩. সব মিলিয়ে
cargo test -p nilui-gpu
```

দ্বিতীয় টেস্টটি সত্যিই কিছু প্রমাণ করে কি না, সেটিও যাচাই করা হয়েছে —
"negative control": `composite()`-এর কল সরিয়ে দিলে টেস্ট fail করে
(`compositor never reported compositing the surface`), ফিরিয়ে আনলে পাস করে।
যে টেস্ট ব্যর্থ হতে পারে না, সেটি টেস্ট নয়।

### ৬.৭.৯ যা এখনো নেই — এবং কেন সেটা গুরুত্বপূর্ণ

এটি একটি **সফটওয়্যার** কম্পোজিটর। নিচের সবগুলো সত্যিই অনুপস্থিত:

- **`linux-dmabuf` নেই।** GPU বাফার সরাসরি নেওয়া যায় না। অ্যান্ড্রয়েডের
  SurfaceFlinger যে gralloc বাফার দেয় সেটি GPU মেমরি, তাই প্রতিটি ফ্রেম CPU-তে
  কপি করতে হবে — ফোনে এটি আসল খরচ। এটাই গ্রাফিক্স পাথ অসম্পূর্ণ হওয়ার মূল কারণ।
- **`xdg-shell` নেই।** তাই stock GTK/Qt অ্যাপ এখনো চলবে না।
- **`wl_region` clipping নেই।** রিজিয়ন গৃহীত ও সংরক্ষিত হয়, প্রয়োগ হয় না।
- **damage-limited redraw নেই।** `damage` যাচাই হয় কিন্তু বাদ দেওয়া হয়; প্রতি
  commit-এ পুরো সারফেস আবার আঁকা হয়।
- **`wl_pointer`/`wl_keyboard` নেই।** অবজেক্ট গ্রহণ করা হয় যাতে ক্লায়েন্ট ব্যর্থ
  না হয়, কিন্তু কোনো ইনপুট ইভেন্ট পাঠানো হয় না। কেবল `wl_touch`-এর encoder আছে।
- **কোনো আসল প্যানেলে এটি চালানো হয়নি।** dev container-এ DRM নোড নেই।

সারকথা: যে কেউ সঠিক subset-টি বলে, সে সারফেস বানিয়ে CPU পিক্সেল পাঠিয়ে সেটি
কম্পোজিট হতে দেখতে পারে। এর বেশি কিছু (বিশেষত `xdg-shell`) এখনো লেখা হয়নি —
এবং এই গাইড সেটি লুকায় না।

### ৬.৭.১০ অ্যান্ড্রয়েড ব্রিজের সাথে সম্পর্ক

`SurfaceProducer` API-টিই সেই অংশ যা একটি Android graphics bridge-এর দরকার হবে:
যার কাছে CPU পিক্সেলে আছে, সে `frame_mut()`-এ লিখে `commit()` ডাকে। কিন্তু
**SurfaceFlinger-এর সাথে সংযোগ এখনো লেখা হয়নি**, এবং `linux-dmabuf` ছাড়া
প্রতিটি ফ্রেম কপি করতে হবে। বিস্তারিত §৭-এ।

---

## ৭. অ্যান্ড্রয়েড ব্রিজ — বাস্তব কন্টেইনার

`services/nilandroidd/src/container.rs`।

### ৭.১ "বাস্তব" মানে এখানে ঠিক কী

1. **rootfs probe।** `/data/android/rootfs`-এ একটি executable Android `init`
   থাকতে হবে। খালি ডিরেক্টরি থাকলেই "চালু হয়েছে" বলা হয় না:
   `RootfsProbe::{Ready, Missing, NotAnAndroidRootfs}`।
2. **লঞ্চ** — Waydroid/LXC যেভাবে করে:
   `unshare(PID|NS|IPC|UTS|NET)` → mount tree private →
   rootfs bind → `pivot_root` → নতুন `/proc`,`/sys`,`/dev` →
   `/system`,`/vendor`,`/data` bind → cgroup v2 limit → container init চালু।
3. **state আসল child process থেকে।** `refresh()` child-কে `try_wait()` করে;
   মরে গেলে `Stopped`। কখনো ভুয়া `Running` নয়।

### ৭.২ নিরাপত্তা সীমানা (অপরিবর্তিত, অলঙ্ঘনীয়)

- কন্টেইনার **কখনো** `/dev/input`, `/dev/kmsg`, `/dev/mem`, `/dev/port` পায় না।
  `isolation.rs` policy স্তরে এগুলো নিষিদ্ধ করে, এবং `plan_container_mounts`-এর
  বিরুদ্ধে ক্রস-চেক করা হয় **কোনো privileged কাজ শুরুর আগে**।
- কন্টেইনারের `/system`, `/data` হোস্টের নয় — কন্টেইনারের নিজের storage।
- `/proc` অবশ্যই নতুন instance, নইলে কন্টেইনার হোস্টের process state পড়তে
  পারবে এবং `/proc/<pid>/root` দিয়ে বেরিয়ে যেতে পারবে।

### ৭.৩ IPC

`/run/nilos/android.sock`, canonical ONUR frame:

| message | কোড | উত্তর |
|---|---|---|
| StartContainer | `0x0B01` | `{"ok":true,"pid":N}` / `{"ok":false,"error":"…"}` |
| StopContainer | `0x0B02` | `{"ok":true}` |
| ContainerStatus | `0x0B03` | state/pid/status_line/rootfs |
| StartActivity | `0x0B04` | in-container agent-এ পাঠানো হয় |

### ৭.৪ rootfs বসানো (আপনাকে এই ধাপটি করতে হবে)

Android-এর নিজের userspace OnuronOS সরবরাহ করে না। একটি rootfs যোগ করুন:

```bash
# /data/android কাঠামো
sudo mkdir -p /data/android/{rootfs,data,system,vendor}
sudo mkdir -p /run/onuron/android

# একটি Android rootfs সংগ্রহ করুন (Waydroid/LineageOS system image) এবং
# সেটি /data/android/rootfs-এ আনপ্যাক করুন, যাতে নিচের একটি পথ তৈরি হয়:
#   /data/android/rootfs/system/bin/init
# অথবা /data/android/rootfs/init

# যাচাই — init অবশ্যই executable হতে হবে
test -x /data/android/rootfs/system/bin/init && echo "rootfs OK" || echo "init নেই"
```

rootfs প্রস্তুত হলে:

```bash
# ডেমন চালু আছে কি না দেখুন
ls -l /run/nilos/android.sock

# shell-এ: হোম → [7] Android → start
```

### ৭.৫ honest failure

rootfs না থাকলে যা দেখবেন:

```
Container: UNAVAILABLE
no container rootfs: /data/android/rootfs does not exist
```

অথবা:

```
rootfs /data/android/rootfs has no Android init binary
```

এটাই সঠিক আচরণ। ভুয়া সাফল্য দেখানো মানে ব্যবহারকারীকে ঠকানো।

### ৭.৬ in-container agent

`android/agent` কন্টেইনারের **ভেতরে** চলে এবং Android-এর `am`/`pm`-এর সাথে
কথা বলে। কন্টেইনার না চললে `forward_to_agent` সৎভাবে ব্যর্থ হয়
(`no nilagent socket`), ভুয়া "OK" দেয় না।

---

## ৮. বিল্ড

### ৮.১ QEMU (dev — সবচেয়ে দ্রুত)

```bash
source "$HOME/.cargo/env"

# সম্পূর্ণ বিল্ড + ইমেজ
./build/build.sh qemu-x86_64

# অথবা শুধু userspace দ্রুত যাচাই
cargo build --release --workspace
```

### ৮.২ আসল ডিভাইস (OnePlus 6T / fajita — রেফারেন্স ডিভাইস)

```bash
./build/build.sh oneplus-fajita
```

aarch64 cross-compile হবে। এরপর:

```bash
# fail-closed preflight যাচাই (ভুল ডিভাইসে ফ্ল্যাশ আটকায়)
./build/flash-device.sh oneplus-fajita
```

> **সতর্কতা:** ফ্ল্যাশিং সবসময় fail-closed safety gate-এর ভেতর দিয়ে যাবে
> (ADR-0002)। gate bypass করবেন না।

### ৮.৩ target তালিকা

| target | কাজ | triple |
|---|---|---|
| `qemu-x86_64` | ডেভেলপমেন্ট | `x86_64-unknown-linux-musl` |
| `qemu-aarch64` | ARM emulation | `aarch64-unknown-linux-musl` |
| `oneplus-fajita` | রেফারেন্স ফোন | `aarch64-unknown-linux-musl` |
| `android-host-arm64` | S25 হোস্টেড (আলাদা) | `aarch64-linux-android` |

---

## ৯. বুট করা

### ৯.১ QEMU-তে

```bash
./build/qemu-boot.sh                # গ্রাফিক্যাল
./build/qemu-boot.sh --headless     # হেডলেস (CI-এর মতো)
./build/qemu-boot.sh --no-rebuild   # দ্রুত পুনরাবৃত্তি
```

হোম স্ক্রিনে পৌঁছালে আপনি দেখবেন:

```
  [1] 📞 Phone      [2] 💬 Messages     [3] 📁 Files
  [4] ⚙️  Settings  [5] 📦 NilPkg       [6] 🔄 SoftBus
  [7] 🤖 Android    [8] 💻 Terminal     [9] 🌟 Hello (.nilax)
  [10] 🧮 Calculator [11] 📝 Notes      [12] 🎵 Music
  [13] 📷 Camera    [14] 🌐 Browser
  [n] 🔔 Notifications  [l] 🔒 Lock screen  [run <id>]
```

### ৯.২ আসল মোবাইলে

```bash
# bootloader আনলক করা ডিভাইস, সঠিক codename নিশ্চিত করুন
./build/build.sh oneplus-fajita
./build/flash-device.sh oneplus-fajita
```

বুটের পর যা কাজ করা উচিত:

- টাচ (inputd evdev দিয়ে)
- টার্মিনাল কমান্ড
- ব্রাউজার (নেটওয়ার্ক থাকলে)
- Android স্ক্রিন আসল container state দেখাবে

---

## ১০. টেস্ট

```bash
source "$HOME/.cargo/env"

# নতুন যোগ করা অংশ
cargo test -p inputd        # evdev ABI, ioctl, gesture
cargo test -p nilshell      # terminal, browser, UI, status bar
cargo test -p nilandroidd   # container lifecycle, mount plan

# আসল ডেমন চালিয়ে IPC যাচাই (mock নয়)
cargo test -p nilpower -p nilnet -- --nocapture

# সম্পূর্ণ workspace (sandbox unshare এখানে EPERM, §১০.১ দেখুন)
cargo test --workspace --exclude nillang
CI=true cargo test --workspace
```

### ১০.১ কেন `CI=true`

`runtime/nilrt`-এর sandbox টেস্ট `unshare(CLONE_NEW*)` ডাকে। একটি
unprivileged container-এ এটি `EPERM` দেয়। প্রকল্পে documented dev fallback
আছে (`CI=true` বা `NILRT_ALLOW_INSECURE_DEV=1`)। **আসল ডিভাইসে/root-এ এই
override লাগে না** — isolation পুরোপুরি বলবৎ থাকে।

override ছাড়া sandbox সচেতনভাবেই unconfined চলতে **অস্বীকার** করে:

```
unshare(...) failed: EPERM. Refusing unconfined execution in production mode.
```

এটাই কাঙ্ক্ষিত নিরাপত্তা আচরণ, বাগ নয়।

### ১০.২ বাস্তব নেটওয়ার্ক টেস্ট

`runtime/nilweb/src/lib.rs`-এ একটি টেস্ট আসল loopback TCP সার্ভার খুলে আসল HTTP
ফেচ করে — mock নয়:

```rust
#[test]
fn fetches_a_real_page_over_http() { ... }
```

### ১০.২.১ আসল ডেমন-ভিত্তিক IPC টেস্ট

`nilpower` আর `nilnet`-এর integration টেস্ট mock দেয় না। তারা **আসল ডেমন
বাইনারি spawn করে**, env দিয়ে ব্যক্তিগত socket দেয়, তারপর **আসল client** দিয়ে
পড়ে:

```bash
cargo test -p nilpower --test powerd_ipc -- --nocapture
cargo test -p nilnet    --test netd_ipc    -- --nocapture
```

লক্ষ্য করুন: টেস্টগুলো `cargo build -p powerd -p netd` আগে না হলে সৎভাবে
SKIP বলে (fail নয়)। CI-তে বাইনারি build থাকলে সেগুলো আসলেই চলে।

### ১০.৩ maturity ledger যাচাই

```bash
python3 build/gen-maturity.py          # README পুনরুৎপাদন
python3 build/gen-maturity.py --check  # drift নেই কি না
python3 -m unittest build.test_gen_maturity
```

generator এই নিয়ম প্রয়োগ করে: `ui_screens` থাকলে `simulated = true` থাকতেই
হবে; আর `shell/src/simulated.rs`-এর প্রতিটি key কোনো না কোনো entry-তে দাবি
করা থাকতে হবে।

### ১০.৪ একটি বাস্তব bug: flaky lock টেস্ট

`pkg/nilpkg`-এর lock টেস্টগুলো সমান্তরালে চালালে মাঝে মাঝে (~২৫%) ব্যর্থ হত:

```
called `Result::unwrap()` on an `Err` value:
  "another nilpkg operation is already running on this app root"
```

কারণ খুঁজে পাওয়া গেছে, এবং সেটি **lock-এর bug নয়**:

- `flock`-এর lock থাকে *open file description*-এ, শুধু fd-তে নয়।
- একই সময়ে চলা অন্য একটি টেস্ট child `fork` করে। `fork` ও তার `exec`-এর
  **মাঝের ছোট্ট জানালায়** child প্যারেন্টের সমস্ত descriptor-এর কপি ধরে রাখে।
- তাই সেই মুহূর্তে sibling টেস্টের lock-টিও "ধরা" থাকে — যদিও fd-টিতে
  `FD_CLOEXEC` সেট আছে (সেটা `exec`-এর পরে কাজ করে, জানালার ভেতরে নয়)।

একটি পরীক্ষা এটা প্রমাণ করেছে: শুধু `sleep(300ms)` যোগ করলেই ১০/১০ পাস
হয়। অর্থাৎ production lock সঠিক; সমস্যাটি টেস্ট হার্নেসের।

**সমাধান:** `pkg/nilpkg/src/lockfile.rs`-এর টেস্টগুলো এখন একটি module-local
`Mutex` নেয়, তাই process-spawning টেস্টটি অন্যগুলোর সাথে ওভারল্যাপ করে না।
সাথে `lock_fd_is_close_on_exec` টেস্টটি production গ্যারান্টি pin করে রাখে।
ফলাফল: ২০/২০ এবং workspace-এ টানা ৩ বার সব পাস।

এই ধরনের "flaky" টেস্ট দেখলে সেটা এড়িয়ে যাবেন না — হয় production bug, নয়
টেস্ট bug। এখানে দ্বিতীয়টি ছিল, কিন্তু প্রমাণ ছাড়া ধরে নেওয়া যায় না।

### ১০.৫ আরও দুটি বাস্তব bug — টেস্ট যা ধরেছে, চোখ যা ধরেনি

Wayland কম্পোজিটর লেখার সময় দুটি bug ধরা পড়েছে যেগুলো কোড পড়ে চোখে পড়ত না।
দুটোই টেস্ট লিখে ধরা পড়েছে, তাই এখানে লিখে রাখা।

**এক: প্রোটোকল ভাঙলে fd leak।** সার্ভার কোনো মেসেজ প্রত্যাখ্যান করলে
`break` দিয়ে session loop ছেড়ে বেরিয়ে যেত — আর বেরোনোর পথে প্রত্যাখ্যাত
মেসেজের সাথে আসা descriptor বন্ধ করার কোড এড়িয়ে যেত। অর্থাৎ একটি দুষ্ট
ক্লায়েন্ট প্রতি মেসেজে একটি করে fd ফাঁস করাতে পারত, আর শেষে পুরো সার্ভিসের
fd টেবিল ভরে যেত (DoS)। সমাধান: descriptor এখন `FdGuard`-এ থাকে, যার `Drop`
সব বন্ধ করে — তাই বেরোনোর যে-কোনো পথ নিরাপদ।

টেস্টটি এভাবে লেখা যে ফাঁস আগে **প্রমাণ** হয়: `/proc/self/fd` গুনলে চলে না,
কারণ Rust টেস্ট সমান্তরালে চলে আর অন্য টেস্টের socket-ও গুননায় পড়ে (এটা
নিজেই একটি flaky টেস্ট বানায়)। তাই প্রতিটি রাউন্ডের memfd-এর **inode** ধরে
রাখা হয় এবং দেখানো হয় সেটি আর খোলা নেই। ঠিক করার আগে টেস্ট fail করত
(`pool descriptors leaked`), পরে পাস করে।

**দুই: blocking `accept` `shutdown()`-কে অর্থহীন করে দেয়।** লিসেনার blocking
ছিল, তাই accept loop চিরকালের জন্য থেমে থাকত। ফল: `WouldBlock` শাখাটি কখনো
চলত না (dead code), কম্পোজিটর SIGTERM-এ বন্ধ হত না, আর socket bound রেখে
যেত। সমাধান: `bind`-এ `set_nonblocking(true)`। গৃহীত (accepted) stream আগের
মতোই blocking থাকে, কারণ ক্লায়েন্ট থ্রেড synchronous পড়ে।

এই bug-টি সত্যিই ধরা পড়ে **প্রসেস চালিয়ে**: `kill -TERM` দেওয়ার পরেও প্রসেস
বেঁচে ছিল। আর একটা ফাঁদ এখানে শিখার মতো — বন্ধ হওয়া প্রসেস `ps`-এ `Z`
(zombie) অবস্থায় দেখায় যদি কন্টেইনারের PID 1 সেটি reap না করে। তাই সিদ্ধান্তে
পৌঁছানোর আগে `/proc/<pid>/stat`-এর state দেখুন, শুধু `ps`-এ প্রসেস থাকা দেখে
"সিগন্যাল মানেনি" ভাববেন না।

সাথে ছোট একটা জিনিস: কম্পোজিটর বাইনারি এখন `process::exit`-এর আগে সার্ভার
`drop` করে, যাতে socket unlink হয়। আগে সরাসরি exit করলে destructor এড়িয়ে
যেত আর পরের বার চালু করার সময় পুরনো socket পড়ে থাকত (bind কোড সেটা সামলায়,
কিন্তু পরিষ্কার রাখা ভালো)।

---

## ১১. সমস্যা সমাধান

### টাচ কাজ করছে না

```bash
ls -l /dev/input/event*
# inputd লগে evdev device তালিকা দেখা উচিত
# যদি "No consumable evdev devices" দেখেন:
cat /proc/bus/input/devices
```

`inputd` rootfs-এ আছে কি না দেখুন:

```bash
ls -l /usr/bin/inputd
```

### ব্রাউজার কিছু দেখাচ্ছে না

```bash
# netd চালু?
ls -l /run/nilos/netd.sock
# DNS কাজ করে?
cat /etc/resolv.conf
# হাতে যাচাই
# (টার্মিনালে) sh -c "..." না লিখে সরাসরি কমান্ড দিন
```

UI বলবে `network error: ...` — সেটি পড়ুন, অনুমান করবেন না।

### Android container চালু হচ্ছে না

```bash
# ১. rootfs আছে?
test -x /data/android/rootfs/system/bin/init && echo OK || echo "init নেই"

# ২. bridge ডেমন চালু?
ls -l /run/nilos/android.sock

# ৩. nilandroidd-এর বুট লগ দেখুন — সেখানে সঠিক কারণ লেখা থাকবে
#    ("does not exist" / "has no Android init binary")
```

`unshare` ব্যর্থ হলে `CAP_SYS_ADMIN` দরকার — nilandroidd root হিসেবে চলতে হবে।

### টার্মিনালে কমান্ড পাওয়া যাচ্ছে না

```
onuron: command not found: <name>
```

তাহলে বাইনারিটি ইমেজে নেই। যোগ করার দুই ধাপ:

1. `build/build.sh`-এর `BINS` তালিকা
2. `build/mkinitramfs.py`-এর `bins` তালিকা

তারপর আবার বিল্ড।

### `cd` অদ্ভুত আচরণ করছে

মনে রাখুন: `cd` lexically `.`/`..` সমাধান করে, তারপর পথ সত্যিই আছে কি না
দেখে। যে পথ নেই, `cd` সেখানে **যাবে না** — এটি সঠিক POSIX আচরণ।

---

## ১২. নতুন daemon/অ্যাপ যোগ করার নিয়ম

1. `services/<name>/` বা `apps/<name>/` তৈরি করুন।
2. `Cargo.toml`-এ workspace member হিসেবে যোগ করুন (যদি লাগে)।
3. IPC-র জন্য `protocol` crate-এর canonical frame ব্যবহার করুন —
   নিজের আলাদা protocol বানাবেন না।
4. `etc/nilos/services.toml`-এ supervisor entry দিন।
5. `build/build.sh` ও `build/mkinitramfs.py`-এর তালিকায় নাম দিন।
6. টেস্ট লিখুন — mock নয়, আসল code path।
7. `docs/maturity.toml`-এ সৎ tier দিন (`IMPLEMENTED` / `FUNCTIONAL PROTOTYPE`
   / `EXPERIMENTAL` / `STUB/SIMULATED` / `PLANNED`) এবং `evidence` পথ দিন।
8. `python3 build/gen-maturity.py && python3 build/gen-maturity.py --check`
9. আর্কিটেকচার বদলালে `docs/adr/`-এ ADR লিখুন।

---

## ১৩. সোনালি নিয়ম (AGENTS.md থেকে)

কোনো পরিবর্তনের আগে জিজ্ঞাসা করুন:

> এটি OnuronOS-কে একটি স্বাধীন Linux-ভিত্তিক মোবাইল OS হিসেবে, native
> NilLang/Alap অ্যাপ্লিকেশন প্ল্যাটফর্ম সহ, শক্তিশালী করে — নাকি চুপচাপ
> OnuronOS-কে অন্য কিছুতে বদলে দিচ্ছে?

যদি উত্তর "অন্য কিছু" হয় — থামুন এবং দ্বন্দ্বটি নথিবদ্ধ করুন।

যেগুলো অনুমতি ছাড়া **করবেন না:**

1. AOSP-তে migrate করা
2. প্রচলিত Android ROM বানানো
3. NilLang/Alap-কে generic web/PWA দিয়ে বদলানো
4. `systemd`/`OpenRC` লুকানো নির্ভরতা হিসেবে ঢোকানো
5. ডিরেক্টরি আছে বলে feature production-ready ধরে নেওয়া
6. প্রমাণ ছাড়া হার্ডওয়্যার সাপোর্ট দাবি করা
7. feature কাজ করানোর জন্য sandbox/SELinux/seccomp বন্ধ করা
8. abstraction থাকা সত্ত্বেও subsystem আবার লেখা
9. Android compatibility সরাসরি native অ্যাপ মডেলে জোড়া লাগানো

---

## ১৪. এখনও যা বাকি (সৎ তালিকা)

- **অ্যান্ড্রয়েড গেস্ট ইউজারস্পেস + গ্রাফিক্স পাসথ্রু** — কন্টেইনার চলে, আর
  Wayland সার্ভারও এখন বাস্তব (§৬.৭), কিন্তু দুটির মধ্যে সেতু লেখা হয়নি:
  SurfaceFlinger-এর বাফার নেওয়ার কোড নেই, এবং `linux-dmabuf` না থাকায় GPU
  বাফার সরাসরি নেওয়া যায় না। এই দুটি হলে UI স্ক্রিনটির `[SIMULATED]` ব্যাজ
  উঠবে।
- **Wayland: `linux-dmabuf`, `xdg-shell`, damage-limited redraw, pointer/keyboard
  ইভেন্ট** — বাস্তব মোবাইল কম্পোজিটরের জন্য প্রয়োজন, এখনো লেখা হয়নি
  (§৬.৭.৯-এ পূর্ণ তালিকা)।
- **টার্মিনাল pipeline/redirection** — ইচ্ছাকৃতভাবে বাদ, UI-তে লেখা আছে।
- **ব্রাউজারে ছবি/CSS** — টেক্সট-মোড ব্রাউজার।
- **আসল মোবাইলে হার্ডওয়্যার bring-up** — DRM/KMS, টাচ, storage, power,
  networking। রেফারেন্স ডিভাইসে যাচাই ছাড়া "supported" বলা যাবে না। ব্যাটারি
  ও নেটওয়ার্কের IPC পথ এখন আসল (আসল ডেমন spawn করে টেস্ট হয়), কিন্তু আসল
  ব্যাটারি/মডেম থেকে মান পড়া এখনো ডিভাইসে যাচাই হয়নি (§৬.৬)।
- **Waydroid/LineageOS rootfs সংগ্রহ** — আইনি ও বিতরণগত কারণে প্রকল্প এটা
  বান্ডল করে না; আপনাকে আলাদা করতে হবে (§৭.৪)।

---

## ১৫. এক নজরে দ্রুত কমান্ড

```bash
# টুলচেইন
source "$HOME/.cargo/env"

# বিল্ড + বুট (QEMU)
./build/build.sh qemu-x86_64 && ./build/qemu-boot.sh

# টেস্ট
cargo test -p inputd -p nilshell -p nilandroidd
CI=true cargo test --workspace

# Wayland কম্পোজিটর (দুটি টার্মিনাল)
export XDG_RUNTIME_DIR=/run/onuron
cargo run -p nilui-gpu --bin wayland_compositor -- --width 1080 --height 2400
cargo run -p nilui-gpu --bin wayland_surface_producer -- --frames 5

# Wayland টেস্ট
cargo test -p nilui-gpu --lib wayland
cargo test -p nilui-gpu --test wayland_cross_process -- --nocapture

# maturity যাচাই
python3 build/gen-maturity.py --check

# আসল ডিভাইস
./build/build.sh oneplus-fajita && ./build/flash-device.sh oneplus-fajita
```

---

*এই নির্দেশিকাটি OnuronOS-এর বর্তমান কোডের সাথে মিলিয়ে লেখা। tier ও অবস্থার
একক সত্য হলো `docs/maturity.toml`; কিছু এখানে ভিন্ন লাগলে সেটিই মানুন।*
