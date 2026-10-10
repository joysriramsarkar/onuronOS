# OnuronOS: বাস্তব ফোনে চালানোর জন্য পূর্ণাঙ্গ বাস্তবায়ন পরিকল্পনা

**ভাষা:** বাংলা  
**প্রকল্প:** [joysriramsarkar/onuronOS](https://github.com/joysriramsarkar/onuronOS)  
**পর্যালোচনার ভিত্তি:** ৯ অক্টোবর ২০২৬-এ `main`-এর সর্বশেষ দেখা কমিট `868fa19b9cdf723702a8e7ecf4a8e19ae7ca811a`  
**পরিকল্পনার লক্ষ্য:** বর্তমান prototype-কে ধাপে ধাপে যাচাইযোগ্য ARM64 Linux system, স্থিতিশীল hosted Android runtime, এবং পরে একটি নির্দিষ্ট পরীক্ষামূলক ফোনে bootable OnuronOS-এ পরিণত করা।

> **সতর্কতা ও সীমা:** এই পরিকল্পনা রিপোজিটরির বর্তমান কোড, workflow, build script এবং ডকুমেন্টেশন দেখে তৈরি। GitHub Actions-এ পাস হওয়া পরীক্ষা মানে যে পরীক্ষাটি সত্যিই চালানো হয়েছে সেটি পাস করেছে—এটি বাস্তব ফোনে boot, কল, ক্যামেরা, GPU, power management, secure boot বা দৈনন্দিন ব্যবহার যাচাইয়ের বিকল্প নয়। বর্তমান generic flasher দিয়ে কোনও দৈনন্দিন ফোনে image flash করা উচিত নয়। পরিকল্পনার acceptance criteria পূরণ না হওয়া পর্যন্ত native-phone target-কে unsupported prototype হিসেবেই দেখাতে হবে।

---

# সূচিপত্র

১. [নির্বাহী সারাংশ ও প্রকল্পের অবস্থান](#অংশ-১--নির্বাহী-সারাংশ-ও-প্রকল্পের-অবস্থান)  
২. [বর্তমান রিপোজিটরির audit](#অংশ-২--বর্তমান-রিপোজিটরির-প্রাসঙ্গিক-audit)  
৩. [কাজের নিয়ম ও প্রকল্প ব্যবস্থাপনা](#অংশ-৩--পরিকল্পনা-চালানোর-নিয়ম-ও-প্রকল্প-ব্যবস্থাপনা)  
৪. [Milestone 0: build ও CI](#অংশ-৪--milestone-0-মূল-build-ও-ci-কে-নির্ভরযোগ্য-করা)  
৫. [Milestone 1: ARM64 QEMU](#অংশ-৫--milestone-1-arm64-qemu-কে-সত্যিকারের-target-করা)  
৬. [Milestone 2: S25 hosted runtime](#অংশ-৬--milestone-2-samsung-s25-এ-hosted-android-runtime)  
৭. [Milestone 3: NilHAL](#অংশ-৭--milestone-3-nilhal-কে-চুক্তি-হিসেবে-পরিণত-করা)  
৮. [Milestone 4: sandbox ও security](#অংশ-৮--milestone-4-app-sandbox-permissions-ও-platform-security)  
৯. [Milestone 5: NilLang/NilUI/runtime](#অংশ-৯--milestone-5-nillang--nilui--actual-app-runtime)  
১০. [Milestone 6: reference phone](#অংশ-১০--milestone-6-প্রথম-native-reference-phone-নির্বাচন)  
১১. [Milestone 7: native phone bring-up](#অংশ-১১--milestone-7-native-phone-bring-up-এর-ধাপ)  
১২. [Milestone 8: boot/init/recovery](#অংশ-১২--milestone-8-boot-init-service-lifecycle-ও-recovery)  
১৩. [Milestone 9: storage ও OTA](#অংশ-১৩--milestone-9-storage-encryption-package-এবং-ota-updates)  
১৪. [Milestone 10: UI ও system apps](#অংশ-১৪--milestone-10-ui-shell-native-ux-ও-system-apps)  
১৫. [Milestone 11: system services](#অংশ-১৫--milestone-11-system-service-catalogue-ও-dependency-management)  
১৬. [Milestone 12: reliability/performance](#অংশ-১৬--milestone-12-performance-reliability-ও-soak-testing)  
১৭. [বাস্তবায়নের সময়রেখা ও অগ্রাধিকার](#অংশ-১৭--বাস্তবায়নের-সময়সরেখা-ও-কাজের-অগ্রাধিকার)  
১৮. [Repository file-by-file action list](#অংশ-১৮--repository-file-by-file-action-list)  
১৯. [Test matrix](#অংশ-১৯--পরীক্ষার-পূর্ণ-matrix-ও-test-command-catalogue)  
২০. [Safe flashing guide](#অংশ-২০--safe-flashing-workflow-ও-command-level-operator-guide)  
২১. [Architecture Decision Records](#অংশ-২১--architecture-decision-records-adr)  
২২. [Risk register](#অংশ-২২--risk-register-ও-mitigation-plan)  
২৩. [প্রথম ৩০টি কাজ](#অংশ-২৩--হাতে-কলমে-প্রথম-৩০টি-কাজ)  
২৪. [Troubleshooting playbook](#অংশ-২৪--troubleshooting-playbook)  
২৫. [Release engineering](#অংশ-২৫--release-engineering-versioning-ও-public-status)  
২৬. [Documentation/source policy](#অংশ-২৬--documentation-ও-research-source-policy)  
২৭. [Glossary](#অংশ-২৭--glossary-গুরুত্বপূর্ণ-ধারণা-এক-জায়গায়)  
২৮. [Final acceptance gate](#অংশ-২৮--final-acceptance-gate-কোন-অবস্থায়-কী-দাবি-করা-যাবে)  
২৯. [এখন করো/করো না](#অংশ-২৯--এখন-কী-করবে-কী-করবে-না)  
৩০. [সারাংশ ও success definition](#অংশ-৩০--summary-প্রকল্পের-success-definition)  
৩১. [Code review policy](#অংশ-৩১--code-review-policy-ও-contributor-checklist)  
৩২. [Next-step decision tree](#অংশ-৩২--শেষ-next-step-decision-tree)  
৩৩. [App compatibility](#অংশ-৩৩--app-compatibility-এবং-api-stability)  
৩৪. [দৈনন্দিন engineering workflow](#অংশ-৩৪--প্রতিদিনের-engineering-workflow)

---

# অংশ ১ — নির্বাহী সারাংশ ও প্রকল্পের অবস্থান

## ১.১ প্রধান সিদ্ধান্ত

OnuronOS-এর লক্ষ্য হওয়া উচিত **একটি সাধারণ OS core, একটি পরিষ্কার HAL চুক্তি, একাধিক backend, এবং আলাদা target profile**। প্রতিটি ফোনের জন্য গোটা OS নতুন করে লেখা হবে না; কিন্তু প্রতিটি ফোনের kernel, device tree, firmware, boot image, partition layout, GPU/display path, modem, camera, audio, power এবং recovery-র কাজ যাচাই করতে হবে। একই source code থেকে x86_64, ARM64 Linux এবং Android `arm64-v8a`-এর জন্য আলাদা binary তৈরি হবে। একই CPU architecture হলেও Linux-musl executable এবং Android native library এক target নয়।

এই পরিকল্পনার ক্রম হলো:

1. বর্তমান পরীক্ষাগুলো deterministically green রাখা এবং build script-কে পরিষ্কারভাবে fail-closed করা।
2. ARM64 QEMU target-কে শুধু script হিসেবে নয়, বাস্তবভাবে build, boot ও persistence-সহ validate করা।
3. Samsung Galaxy S25-এ Android-host APK-কে প্রথমে UI demo, পরে JNI/native Rust bridge-সহ hosted runtime হিসেবে নির্ভরযোগ্য করা।
4. NilLang → package → trusted install → sandbox → runtime → NilUI → input event পর্যন্ত প্রকৃত app execution loop তৈরি করা।
5. SELinux, app isolation, publisher trust, verified boot, update rollback ও recovery-কে release gate হিসেবে প্রতিষ্ঠা করা।
6. একটি নির্দিষ্ট, unlockable, আগে থেকেই Linux community support-যুক্ত পরীক্ষার ফোন নির্বাচন করে device port শুরু করা।
7. ফোনে boot, storage, display, touch, charging, network, suspend/resume, audio, camera এবং telephony-র evidence সংগ্রহ করা; তারপরই supported status দাবি করা।

এই ক্রমের লাভ হলো সমস্যাকে স্তরে ভাগ করা যায়। যদি ARM64 QEMU-তেও initramfs ঠিকমতো তৈরি না হয়, তবে ফোনের device tree বা bootloader debug করার দরকার নেই। যদি QEMU ও native UI চলে কিন্তু S25-এর host bridge কাজ না করে, সেটি Android-host integration-এর সমস্যা; kernel সমস্যার নয়। একইভাবে native phone-এ display কাজ করলেও modem না চললে OS-কে পুরোপুরি supported বলা যাবে না।

## ১.২ এখনকার উন্নতি কী প্রমাণ করে—আর কী করে না

সর্বশেষ দেখা `868fa19` কমিটে আগের command-queue test race-এ আরও robust synchronization/poison recovery যোগ হয়েছে। ওই commit-এর সময়কার ছয়টি workflow—Linux/QEMU, Code Quality, Security Audit, SELinux Policy CI, Windows Simulator Build এবং Farm CI—সব পাস করেছে। এটি ইতিবাচক, কারণ আগের `cd7d2a9` commit-এ `android-host`-এর একটি queue test ভেঙে গিয়েছিল; পরে `8051eaaa`-তে race fix এবং SELinux loader/build hygiene-র পরিবর্তন, তারপর `868fa19`-তে global test synchronization আরও শক্ত করা হয়েছে।

তবে `linux-qemu.yml` এখনও `x86_64-unknown-linux-musl` build করে এবং `qemu-system-x86_64`-এর smoke test চালায়। তাই green workflow-কে ARM64 QEMU validation হিসেবে গণ্য করা যাবে না। `build/qemu-aarch64.sh` ও `build/qemu-aarch64.ps1` যোগ হয়েছে, কিন্তু আলাদা ARM64 CI matrix বা ওই architecture-এ সফল boot log এখনো প্রমাণিত হয়নি। এগুলোকে **ARM64 bring-up tools** বলা সঠিক; **ARM64 validated target** নয়।

`android-host/build-ndk.sh` ARM64 Android native library তৈরির পথ যোগ করে, কিন্তু শুধু script উপস্থিত থাকলেই APK-তে `.so` সঠিকভাবে packaged হয় বা Java↔Rust JNI path পুরোপুরি চলে—তা প্রমাণ হয় না। `MainActivity`-এর UI, `NativeBridge`-এর native method declarations, Rust `#[no_mangle]` exports, camera/audio command dispatcher, এবং Android permissions/runtime consent একসঙ্গে পরীক্ষা করতে হবে।

`nilinit`-এ core process-গুলো শুরু হওয়ার পর live-process handle-এর ভিত্তিতে health check এসেছে। এটি আগের চেয়ে ভালো, কিন্তু process চলা মানেই service-এর socket, protocol, hardware backend বা application readiness কাজ করছে নয়। পরের ধাপে service-specific readiness handshake দরকার হবে।

## ১.৩ প্রকল্পের বর্তমান maturity-র নীতি

`docs/maturity.toml`-কে subsystem-এর truth source হিসেবে ব্যবহার করতে হবে। UI-র screenshot, in-memory mock, placeholder JPEG, hard-coded Wi-Fi state, simulated VoLTE বা test-only bytecode renderer-কে real feature বলা যাবে না। Maturity tier প্রতিটি feature-এর evidence অনুযায়ী নির্ধারিত হবে:

- **Not implemented:** পরিকল্পনা আছে, চালানোর মতো implementation নেই।
- **Stub / simulated:** UI বা API-র একটি অংশ আছে, কিন্তু বাস্তব underlying device/protocol নেই।
- **Experimental:** আংশিক implementation; failure handling/compatibility অসম্পূর্ণ।
- **Functional prototype:** নির্দিষ্ট test environment-এ end-to-end কাজ করে; production hardening এখনও বাকি।
- **Production-ready:** পুনরাবৃত্ত hardware test, security review, rollback, performance, lifecycle এবং release procedure দ্বারা প্রমাণিত।

এই প্রকল্পে কোনও subsystem-কে কেবল compile হওয়া, unit test পাস করা, code path থাকা বা `Ok(())` ফেরানোর কারণে functional বলা যাবে না। প্রতিটি claim-এর সঙ্গে test log, device profile, test command ও expected/observed result সংযুক্ত করতে হবে।

## ১.৪ প্রকল্পের তিনটি পৃথক delivery track

### Track A — Virtual machine system

এটি OS core-কে স্থিতিশীল করার প্রধান ট্র্যাক। বর্তমান x86_64 QEMU reference বজায় থাকবে। এর পাশাপাশি ARM64 QEMU `virt` target-এ cross-compile, initramfs, data filesystem, boot, service health, storage persistence ও shutdown পরীক্ষা যোগ করতে হবে। Track A সফল হলেই বাস্তব ফোনের সব driver পাওয়া যাবে—এমন নয়; কিন্তু ARM64 userspace এবং boot assumptions যাচাই হবে।

### Track B — Android-host runtime

এটি Samsung S25-এ Android-এর ভিতরে চালানো একটি hosted environment। Android kernel ও security model থাকবে; Onuron screen একটি Activity/Surface-এ চলবে। Host-side API-র মাধ্যমে screen/input/battery/connectivity ইত্যাদি পড়া বা action শুরু করা হবে—যেখানে Android অনুমতি দেয়। এই track OS replacement নয় এবং সেই ভাষায় প্রচার করা যাবে না।

### Track C — Native phone port

এখানে Linux kernel ফোনের SoC-তে boot করবে; `nilinit` PID 1 হবে; rootfs ও services native Linux environment-এ চলবে। প্রাথমিক ফোনটি হবে **একটি** reference device, একসঙ্গে তিন-চারটি ফোন নয়। এই track-এর আগে target codename, bootloader state, kernel/device tree, firmware, partition map, recovery route ও rollback plan লিখিতভাবে অনুমোদিত থাকতে হবে।

## ১.৫ এই পরিকল্পনার ‘Done’ মানে কী

পরিকল্পনা সম্পূর্ণ বলা যাবে না যখন শুধু UI দেখা যায় বা emulator boot করে। ন্যূনতম তিনটি পৃথক Done Gate থাকবে:

- **ARM64-QEMU Gate:** clean checkout থেকে reproducible target build; pinned kernel verify; initramfs architecture check; QEMU boot; healthy services; data write → reboot → data read; failure injection; CI-তে পুনরাবৃত্ত green run।
- **Hosted-S25 Gate:** APK source থেকে তৈরি; native Rust `.so` packaged; JNI symbols matched; touch/display pipeline বাস্তবে কাজ করে; camera/audio command বাস্তব host API-তে যায়; permission denial সঠিকভাবে সামলানো হয়; network/call/SMS status সত্য তথ্য থেকে আসে; app lifecycle-এ leaked service/thread থাকে না।
- **Native-Phone Gate:** সঠিক device-specific image; bootloader ও recovery পরীক্ষিত; console/boot evidence; persistent storage; display/touch; power/thermal; network; audio; camera; modem capability matrix; verified boot/release signing model; installation/rollback procedure; দীর্ঘ runtime test। সব capability একসঙ্গে না চললে ঠিক কোনগুলি supported তা স্পষ্ট করে বলতে হবে।

---

# অংশ ২ — বর্তমান রিপোজিটরির প্রাসঙ্গিক audit

## ২.১ ARM64 QEMU launcher-এর অবস্থা

বর্তমান `build/qemu-aarch64.sh` এবং `build/qemu-aarch64.ps1` আলাদা `out/aarch64-qemu/` directory ব্যবহার করে, `qemu-system-aarch64 -M virt` চালায়, `cortex-a57` CPU profile, serial console এবং VirtIO block/network ডিভাইস যুক্ত করে। এটা একটি উপযোগী পদক্ষেপ। কিন্তু script কয়েকটি শর্ত পূরণ না করলে তা নির্ভরযোগ্য boot path নয়:

1. `--no-rebuild` flag-এ মান ভুলভাবে set হচ্ছে; এটি ঠিক করতে হবে যাতে flag সত্যিই rebuild বন্ধ করে।
2. `mkinitramfs.py --arch aarch64` kernel ডাউনলোড ও initramfs তৈরির কাজ করে, কিন্তু target binary আগে তৈরি না থাকলে `nilinit` না পাওয়া বা ভুল binary fallback হওয়ার ঝুঁকি আছে। Build orchestration-এ আগে ARM64 `cargo build`, পরে initramfs packaging নিশ্চিত করতে হবে।
3. `prepare_rootfs()`-এর architecture-specific release directory না পেলে `target/release` fallback ব্যবহার হয়। Host-native binary ARM64 rootfs-এ চলে যেতে পারে। Cross-architecture image-এ host fallback স্বয়ংক্রিয়ভাবে নিষিদ্ধ করতে হবে।
4. `data.img` তৈরি হয় `truncate -s 512M` দিয়ে। এতে filesystem নেই। `nilinit` raw device-কে ext4/ext2 হিসেবে mount করার চেষ্টা করলে mount ব্যর্থ হবে এবং `/data`-কে tmpfs-এ নামিয়ে দিতে পারে। Data image তৈরির পর `mke2fs` বা সমমানের reproducible formatter দিয়ে filesystem তৈরি করতে হবে এবং boot log-এ filesystem identity যাচাই করতে হবে।
5. Kernel digest pin-এ mismatch হলে script কেবল log করে এগিয়ে যায়। Digest mismatch অবশ্যই fatal error হবে; downloaded বা cached file-কে যাচাই ছাড়া ব্যবহার করা যাবে না।
6. Linux/QEMU workflow এখনও x86_64-only। ARM64 target-এর জন্য আলাদা build/test job যোগ না করলে ARM64 script CI-তে কখনও যাচাই হবে না।
7. ARM64 `virt` machine-এ QEMU console, PCI/virtio, block device path, initramfs argument এবং kernel configuration একত্রে যাচাই করা হয়নি। এগুলি প্রথম boot experiment-এর log-সহ যাচাই করতে হবে।

## ২.২ Android native build-এ বর্তমান ফাঁক

`android-host/build-ndk.sh` `aarch64-linux-android` target-এর জন্য `libandroid_host.so` তৈরি করে `app/src/main/jniLibs/arm64-v8a/`-এ রাখার উদ্দেশ্যে লেখা। APK build config-এ `jniLibs` source path যোগ হয়েছে। এগুলি দরকারি scaffolding। কিন্তু:

- Gradle task এখনও native build script-কে invoke করে CI/build process-এর অংশ করে না। Developer-কে আগে আলাদাভাবে script চালাতে হবে, নইলে APK native library ছাড়াই তৈরি হতে পারে।
- fallback `cargo build --target` পথের জন্য Android target install, NDK linker/CC configuration ও API level নিশ্চিত করার দরকার। `cargo-ndk` থাকলে সহজ হতে পারে, কিন্তু CI environment সেট আপ করতে হবে।
- Rust code-এ `JNI` export signatures, Java static native declarations এবং string/array marshaling-এ ABI mismatch হলে app launch বা call path crash করতে পারে। Unit test Rust-side function test করে; তা Android VM-তে JNI call-এর বাস্তব পরীক্ষা নয়।
- `NativeBridge` missing library হলে exception এড়িয়ে চলে, যা UI demo-কে launch করতে সাহায্য করে। কিন্তু release build-এ production-only feature silently disabled থাকা যাবে না। Native path mandatory হলে handshake-এর ফল স্পষ্ট করে দেখাতে হবে।
- `OnuronBridgeService` একটি worker thread-এ command poll করে। Android lifecycle, background execution, main-thread operation, permission result, bounded queue ও service kill/restart behavior পরীক্ষা করা দরকার।
- Command parser-এ বর্তমানে সীমিত কিছু action implemented। camera preview/capture, actual audio playback/recording, brightness, network, host-event reporting এবং telephony state-কে end-to-end যুক্ত করতে হবে।
- Android permission manifest-এ যোগ করা হলেও runtime permission request, rationale, denial, “don't ask again”, feature unavailable, API level এবং user-consent workflow লিখতে হবে।
- `android-host/app/.gradle`, `app/build` এবং `android-host/Onuron.apk`-এর মতো generated artifact git tree-তে এখনও দেখা যায়। `.gitignore` ভবিষ্যতের untracked file-কে লুকোয়; আগে tracked file বাদ দিতে `git rm --cached`-এর মতো tracked-state cleanup দরকার।

## ২.৩ Init, services ও storage

`nilinit/src/main.rs` `services.toml` পড়ে core service spawn করে এবং গুরুত্বপূর্ণ process alive কি না দেখে। পরবর্তী ধাপ:

- service binary সত্যিই target rootfs-এ আছে কি না তা image-build stage-এ validate করতে হবে।
- `socket_activation` হিসেবে চিহ্নিত service-এর listener register, activation request, file descriptor handoff ও protocol accept বাস্তবে কাজ করে কি না পরীক্ষা করতে হবে।
- Service-specific readiness endpoint যোগ করতে হবে; যেমন `nild`-এর IPC socket ready, `nilbus` message round-trip, `netd` initial state report, `audiod` backend initialized।
- Boot success message-এ শুধু live process count নয়, প্রতিটি mandatory service-এর health evidence থাকতে হবে।
- `/data` mount-এর আগে disk existence, valid filesystem, expected UUID/label, read-only recovery condition এবং filesystem error সামলাতে হবে।
- Blank disk, wrong architecture, wrong filesystem, read-only disk, storage full, missing `/data` আৰু abrupt power loss-এর test থাকতে হবে।
- `tmpfs` fallback বৈধ emergency behavior হতে পারে, কিন্তু user-visible warning prominent হতে হবে; settings UI যেন ephemeral storage-কে persistent হিসেবে না দেখায়।

## ২.৪ Security ও verified boot

`runtime/nilrt/src/sandbox.rs`-এ namespace isolation, `PR_SET_NO_NEW_PRIVS`, chroot/pivot-root ও permission-driven filesystem restrictions আছে। Network permission না থাকলে `CLONE_NEWNET` যোগ হয়েছে। কিন্তু namespace flags যোগ হওয়া মানেই পূর্ণ sandbox নয়। Network namespace-এর ভিতরে interface বন্ধ আছে কি না, loopback, raw socket, AF_UNIX escape path, inherited file descriptor, host bridge endpoint, IPC socket ACL এবং host service authority আলাদা পরীক্ষা করতে হবে।

App UID allocator registry collision কমায় এবং একই registry-তে existing ID-কে একই UID দেওয়ার চেষ্টা করে। তবে concurrent launch-এ registry read-modify-write atomic না হলে race হতে পারে; registry file corruption, stale entries, allocation exhaustion, permissions, `NIL_APP_UID` override এবং privilege boundary পরীক্ষা করতে হবে। Production launch-এ developer override পরিবেশভেরিয়েবল trusted environment ছাড়া ব্যবহার করা চলবে না।

SELinux-related code এখন আগের চেয়ে ভালো, তবে policy build script-এ compile failure নীরবে উপেক্ষা করা চলবে না। `nilinit` policy load failure-এ success log দেবে না। Audit script-ও শুধু regular-expression search করে real SELinux policy semantics প্রমাণ করতে পারে না; `secilc`/policy compiler validation, policy test suite এবং runtime AVC/enforcing check প্রয়োজন।

`mkvbmeta.py`-তে স্বাক্ষর ও image hash আছে, কিন্তু একটি custom text descriptor বা embedded public key-ই bootloader root of trust নয়। Fresh key তৈরি করে সেই public key-কে একই descriptor-এ লিখলে attacker image ও descriptor দুইটিই বদলাতে পারলে trust প্রতিষ্ঠিত হয় না। Production verified boot-এর জন্য independently trusted/pinned key, bootloader integration, boot partition integrity, rollback index/anti-rollback, signed manifest, key rotation ও recovery key policy দরকার। `fastboot flash` path-কে standard Android AVB support আছে বলে দাবি করা যাবে না যতক্ষণ বাস্তব bootloader verify test করা হয়নি।

## ২.৫ UI ও NilLang

NilLang parser, AST, serialized bytecode এবং `NilVM::render_scene()` আছে। `render_scene()` UI-র string/tree-like representation তৈরি করে; সেটি নিজে GPU framebuffer-এ আঁকা, gesture বুঝে Button action চালানো বা reactive state update করার সমান নয়। Test-এ install করা bytecode NilVM-এ পুনরায় load করা হয়, কিন্তু `nilrt-launch`-এর বাস্তব sandbox, system app lifecycle ও NilUI compositor-এর সঙ্গে পূর্ণ integration এখনও প্রমাণিত নয়।

প্রথম লক্ষ্য সম্পূর্ণ language specification লেখা নয়; প্রথম লক্ষ্য হবে একটি ছোট, কিন্তু end-to-end **Hello app**: `.nil` source → compiler → `.nilax` package → trusted signature verification → install → permission broker → `nilrt-launch` → VM → NilUI widget tree → framebuffer → touch event → Button callback → visible state change → app exit। এই vertical slice সম্পূর্ণ হলে তবেই language feature বাড়ানো সুবিধাজনক।

---

# অংশ ৩ — পরিকল্পনা চালানোর নিয়ম ও প্রকল্প ব্যবস্থাপনা

## ৩.১ কাজের আকার

প্রতিটি task-কে এমন ছোট করতে হবে যাতে এক বা দুইটি pull request-এ review করা যায়। “ARM64 support implement করা” এমন একটি অস্পষ্ট task নয়। এর বদলে:

- `target-aarch64-musl` toolchain setup;
- cross-build smoke test for `nilinit`;
- image manifest arch validation;
- ARM64 kernel checksum enforcement;
- mkfs-formatted data image;
- QEMU serial boot harness;
- core service readiness protocol;
- data persistence reboot test;

এসব আলাদা, measurable task হবে। প্রতিটি PR-এ expected behavior, failure modes, test command, test output এবং কোন maturity row পরিবর্তিত হয়েছে তা লিখতে হবে।

## ৩.২ Branch এবং release policy

`main`-কে bootable prototype-এর reference branch রাখা হবে। `main`-এ merge হওয়ার আগে ন্যূনতম cargo tests, clippy, Python builder tests ও x86_64 QEMU gate পাস করবে। ARM64 support যোগ করার পরে ARM64 build ও QEMU test-কে main gate-এ আনা হবে। Native device image তৈরির PR-এর ক্ষেত্রে device-independent tests-এর পাশাপাশি target-profile validation দরকার হবে।

প্রস্তাবিত workflow:

- ছোট feature: feature branch → tests → PR review → merge;
- architecture/build change: x86_64 + ARM64 QEMU matrix;
- security change: regression test + negative test + threat-model note;
- device port: hardware test log, recovery note এবং hardware-support matrix update;
- release: signed manifest + pinned toolchain + immutable checksums + recovery image + known issues list।

কোনও “green CI” badge-কে এমনভাবে ব্যবহার করা যাবে না যেন সব supported devices বা সব hardware feature পরীক্ষা করা হয়েছে। Badge-এর পাশে কী পরীক্ষা চলে তা সংক্ষেপে বোঝাতে হবে।

## ৩.৩ Evidence directory

`docs/evidence/`-এর মতো একটি পরিষ্কার structure যোগ করো, যেখানে প্রত্যেক target-এর ফল নথিবদ্ধ হবে:

```text
 docs/evidence/
 ├── qemu-x86_64/
 │   ├── build-manifest.json
 │   ├── boot-log.txt
 │   ├── service-health.json
 │   └── persistence-test.txt
 ├── qemu-aarch64/
 │   ├── build-manifest.json
 │   ├── boot-log.txt
 │   └── persistence-test.txt
 └── oneplus-fajita/
     ├── device-identity.md
     ├── boot-log.txt
     ├── hardware-matrix.md
     └── recovery-test.md
```

গোপন তথ্য, ব্যক্তিগত ফোন নম্বর, Wi-Fi password, IMEI/serial অথবা signing private key এই directory-তে থাকবে না। Log-এ sensitive data থাকলে sanitise করতে হবে। File name-এ commit SHA, target এবং তারিখ জুড়তে পারো; কিন্তু file name-ই evidence নয়—log-এ actual revision/target লিখতে হবে।

## ৩.৪ Maturity পরিবর্তনের নিয়ম

কোনও subsystem-এর maturity বাড়াতে হলে PR template-এ এই প্রশ্নগুলি থাকবে:

1. ঠিক কোন hardware/target-এ পরীক্ষা করা হয়েছে?
2. Test কি বাস্তব backend ব্যবহার করেছে, নাকি fake/mock?
3. কোন command/test case চালানো হয়েছে?
4. Success ও failure behavior কী ছিল?
5. Log/artifact কোথায় রাখা আছে?
6. Security permission/denial path পরীক্ষা হয়েছে?
7. Reboot, restart, suspend/resume অথবা lifecycle edge cases পরীক্ষা হয়েছে?
8. Feature unavailable হলে UI/service কী দেখায়?
9. README, `docs/maturity.toml`, `docs/reference-board.md` ও `docs/completion-checklist.md` একই তথ্য বলছে কি?

এই প্রশ্নগুলির উত্তর না থাকলে maturity status বাড়ানো হবে না।


# অংশ ৪ — Milestone 0: মূল build ও CI-কে নির্ভরযোগ্য করা

## ৪.১ লক্ষ্য

প্রথম milestone-এর লক্ষ্য নতুন feature তৈরি করা নয়। লক্ষ্য হলো এমন একটি পরিষ্কার engineering baseline তৈরি করা যার ওপর পরবর্তী কাজ দাঁড়াবে। এখন x86_64 QEMU CI পাস করছে—এটিকে regression-free রাখতে হবে। একই সঙ্গে code-quality test-এ global state race পুনরায় যেন না আসে, image builder host binary ভুল করে target rootfs-এ কপি না করে, kernel hash mismatch-এ build যেন থেমে যায় এবং success output যেন প্রকৃত success-কে প্রতিফলিত করে তা নিশ্চিত করতে হবে।

## ৪.২ JNI global-state test isolation

`android-host/src/jni_bridge.rs`-এ `GLOBAL_COMMAND_QUEUE`, `GLOBAL_EVENT_QUEUE`, `GLOBAL_CAMERA_FRAMES`, `GLOBAL_AUDIO_PLAYBACK`, `GLOBAL_AUDIO_RECORD`, global input এবং frame state আছে। Integration runtime-এ global singleton গ্রহণযোগ্য হতে পারে, কিন্তু unit test-গুলো একসঙ্গে চললে test fixture একই queue-তে একে অন্যের item দেখতে পারে। সাম্প্রতিক fix-এ test lock এবং poisoned mutex recover করার helper আছে। এটি race কমায়; তবু test design-কে আরও স্বাধীন করা উচিত।

দুটি test layer আলাদা করো:

- pure logic tests: queue struct বা buffer object-কে dependency injection দিয়ে instantiate করবে, global singleton স্পর্শ করবে না;
- JNI export integration tests: একটি global lock-এর অধীনে চলবে, আগে state reset করবে এবং শেষে cleanup করবে।

দীর্ঘমেয়াদে static `GLOBAL_*`-এর বদলে `BridgeState` structure তৈরি করে তার মধ্যে queue, frame, audio buffer এবং event list রাখো। Production JNI layer একটি process-wide `OnceLock<BridgeState>` ব্যবহার করতে পারে, অথচ unit test নিজের `BridgeState` তৈরি করবে। এতে test ordering আর shared state-এর উপর নির্ভরতা কমবে।

প্রয়োজনীয় পরীক্ষা:

```powershell
cargo test -p android-host -- --test-threads=1
cargo test -p android-host
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -W clippy::all
```

এগুলি শুধু developer machine-এ একবার চালানো যথেষ্ট নয়। GitHub Actions-এ একই commit-এ test parallel mode-এ চলবে। একশো বার test চালানো বাধ্যতামূলক নয়, তবে stress CI job বা scheduled job-এ test command কয়েকবার পুনরাবৃত্তি করালে flaky behavior ধরা সহজ হবে। Test failure-এর ক্ষেত্রে panic message, queue length এবং কোন fixture state reset হয়নি তা দেখাতে হবে; real phone number বা ব্যক্তিগত তথ্য log করা চলবে না।

## ৪.৩ Host ও target artifact-এর পৃথকতা

`build/mkinitramfs.py`-তে host fallback-এর ঝুঁকি সবচেয়ে আগে ঠিক করতে হবে। Target archive বানানোর সময় manifest-এ একটি নির্দিষ্ট architecture লেখা থাকবে। প্রতিটি executable কপি করার আগে executable-এর architecture একই কি না যাচাই করা হবে। Linux host-এ `file`, ELF header parser বা `readelf` দিয়ে যাচাই করা যায়; Windows development host-এর জন্য Python ELF-header parser বা CI-তে যাচাইয়ের ব্যবস্থা রাখতে হবে।

প্রস্তাবিত behavior:

- `--arch x86_64` হলে কেবল `x86_64-unknown-linux-musl` release directory গ্রহণ;
- `--arch aarch64` হলে কেবল `aarch64-unknown-linux-musl` release directory গ্রহণ;
- target binary missing হলে build থামবে; host-native `target/release` automatic fallback হবে না;
- development-এর জন্য fallback লাগলে `--allow-host-binaries-for-tests`-এর মতো explicit non-release flag থাকবে, যা release manifest-এ `non_release=true` বসাবে এবং CI release gate তা প্রত্যাখ্যান করবে;
- initramfs-এর সব ELF file-এ architecture validation হবে, শুধু `nilinit`-এ নয়।

একটি manifest উদাহরণ:

```json
{
  "target": "aarch64-qemu",
  "userspace_triple": "aarch64-unknown-linux-musl",
  "kernel_arch": "aarch64",
  "build_revision": "<git-sha>",
  "non_release": false,
  "binaries": {
    "nilinit": {"path": "usr/bin/nilinit", "sha256": "...", "elf_machine": "AArch64"}
  }
}
```

এটি illustrative schema; implementation-এ naming একবার স্থির করে সব tool-এ একই রাখতে হবে।

## ৪.৪ Kernel digest verification fail-closed করা

`ARCH_CONFIGS`-এ x86_64 ও aarch64 kernel-এর SHA-256 pin এসেছে। কিন্তু `ensure_kernel()`-এ ফাইলের size যথেষ্ট বড় হলে digest calculate করা হয়; digest মিলে না গেলেও বর্তমানে function ফেরত যেতে পারে। Download-এর পরেও mismatch হলে hard failure বাধ্যতামূলক করতে হবে।

সঠিক logic:

1. file absent হলে download, যদি download enabled থাকে;
2. download শেষে expected digest আবশ্যিক হলে hash match যাচাই;
3. mismatch হলে file quarantine/delete করে non-zero exit;
4. existing cached file-ও প্রত্যেক run বা cache validation-এর সময় hash-check;
5. expected digest absent থাকলে release build ব্যর্থ; developer override ব্যবহার করলে output স্পষ্টভাবে untrusted/development-only;
6. URL ও version source manifest-এ থাকবে, কিন্তু URL-কে integrity protection ধরে নেওয়া যাবে না;
7. kernel binary-এর version/build metadata ও checksum একই artifact manifest-এ লিখতে হবে।

Test-এর জন্য temporary fake kernel file তৈরি করে wrong bytes দিলে `ensure_kernel()` ব্যর্থ হওয়া যাচাই করবে। ঠিক hash-সহ fixture দিলে success হবে। Hash update করার সময় release/source URL, architecture এবং provenance note যাচাই ছাড়া SHA বদলানো যাবে না।

## ৪.৫ Generated artifact cleanup

রিপোজিটরির recursive tree-তে `.gitignore`-এ ignore করা সত্ত্বেও `android-host/app/.gradle/`, `android-host/app/build/`, `android-host/Onuron.apk`-এর মতো পুরোনো generated files এখনও আছে। Ignore rule tracked file-কে নিজে থেকে untrack করে না।

কাজ:

1. `git ls-files android-host/app/.gradle android-host/app/build android-host/Onuron.apk` দিয়ে tracked path তালিকা বের করো।
2. প্রয়োজনীয় generated files-কে `git rm --cached` দিয়ে index থেকে সরাও, কিন্তু developer-এর local files মুছে ফেলার প্রয়োজন নেই।
3. Gradle wrapper jar ও scripts legitimate source artifacts হিসেবে থাকবে; সেগুলোকে blanket ignore করা যাবে না।
4. `local.properties`-এ developer-specific SDK path রাখা যাবে না।
5. Debug APK-র জন্য GitHub workflow artifact ব্যবহার করো; repository-র tracked binary নয়।
6. `git status --ignored` ও fresh clone test দিয়ে নিশ্চিত করো যে build-এর পরে working tree clean থাকে।

এতে repository diff ছোট হবে, personal machine paths leak হবে না এবং Android build reproducibility যাচাই করা সহজ হবে।

## ৪.৬ Build script-এর error contract

প্রতিটি builder script-এর contract লিখতে হবে: input কী, output কী, কখন non-zero exit দিতে হবে, কোন tool আবশ্যক, কোন output destructive, এবং কোন files overwrite হবে।

`build/build.sh`-এ `set -euo pipefail` আছে, কিন্তু কিছু tolerated `|| true` এখনও optional component বা copy step-এর ফল আড়াল করতে পারে। প্রতিটি `|| true` পর্যালোচনা করো। Optional component হলে warning ও manifest-এ omission record করো; required component হলে build fail করো। “Build completed successfully” শুধু তখন হবে যখন output existence, type, architecture, checksums, rootfs binary list এবং packaging verification সব পাস করবে।

Builder শেষে একটি validation script চালাও:

- expected output files উপস্থিত;
- image files zero-filled placeholder নয়;
- kernel expected architecture;
- initramfs archive extract/parse করা যায়;
- `/init` executable আছে;
- core service executable আছে;
- `services.toml`-এর required binaries rootfs-এ আছে;
- `/data` image-এ filesystem signature আছে;
- boot args documented;
- manifest checksums সঠিক;
- image build log success marker কেবল সব check শেষে।

## ৪.৭ Milestone 0 acceptance criteria

- [ ] `cargo test --workspace --all-targets` clean checkout-এ pass;
- [ ] `cargo clippy --workspace --all-targets -- -W clippy::all` pass;
- [ ] `android-host` test parallel ও sequential উভয় mode-এ pass;
- [ ] `mkinitramfs.py` wrong-architecture binary reject করে;
- [ ] wrong kernel SHA-256 hard failure দেয়;
- [ ] generated Android artifacts আর tracked নয়;
- [ ] image builder missing required output-এ non-zero exit দেয়;
- [ ] build manifest-এ target triple, build SHA, kernel hash, initramfs hash ও binary list আছে;
- [ ] x86_64 QEMU CI আগের মতো পাস করে;
- [ ] README ও docs outdated claim-এ সংশোধন হয়।

---

# অংশ ৫ — Milestone 1: ARM64 QEMU-কে সত্যিকারের target করা

## ৫.১ Build matrix আলাদা করো

একটি generic build script সব architecture-কে magic label দিয়ে সমাধান করবে না। Canonical target ID নির্ধারণ করো:

- `qemu-x86_64` — host/kernel/userspace x86_64;
- `qemu-aarch64` — QEMU `virt`, AArch64 kernel ও AArch64 userspace;
- `android-host-arm64` — Android APK-তে অন্তর্ভুক্ত `arm64-v8a` JNI library;
- `oneplus-fajita` — ভবিষ্যৎ native device-specific profile;
- অন্য device profile পরে, reference phone support স্থিতিশীল হওয়ার পর।

`arm64-generic`, `aarch64-generic`, `aarch64-qemu` নামে তিনটি আলাদা target যেন অর্থহীনভাবে একই codepath-এ map না করে। Compatibility alias রাখতে পারো, কিন্তু canonical internal target name একটিই হবে। CI artifact, docs, output path, manifest এবং test logs-এ একই identifier ব্যবহার করো।

## ৫.২ Toolchain setup

Linux userspace-এর জন্য `aarch64-unknown-linux-musl` target install করো। Cross-linking-এর জন্য musl cross compiler/target linker প্রয়োজন হতে পারে; Rust target install করলেই সব native dependency cross-link হবে না। `setup-toolchain.sh`-এ host OS অনুযায়ী dependency check রাখো, কিন্তু target linker detection fail করলে build-কে ভুলভাবে সফল দেখিও না।

Windows ব্যবহার করলে WSL2/Linux build environment দিয়ে ARM64 image build করাটা প্রথম পর্যায়ে সহজ হতে পারে। Windows PowerShell script launcher হিসেবে রাখা যেতে পারে, কিন্তু image creation tools, QEMU, musl toolchain ও filesystem utility version pin/check করতে হবে। Windows host-এর `cargo build --target aarch64-unknown-linux-musl` path সত্যিই supported কিনা separately validate না হওয়া পর্যন্ত WSL2-কে official route হিসেবে লিখো।

Build command-এর প্রত্যাশিত রূপ:

```bash
rustup target add aarch64-unknown-linux-musl
cargo build --release --workspace --target aarch64-unknown-linux-musl
python3 build/mkinitramfs.py --arch aarch64
```

তবে এই command sequence-কে final বলে ধরে নিও না: initramfs builder-কে build orchestration-এর অংশ করা হলে duplicate build এড়ানো যায়। ভালো interface হবে `build-target.py --target qemu-aarch64` যা toolchain check, cross-build, kernel verification, rootfs stage, filesystem create, manifest generation এবং local QEMU boot test-কে এক ক্রমে চালাবে।

## ৫.৩ QEMU `virt` device map

ARM64 QEMU `virt` machine-এ hardware abstraction বাস্তব ফোনের SoC-এর অনুকরণ নয়। এটি ARM64 CPU, virtual interrupt/controller, RAM, serial console, VirtIO block/network এবং VirtIO GPU/input-এর মতো পরীক্ষার উপযোগী ডিভাইস দেয়। এই target দিয়ে kernel boot, init, storage, networking stack-এর কিছু অংশ এবং userland architecture validate হবে। Qualcomm-specific camera/modem/GPU driver কিংবা OnePlus-র PMIC/power behavior এখানে validate হবে না।

Reference file-এ QEMU invocation স্পষ্টভাবে রেকর্ড করো:

- QEMU binary/version;
- machine `virt` ও CPU profile;
- RAM ও vCPU count;
- kernel SHA-256 ও source/version;
- initramfs SHA-256;
- command line;
- disk file, disk format এবং filesystem UUID;
- network mode;
- serial log path;
- expected boot markers;
- test timeout;
- exit condition ও cleanup।

GUI mode ও headless mode-এর argument conflict দূর করতে হবে। `-nographic -serial mon:stdio` এবং GUI mode-এ `-serial stdio` ব্যবহারের ফলে port duplicate/console routing সমস্যা হচ্ছে কি না পরীক্ষা করো। CLI option parse-এ `--no-rebuild`, `--no-disk`, `--no-net`, `--gui`, memory, CPU ও SMP value tests যোগ করো।

## ৫.৪ ARM64 kernel configuration ও runtime assumptions

Kernel binary ডাউনলোড হচ্ছে বলে kernel configuration যথেষ্ট, এমন অনুমান করা যাবে না। Upstream Alpine kernel-এর configuration-এ ARM64 QEMU `virt`-এর প্রয়োজনীয় virtio devices ও filesystem support আছে কি না যাচাই করো। Rootfs RAM-এ থাকে বলে initial root mount, console এবং userspace interpreter/static linking requirements খেয়াল রাখতে হবে। Linux musl static binary build করলেও crate feature বা external dynamic library থাকলে runtime dependency থাকতে পারে; `ldd`-জাতীয় tool দিয়ে static linkage অনুমান করে নেওয়া নয়, ELF interpreter এবং dependencies যাচাই করতে হবে।

`rdinit=/init` ব্যবহার করলে kernel initramfs-এ `/init` খুঁজে চালাবে। কিন্তু boot args-এ `root=/dev/ram0` যুক্ত করা প্রয়োজন কি না, ব্যবহৃত kernel/initramfs flow অনুযায়ী যাচাই করো। একটি mode-এ কাজ করে অন্য target-এ কাজ করবে ধরে নিও না। Kernel log-এ actual command line এবং successful exec evidence capture করবে।

## ৫.৫ ARM64 data image তৈরির সঠিক পদ্ধতি

Blank 512 MB raw file filesystem নয়। Data image বানানোর নির্ভরযোগ্য ধাপ:

1. target output directory create;
2. নির্ধারিত size-এর sparse বা fixed image তৈরি;
3. filesystem type স্থির করা—প্রোটোটাইপে ext4 অথবা ext2; পরে encryption/verity architecture আলাদা করে নির্ধারণ;
4. `mke2fs` বা সমতুল্য tool দিয়ে filesystem initialise;
5. filesystem UUID/label record;
6. image-এ filesystem signature যাচাই;
7. QEMU-তে VirtIO block device হিসেবে সংযুক্ত;
8. `nilinit` mount log-এ device/UUID/type যাচাই;
9. boot test-এ `/data` temporary `tmpfs` নয়, disk filesystem-এ mount হয়েছে তা নিশ্চিত;
10. write/reboot/read persistence test।

Image formatting host environment-এর উপর নির্ভর করলে build provenance-এ `e2fsprogs` version লিখতে হবে। Sparse image-এ `truncate` ব্যবহার করলে host disk space কম লাগতে পারে; কিন্তু file system structure তৈরি হয় না। Format tool না থাকলে script স্পষ্টভাবে fail করবে। Blank image দিয়ে silent tmpfs fallback হতে দেওয়া যাবে না।

Data persistence test-এর sequence হবে:

- প্রথম boot: unique test token লেখা;
- sync/fsync এবং clean shutdown অথবা controlled reboot;
- একই persistent image পুনরায় attach;
- দ্বিতীয় boot: token পড়ে expected value compare;
- optional negative case: wrong disk / missing disk যুক্ত করলে warning দেখা যায়;
- JSON result-এ data image hash, before/after token hash ও boot logs;
- test cleanup শুধু temporary test images মুছবে, developer-এর real disk image নয়।

## ৫.৬ ARM64 initramfs package validation

`prepare_rootfs(arch=...)`-এ binary list এখন x86_64 এবং ARM64 build-এর জন্য ব্যবহার হয়। তালিকাটি `services.toml`, shell launcher, essential daemons এবং command line tools-এর সঙ্গে এক source of truth-এ রাখো। Duplicate/unused binary list কমাও। Required set এবং optional set আলাদা করো।

Rootfs package validation:

- `/init`, `/bin/nilinit`, `/sbin/init` একই target architecture;
- `services.toml`-এর প্রত্যেক `exec` path আছে এবং executable;
- required service binary missing হলে fail;
- optional service missing হলে manifest-এ “not bundled” entry;
- symlink policy documented—যদি symlink support করা হয়, canonical path/loop validation থাকতে হবে; বর্তমান strict rejection-এর সঙ্গে builder ও filesystem layout consistent হতে হবে;
- file permissions deterministically set;
- fixed timestamp এবং sorted file order reproducibility বজায় রাখে;
- rootfs-এ `local.properties`, `.gradle`, signing private key, test token বা host secret ঢোকেনি;
- archive unpack/parse করে file table-এর expected paths যাচাই।

## ৫.৭ ARM64 smoke test: শুধু boot marker নয়

বর্তমান `build/qemu-smoke.py` x86_64 boot completion ও core-service health marker যাচাই করে। ARM64-এর জন্য একই code reuse করা যেতে পারে, কিন্তু target-specific serial console marker, kernel architecture, rootfs manifest, persistent disk এবং QEMU invocation যাচাই করতে হবে। একটি `qemu-smoke.py --arch aarch64` mode যোগ করো অথবা পৃথক `qemu-aarch64-smoke.py` রাখো। Shared code ব্যবহার করলে test helper-গুলো architecture-aware হবে এবং x86_64 test output হুবহু বদলাবে না।

Smoke test-এর expected output:

```text
[PASS] kernel architecture = aarch64
[PASS] initramfs architecture = aarch64
[PASS] PID 1 started
[PASS] /data mounted on persistent ext filesystem
[PASS] required services ready (N/N)
[PASS] user-space health endpoint round-trip
[PASS] data persisted across reboot
```

এখানে marker-গুলি উদাহরণ; প্রকৃত logging format একটি স্থির schema অনুযায়ী implement করো। Smoke test string-only heuristic-এ আটকে থাকবে না। Panic marker, timeout, missing service এবং “boot completion without core service evidence” সব failure হবে।

## ৫.৮ ARM64 CI matrix

`.github/workflows/linux-qemu.yml`-এ আলাদা architecture target যোগ করো। শুরুতে একই runner-এ x86_64 ও AArch64 userspace build হতে পারে; কিন্তু QEMU system emulator dependency explicit রাখতে হবে। ARM64 CI job-এ kernel file download-এ checksum pin enforcement, cross-build, initramfs package, static validation, QEMU boot, service health এবং persistence test চালাও। QEMU availability না থাকলে job skip করে green করা যাবে না।

CI artifacts-এ রাখবে:

- target build manifest;
- kernel/initramfs checksum;
- serial boot log;
- service health result;
- data persistence result;
- failure হলে rootfs manifest ও last boot log।

ARM64 CI target-এর প্রথম কয়েকটি run-এ failure আসা স্বাভাবিক। গুরুত্বপূর্ণ হলো failure লুকানো নয়; log পরিষ্কার করে fix commit যোগ করা। Release approval-এর আগে একই SHA-তে clean ARM64 build ও QEMU boot pাস করতে হবে।

## ৫.৯ Milestone 1 acceptance criteria

- [ ] clean checkout থেকে explicit ARM64 target build;
- [ ] host binary target rootfs-এ কপি হতে পারে না;
- [ ] wrong kernel hash build বন্ধ করে;
- [ ] `data.img`-এ valid filesystem signature আছে;
- [ ] ARM64 QEMU boot script flag tests পাস;
- [ ] QEMU-তে core services actual readiness-সহ start;
- [ ] `/data` সত্যিকারের disk-এ mount;
- [ ] reboot-এর পর test data ফিরে আসে;
- [ ] ARM64 CI job green এবং artifact আপলোড করে;
- [ ] docs স্পষ্ট করে বলে: ARM64 QEMU validated, physical ARM64 phone এখনও validated নয়।


# অংশ ৬ — Milestone 2: Samsung S25-এ hosted Android runtime

## ৬.১ hosted runtime-এর সংজ্ঞা স্থির করা

S25 হলো একটি বাস্তব Android phone, কিন্তু এখানে OnuronOS hosted mode-এ চলবে। Android kernel, Android system services, Android permission model, Android modem/radio stack এবং Android graphics/audio/camera framework-ই আসল device control করবে। Onuron app নিজস্ব UI দেখাবে এবং অনুমোদিত Android API-র মাধ্যমে কিছু hardware capability ব্যবহার করবে। এটি Onuron kernel boot নয়। UI-তে `OnuronOS` লেখা থাকলেও settings বা about screen-এ স্পষ্টভাবে `Hosted runtime — Android host` দেখাতে হবে।

এই পার্থক্য শুধু বিপণনের ভাষা নয়; architecture-ও আলাদা। Hosted mode-এ Android app-এর sandbox থেকে সাধারণভাবে `/dev/dri/card0`, modem device, kernel log বা raw input node access করা যাবে না। JNI library সেই অনুমতি নিজে থেকে পায় না। Hardware call করতে হলে Android framework API, permission, role, explicit intent বা system-privileged service দরকার হতে পারে। Native Rust library কেবল Android app-কে process-local computation/bridge দিতে পারে; সেটি Android app-কে root service বানায় না।

## ৬.২ প্রথম hosted milestone: Android UI demo

প্রথম লক্ষ্য হবে native Rust library ছাড়া বা feature flag off রেখে APK build/install হওয়া, UI render, touch event, app lifecycle, battery indicator এবং browser/terminal mock experience পরিষ্কারভাবে কাজ করা। এই APK-কে demo বলা হবে। Demo mode-এ যেসব screen fabricated value দেখায়, সেখানে simulation badge থাকবে। Demo mode-এর dialer যেন real call চলছে এমন ভান না করে; একটি `Open Android dialer` action হলে স্পষ্ট UI message থাকবে। Camera app test image দেখালে তা `test pattern` হিসেবে চিহ্নিত হবে।

PowerShell-এ project setup যাচাই করতে হবে:

```powershell
cd android-host\app
.\gradlew.bat --version
.\gradlew.bat clean assembleDebug
```

APK output `app/build/outputs/apk/debug/app-debug.apk`-এর মতো Gradle path-এ থাকবে। Directory বা path বাস্তবে build configuration অনুযায়ী যাচাই করতে হবে; generated APK tracked repository-তে নয়, CI artifact হিসেবে রাখতে হবে।

ফোনে install করার আগে:

```powershell
adb devices
adb install -r .\build\outputs\apk\debug\app-debug.apk
adb logcat -s OnuronOS OnuronBridgeService OnuronNativeBridge
```

এই command-গুলি illustrative; actual APK path output থেকে নিতে হবে। ফোনে USB debugging অনুমোদন দিতে হবে। একই `applicationId` কিন্তু অন্য signing key-তে তৈরি APK update না হলে পুরোনো app uninstall করার আগে user data/setting দরকার কি না যাচাই করবে। প্রথম install-এর জন্য personal data মুছে ফেলার প্রয়োজন নেই।

## ৬.৩ Rust native library build ও Gradle integration

`android-host/build-ndk.sh`-কে build workflow-এর সুনির্দিষ্ট অংশ বানাতে হবে। একটি design হলো Gradle-এ `Exec` task যুক্ত করে `preBuild`-এর আগে Rust library build করা। আরেকটি হলো CI script-এ Rust library build করে `assembleDebug` চালানো। দুটো পদ্ধতিই গ্রহণযোগ্য, কিন্তু developer workflow এবং CI-তে একই command contract ব্যবহার করতে হবে।

প্রস্তাবিত staged path:

```text
android-host/
├── Cargo.toml
├── build-ndk.sh
└── app/src/main/jniLibs/arm64-v8a/libandroid_host.so
```

Build requirements:

- NDK version pin;
- Rust toolchain channel/version pin বা controlled stable policy;
- `aarch64-linux-android` Rust target install;
- cargo-ndk version pin অথবা target linker environment explicit;
- Android API level স্থির করা;
- `.so` ELF machine type AArch64 তা যাচাই;
- exported JNI symbols expected symbol list-এর সঙ্গে তুলনা;
- build log-এ source commit, NDK version, Rust version, Android API level;
- build fail হলে Gradle APK packaging চলবে না;
- native library না থাকলে demo build ও native-required release build আলাদা variant হবে।

`NativeBridge` বর্তমানে `libnilhal` আগে এবং `libandroid_host` পরে load করার চেষ্টা করে। কোন library কোন JNI export দেয়, এবং `System.loadLibrary()`-এর নাম artifact filename-এর সঙ্গে কীভাবে মেলে, তা একবার নির্ধারণ করতে হবে। Rust crate `crate-type = ["cdylib", "rlib"]` দিয়ে `libandroid_host.so` তৈরি করতে পারে। যদি `nilhal` আলাদা `.so` না বানানো হয়, তবে Java-র প্রথম library load fail করা expected behavior—কিন্তু log level ও user-visible status confusing হবে। Native architecture finalise করে loader fallback deterministic করো।

## ৬.৪ JNI contract ও memory safety

JNI boundary-তে Java object/array/string-কে Rust pointer হিসেবে নেওয়া বা ফেরত দেওয়ার সময় memory lifetime ও JNI ownership গুরুত্বপূর্ণ। `JNIEnv*`, `jclass`, `jobject`, `jbyteArray`, `jshortArray`, `jintArray`, `jstring`—এসব type-এর ABI declaration Java method signature-এর সঙ্গে হুবহু মেলাতে হবে। C-style exported symbols-এ Rust raw pointer ব্যবহার করা গেলে pointer length, null check, array region copy, UTF-8 conversion, local reference lifetime এবং Java exception handling লিখিতভাবে নির্ধারণ করো।

প্রস্তাবিত test layers:

1. Rust pure function test — host JSON থেকে internal enum decode;
2. JNI export unit test — null inputs, zero-length array, invalid UTF-8/JSON, oversize length;
3. Android instrumentation test — Java `NativeBridge` থেকে real native symbol invoke;
4. device smoke test — APK install, native load, handshake, surface creation, sample frame;
5. lifecycle test — Activity pause/resume, screen rotation, service stop/restart, process death;
6. stress test — repeated frame queue, bounded memory, audio queue overflow, repeated camera requests।

JNI function-এ Java-র দাবি করা `maxLen` negative হতে পারে বা actual array length-এর চেয়ে বড় হতে পারে; Rust-কে `maxLen` trust করে raw slice বানাতে দেওয়া যাবে না। Array length Android/JNI runtime থেকে validate করতে হবে এবং expected maximum enforce করতে হবে। Frame width × height multiplication overflow check করতে হবে। 1080×2340 RGBA frame প্রায় 10 MB, প্রতি frame-এ full-copy করলে 60/120 FPS-এ বড় memory bandwidth খরচ হয়; frame buffer size, pixel format, synchronization ও reuse plan মাপতে হবে।

## ৬.৫ Frame rendering pipeline

বর্তমান Rust `AndroidHostDisplay.present_frame()` `jni_bridge::publish_display_frame()`-এ raw `u32` pixel buffer পাঠায় এবং cache রাখে। Java side-এ `NativeBridge.getLatestFrame(int[])` wrapper আছে, কিন্তু UI frame সত্যিই সেখান থেকে পড়ে `Surface`-এ present করছে কি না তা যাচাই করতে হবে। Unit test-এ buffer publication সফল হওয়া display hardware-এ frame দেখানোর প্রমাণ নয়।

Minimum pipeline:

1. Android `SurfaceView`/`Surface` lifecycle callback `surfaceCreated`, `surfaceChanged`, `surfaceDestroyed`;
2. actual surface dimension and density read;
3. Rust renderer frame buffer generate;
4. native buffer to Java/shared buffer copy;
5. Java `Canvas` বা NDK `ANativeWindow`/graphics route-এ display;
6. pixel format/channel order/stride orientation validate;
7. UI thread/RenderThread synchronization;
8. frame drops এবং render duration metric;
9. rotation/recreation-এর পরে old surface handle ব্যবহার না করা;
10. display unavailable হলে clean error এবং UI fallback।

শুরুতে Java Canvas route যথেষ্ট হতে পারে; পরে performance measurement অনুযায়ী `Surface.lockCanvas`, `ANativeWindow` অথবা GPU path বেছে নাও। “120 Hz” static constant ধরে নেওয়া যাবে না। S25-এ dynamic refresh mode থাকে; hosted app system refresh control-এর পূর্ণ নিয়ন্ত্রণ পায় না। Surface dimension hardware resolution-এর চেয়ে window size/density value হতে পারে। `width=1080`, `height=2340`, `refresh_rate=120`-কে model-specific hard-code না করে actual host metrics থেকে নাও এবং display metadata-তে source লিখো।

## ৬.৬ Touch/input event flow

`MainActivity`-এর touch event `NativeBridge.onHostTouchEvent`-এ যায়, Rust `AndroidHostInput`-এ ingest হয়—এমন একটি route থাকা দরকার। Input event-এ coordinate system একটি নির্দিষ্ট policy মেনে চলবে: Android View pixel coordinates, density-independent logical coordinates, compositor framebuffer coordinates-এর মধ্যে explicit transform থাকবে। Touch gesture event-এ action code, pointer ID, x/y, pressure, event time এবং cancellation type লাগতে পারে। Multi-touch, pointer-up, rotation, stylus, keyboard, edge swipe এবং app pause-এ active touch cancel handling পরীক্ষা করতে হবে।

Acceptance test:

- screen top-left/center/bottom-right tap থেকে expected logical coordinate আসে;
- pointer down/move/up এক gesture হিসাবে নির্ভুলভাবে আসে;
- cancellation-এ stuck pressed button থাকে না;
- screen rotation/resize-এর পরে calibration সঠিক;
- multi-touch pointer ID একে অন্যকে overwrite করে না;
- input event queue bounded বা overflow behavior documented;
- host app background-এ গেলে Onuron UI active touch state reset করে।

## ৬.৭ Android permissions ও user consent

Manifest permission যোগ করা runtime permission grant-এর সমান নয়। `CAMERA`, `RECORD_AUDIO`, `SEND_SMS`, `CALL_PHONE`, phone state, Wi-Fi পরিবর্তন এবং notification-এর জন্য Android API level, user role ও permission restrictions পর্যালোচনা করতে হবে। `FLASH_LIGHT` permission name/version-সংক্রান্ত assumptions যাচাই করো; torch-এর জন্য CameraManager use-case এবং `CAMERA` permission/device capability বিবেচনা করতে হবে।

এমন command dispatcher বানানো চলবে না যেখানে arbitrary JSON command নিঃশব্দে `SmsManager.sendTextMessage()` চালিয়ে দেয়। Hosted runtime user-এর দৃশ্যমান UI থেকে explicit action তৈরি করবে; sensitive action-এর আগে confirmation বা OS permission চাইবে। SMS/call functionality-তে number validation, URI encoding, duplicate submission, timeout, cancel, failure callback ও sent/delivered status আলাদা থাকবে। `Intent.ACTION_DIAL` শুধু dialer খুলেছে; কল সত্যিই active হয়েছে এ দাবি করা যাবে না। SMS composer খোলা আর message পাঠানো আলাদা status।

প্রস্তাবিত security behavior:

- bridge server/service শুধুমাত্র app-internal, non-exported endpoint;
- JSON command schema versioned, bounded, typed এবং allowlisted;
- `action` name অজানা হলে reject;
- recipient/number-length validation;
- log-এ SMS body/phone number redact;
- sensitive operation-এর response ID/correlation ID;
- duplicate request detection এবং timeout;
- runtime permission denied হলে fake success নয়, typed error;
- Android restricted API থেকে operation সম্ভব না হলে feature unavailable status;
- test environment-এ mock backend, কিন্তু production UI-তে mock data visibly simulated।

## ৬.৮ Camera pipeline

`android-host/src/camera.rs` real JPEG frame queue-তে frame এলে সেটি ব্যবহার করে; না পেলে `VALID_BASELINE_JPEG` return করে। এই fallback unit tests-এর জন্য গ্রহণযোগ্য, কিন্তু camera app-কে real picture দেখাচ্ছে বলে দাবি করা যাবে না। Production mode-এ mock fallback disable করো; development demo mode-এ explicit `Camera backend: Test pattern` দেখাও।

Android side-এ Camera2 sequence:

1. runtime `CAMERA` permission;
2. camera ID list থেকে capability query;
3. compatible preview/capture session;
4. `ImageReader`/`Image` lifecycle;
5. JPEG/YUV frame buffer copy এবং image close;
6. buffer JNI-তে bounded queue দিয়ে পাঠানো;
7. Rust consumer camera ID/frame timestamp/size validation;
8. timeout/no-frame error;
9. preview stop/Activity pause-এ session release;
10. torch state camera capability ও actual callback থেকে আপডেট।

Camera ID `0=rear wide, 1=front selfie, 2=ultrawide` ধরে নেওয়া যাবে না; Android device camera IDs arbitrary strings হতে পারে। Host layer actual camera ID string বা stable internal index mapping দেবে। S25-এ multiple rear sensors, logical multi-camera এবং API-specific settings থাকতে পারে; generic device-এর camera count কমও হতে পারে। API abstraction `CameraInfo`, `CameraCapabilities`, `CaptureResult`, `Frame` এবং `CameraError`-এর মতো typed object দিতে পারে।

Test matrix-এ front/rear camera, permission deny, camera busy, app background, low storage, rotate, frame timeout এবং camera unavailable অন্তর্ভুক্ত করতে হবে। Camera preview চললে memory bounded এবং thermal load observable হতে হবে। Photo capture সফল হওয়ার প্রমাণ JPEG byte length নয়; file decode করা যায়, dimensions expected, timestamp পাওয়া যায় এবং frame-এ সত্যিকারের sensor data আছে—এমন instrumentation test লাগবে।

## ৬.৯ Audio pipeline

Rust `AndroidHostAudio` PCM samples scale করে queue-তে রাখে। কিন্তু hardware playback `AudioTrack` দিয়ে না চললে output sample count মানে audible sound নয়। Android side-এ একটি playback worker `AudioTrack`-এ bounded buffer লেখে; record worker `AudioRecord` থেকে PCM পড়ে queue-তে দেয়। Sampling rate, channel count, PCM format, frame boundary, endianness, latency, underrun/overrun, mute এবং route change স্পষ্ট করতে হবে।

প্রথম implementation mono PCM16 48 kHz হতে পারে। পরে stereo/route adaptation দরকার হলে protocol version বাড়াবে। Java worker main thread-এ blocking I/O করবে না। Playback/record buffer-এ max capacity, backpressure policy এবং `AudioFocus` policy থাকবে। Microphone permission না থাকলে record function zero samples দিয়ে success না করে explicit permission error ফেরত দেবে। Silence এবং failed capture-কে একইভাবে দেখালে debugging কঠিন হবে; result-এ sample count, error code ও timestamp রাখতে হবে।

বাস্তব পরীক্ষা:

- known tone playback করে loopback recording বা external check;
- microphone sample non-zero হতে পারে, কিন্তু নীরব ঘরে zero sample বৈধও হতে পারে—তাই RMS-only assertion নয়, test tone/known stimulus ব্যবহার;
- mute on/off এবং volume change শুনে/measure করে যাচাই;
- AudioTrack initialization fail, focus loss, Bluetooth route change;
- Activity pause/resume, incoming host call, audio interruption;
- long playback memory/buffer leak এবং underrun count;
- 10-minute capture/playback soak test।

## ৬.১০ Network, battery, sensors ও connectivity

`AndroidHostNetwork`-এর আগের hard-coded IP/SSID/carrier demo state পুরোপুরি বাদ দিয়ে host event stream থেকে actual connectivity state derive করতে হবে। Android সাধারণ app হিসেবে Wi-Fi scan/connect সবসময় control করতে পারে না; OS policy ও location permission/API restrictions বিবেচনা করে actual capability report করো। UI-তে `Connected via Android host`, `Wi-Fi state unavailable`, `Access denied` অথবা `Simulated` আলাদা state রাখো।

Battery receiver-এ level/status পাওয়া গেলেও current, voltage, thermal, health এবং charge technology সব device/API-তে পাওয়া যাবে এমন নয়। যা unavailable, সেটাকে fake constant দিয়ে পূরণ করা যাবে না। Sensor pipeline-এ sensor type, value length, sampling rate, monotonic timestamp, unit এবং lifecycle registration/unregistration থাকতে হবে। GPS/cell location-এর মতো sensitive information explicit permission ও privacy policy ছাড়া collect/log করা যাবে না।

## ৬.১১ Android-host acceptance criteria

- [ ] source checkout থেকে clean APK build;
- [ ] Rust library reproducibly build ও APK-তে packaged;
- [ ] runtime linker JNI symbols load করে;
- [ ] display sample frame সত্যিই screen-এ দেখা যায়;
- [ ] touch event round-trip test pass;
- [ ] camera sample fallback production variant-এ বন্ধ;
- [ ] camera capture Android Camera2 frame থেকে সত্যিকারের image দেয়;
- [ ] AudioTrack playback এবং AudioRecord capture actual phone-এ পরীক্ষা;
- [ ] call/SMS UI true status ও permission states report করে;
- [ ] network/battery/sensors real source-ভিত্তিক; mock visibly marked;
- [ ] Activity/service lifecycle crash/memory-leak test পাস;
- [ ] CI APK build artifact publish করে, tracked APK নয়;
- [ ] about screen-এ স্পষ্ট লেখা থাকে: Android-hosted Onuron runtime, native Onuron boot নয়।

---

# অংশ ৭ — Milestone 3: NilHAL-কে চুক্তি হিসেবে পরিণত করা

## ৭.১ কেন HAL design আগে দরকার

বিভিন্ন ফোনে একই Onuron core চলাতে HAL দরকার, কিন্তু HAL শুধু trait declaration-এর নাম নয়। প্রতিটি method-এর semantics, units, timeouts, error model, availability, lifecycle, permissions এবং cancellation behavior নির্দিষ্ট করতে হবে। `DisplayHal::present_frame()` `Ok(())` return করলেই frame present হয়েছে—এমন নয়। `NetworkHal::connect_wifi()` `Ok(())` মানে request enqueue হয়েছে, association হয়েছে, IP এসেছে, না internet connectivity আছে—তা method contract-এ স্পষ্ট না থাকলে UI ভুল state দেখাবে।

HAL-কে তিনটি layer-এ ভাগ করা উচিত:

1. **Domain interface:** OS services যে typed operation চায়;
2. **Backend implementation:** Linux native, Android host, QEMU virtual, fake test backend;
3. **Capability/status model:** কোন operation available, permission denied, temporarily busy, unsupported, failed বা successful।

একটি global `NilHal::auto()` initialization convenient হতে পারে, কিন্তু backend selection silent heuristic দিয়ে নয়, boot manifest/profile ও detected platform-এর verified data দিয়ে করা উচিত। Selected backend-এর identity log ও system diagnostics-এ দৃশ্যমান হবে। Auto-detection ব্যর্থ হলে fake backend-এ silently degrade করা যাবে না; developer demo mode-এ তা করা যায়, production build-এ error হতে হবে।

## ৭.২ Error type ও readiness

একটি `HalError` enum-এ অন্তত এই ক্ষেত্রগুলি থাকা উচিত:

- `UnsupportedOperation`;
- `PermissionDenied`;
- `DeviceNotFound`;
- `Busy`;
- `Timeout`;
- `InvalidArgument`;
- `BackendUnavailable`;
- `IoError`;
- `ProtocolError`;
- `NotReady`;
- `Cancelled`।

Error string মানব-পাঠ্য হবে, কিন্তু caller শুধু string parse করে program flow করবে না। প্রতিটি error-এ subsystem, operation, recoverability এবং optional OS error code থাকতে পারে। Readiness আলাদা হবে: `Unavailable`, `Initializing`, `Ready`, `Degraded`, `Failed`। উদাহরণস্বরূপ, camera permission না থাকলে camera backend failure না-ও হতে পারে; capability available, permission denied state হওয়া উচিত। Device-এ camera নেই হলে unsupported। Camera present কিন্তু অন্য app use করছে হলে busy।

## ৭.৩ DisplayHal

Display API-তে width/height শুধু static spec নয়, বর্তমান logical surface dimensions। Refresh rate dynamic হতে পারে। Brightness command requested value, actual applied value এবং unsupported status আলাদা করবে। `present_frame()`-এর buffer format (ARGB8888/BGRA8888/RGB565), stride, dimensions, color space ও ownership lifetime specify করো। Zero-sized buffer বা dimension overflow invalid। Present complete/vsync feedback চাইলে async present API দরকার হতে পারে।

Linux backend-এ `/dev/fb0` বা DRM/KMS comment রেখে `Ok(())` return করা production-ready নয়। হয় real device node খুলে map/present করবে, অথবা `UnsupportedOperation`/`BackendUnavailable` ফিরবে। QEMU backend-এর virtual display support বাস্তবে থাকলে supported status; না থাকলে fake backend visibly indicated থাকবে। Android backend actual Surface route ব্যবহার করবে।

## ৭.৪ InputHal

Input event model-এ touch, key, stylus, pointer, gesture এবং cancellation semantics দরকার। Raw device event এবং high-level UI event আলাদা করা ভালো। Linux native route-এ `/dev/input/event*` পড়া, permissions, `libinput` বা direct evdev selection, device discovery, event loop এবং hotplug handling থাকবে। In-memory `Vec<HalInputEvent>` queue test backend-এ ঠিক আছে; production `LinuxInput.poll_events()`-এর default path হিসেবে ব্যবহার করা যাবে না।

একটি `InputDeviceInfo` enumeration endpoint যোগ করো। Device ID stable হওয়ার নিশ্চয়তা না থাকলে boot থেকে boot-এ একই string ধরে নেবে না। Absolute/relative coordinates, scaling, axis min/max, pressure, multitouch slot ও orientation calibration device metadata থেকে পড়া হবে। Input service crash হলে touch pipeline graceful recovery দেখাবে; UI stuck state-এ থাকবে না।

## ৭.৫ NetworkHal

Network API-তে `get_state`, `scan_wifi`, `connect_wifi`, `set_cellular_enabled`-এর semantics স্পষ্ট করতে হবে। Wi-Fi connect request asynchronous হওয়া উচিত, কারণ association, DHCP, DNS ও internet reachability এক ধাপ নয়। `connect_wifi()` accepted request ID দিতে পারে; পরে event stream-এ authenticating/associated/address acquired/connected/failed state জানাবে। Password logs-এ যাবে না; config storage encryption policy-র সঙ্গে সামঞ্জস্য রাখতে হবে।

Android-host backend ConnectivityManager-ভিত্তিক read-only actual state দিতে পারে; system privileged API ছাড়া Wi-Fi enable/disable বা scan restricted হতে পারে। Linux backend-এ `NetworkManager`, `iwd`, `wpa_supplicant` বা একটি নির্দিষ্ট service model বেছে নিতে হবে; একই সঙ্গে একাধিক network manager যেন interface control-এর জন্য প্রতিযোগিতা না করে। Network namespace isolation থাকা apps-এর জন্য policy-based mediation লাগবে, কেবল app permission list পড়া নয়।

## ৭.৬ PowerHal ও sensors

Power HAL-এ battery capacity, status, charging source, voltage/current/temp availability, screen timeout, wakelock এবং performance policy অন্তর্ভুক্ত হতে পারে। Missing sysfs node-এ fake battery percentage নয়—`Unavailable` বা source metadata দিতে হবে। QEMU-তে battery নাও থাকতে পারে; QEMU UI-তে battery icon synthetic হলে সেটি test fixture হিসাবে mark হবে।

Suspend/resume native phone-এ power-management-এর সবচেয়ে ঝুঁকিপূর্ণ অংশগুলির একটি। `write /sys/power/state`-এর মতো raw operation-কে সরাসরি Settings UI থেকে invoke করো না। `powerd` lifecycle coordination, wake lock, alarms, filesystem sync, network policy এবং wake reason পরিচালনা করবে। প্রথম hardware milestone-এ suspend test বাদ রাখা যেতে পারে; কিন্তু release claim-এ suspended state কাজ করে না বা unvalidated তা লিখতে হবে।

## ৭.৭ CameraHal, AudioHal, TelephonyHal

এই তিনটি HAL-এ placeholder success সবচেয়ে বিপজ্জনক, কারণ caller বাস্তব action হয়েছে ধরে নিতে পারে। `CameraHal::capture_frame()`-এ fallback JPEG শুধু explicit test fixture backend-এর অংশ হবে। Linux camera backend frame stream ഇല്ലെങ്കে `DeviceNotFound` বা `BackendUnavailable` ফিরবে। `AudioHal::play_stream()` real output device বা Android AudioTrack-এ queue না দিলে সফল হবে না। Recording empty buffer দিতে পারে, কিন্তু microphone unavailable ও silence আলাদা state। Telephony `dial()`-এ synthetic `call_id` দেওয়া যেতে পারে internal request handle হিসেবে, কিন্তু state `Active` হবে কেবল real host/modem callback থেকে।

Telephony state machine: `Idle` → `Dialing` → `Ringing`/`Active` → `Disconnected`/`Failed`. Incoming call state `Ringing`; answer accepted command আর call active হওয়ার event আলাদা। SMS state `Queued`, `Submitting`, `Sent`, `Delivered` (যদি supported), `Failed`; provider callback ছাড়া delivered বলা যাবে না। Android-host mode OS dialer খুললে state হবে `ExternalDialerOpened`, active call নয়। Native Linux modem integration পরে বাস্তব ModemManager/Telephony stack-এর ওপর নির্ভর করবে।

## ৭.৮ HAL conformance suite

প্রত্যেক backend-এর জন্য এক shared conformance suite তৈরি করো। Fake backend expected behavior model করবে; Linux/Android/QEMU backend একই protocol test চালাবে, কিন্তু hardware features unsupported হলে expected capability response compare হবে। Backend conformance suite-এ থাকবে:

- capability discovery;
- missing device behavior;
- permission denial;
- timeout;
- command cancellation;
- event queue ordering;
- resource cleanup;
- stale connection recovery;
- malformed input;
- state after resume;
- memory bounds;
- telemetry truthfulness।

Conformance suite-এর ফল `docs/evidence/<target>/hal-conformance.json`-এ লিখতে পারো। একটি test case skip হলে reason required; skip-কে pass হিসেবে গোনা যাবে না।

---

# অংশ ৮ — Milestone 4: App sandbox, permissions ও platform security

## ৮.১ Threat model লিখে শুরু করো

Sandbox code-কে production security claim-এ উন্নীত করার আগে threat model লিখতে হবে। OnuronOS-এ অন্তত চারটি trust boundary রয়েছে: app ↔ app, app ↔ system daemon, app ↔ host hardware, update/package ↔ OS image। প্রতিটি boundary-তে attacker কী করতে পারে, তার privilege কী, kernel/LSM-এর assumptions কী, এবং failure হলে কী ক্ষতি হতে পারে তা নির্দিষ্ট করা দরকার।

Threat model-এ বিবেচ্য attacker:

- malicious বা compromised `.nilax` app;
- unsigned বা untrusted publisher package;
- corrupted package download/cache;
- local user app-এর data পড়তে চেষ্টা করছে;
- compromised bridge app host OS-এর permission অপব্যবহার করছে;
- malformed IPC message পাঠানো local process;
- update image বা vbmeta বদলে দেওয়ার চেষ্টা;
- kernel/firmware bug কাজে লাগানো attacker;
- power loss বা storage corruption-এ update মাঝপথে থেমে যাওয়া।

এটি threat model থেকে অতি উচ্চ নিরাপত্তা claim বন্ধ করা নয়; বরং কোন security control কোন attacker-এর বিরুদ্ধে কাজ করে তা বোঝানো। Namespace + seccomp defence-in-depth, কিন্তু Linux kernel exploit আটকানোর নিশ্চয়তা নয়। Android-host runtime-এ host Android security model-এর সীমা স্পষ্টভাবে লিখতে হবে।

## ৮.২ Namespace sandbox

`runtime/nilrt/src/sandbox.rs`-এর isolation flags, mount private করা, `PR_SET_NO_NEW_PRIVS`, `pivot_root`/`chroot`, `/proc`, `/tmp`, `/sys` ও device masks—সবগুলি privilege ও error handling-সহ পর্যালোচনা করতে হবে। Namespace setup ব্যর্থ হলে production mode-এ direct unconfined execution যেন না হয়। CI fallback-কে environment variable দিয়ে অনুমোদন করা হলে সেটি শুধুমাত্র test runner-এ ব্যবহৃত হচ্ছে কি না নিশ্চিত করো। Release image-এ `CI=true` বা `NILRT_ALLOW_INSECURE_DEV=1` সেট হয়ে থাকলে package validation তা reject করবে।

বিশেষ করে `CLONE_NEWNET` ঢোকানোর পরে আরও পরীক্ষা দরকার:

- network namespace তৈরি হয়েছে কি না;
- loopback interface up/down state;
- app internet permission না পেলে external interface access হয় না;
- app-এর inherited file descriptor-এ host network socket আগে থেকেই আছে কি না;
- UNIX domain socket-এ system daemon access policy প্রয়োগ হয় কি না;
- app host DNS/control endpoint বা local proxy ব্যবহার করে permission bypass করতে পারে কি না;
- network-capable permission grant পেলে approved broker path কীভাবে network request allow করে;
- host networking direct access না দিলে documented, enforceable proxy/API route কী।

শুধু `CLONE_NEWNET` যোগ করলেই network অনুমতি পাওয়া app internet পাবে না। Namespace isolation প্রথমে network বন্ধ করে; অনুমোদিত network service, veth/NAT/proxy কিংবা broker-mediated API আলাদা design করতে হবে। UI-তে “network allowed” দেখানো এবং actual connectivity path না থাকা feature mismatch।

## ৮.৩ App UID registry

`allocate_app_uid_with_registry()` deterministic hash থেকে শুরু করে available slot probe করে। তবে concurrent installer/launcher দুটো একসঙ্গে registry পড়ে একই UID allocate করতে পারে যদি registry update file lock বা atomic transaction-সহ না হয়। JSON truncate-write crash করলে registry corrupt হতে পারে। `NIL_APP_UID` override-ও normal UID boundary bypass করার সম্ভাব্য পথ।

প্রস্তাবিত design:

- UID registry-এর single-writer lock (flock বা সমতুল্য) নিয়ে read/modify/write;
- temporary file → fsync → atomic rename;
- corrupt registry হলে fail closed; empty registry বানিয়ে existing installed app-এর UID collision সৃষ্টি নয়;
- allocate range স্থির; collision-free guarantee range exhaustion-এর ক্ষেত্রে explicit failure;
- system UID/GID reserved ranges থেকে পৃথক;
- package ID validation এবং install-time UID allocation atomic;
- deleted app UID recycle করার policy; recycle করলে stale process/data ownership ঝুঁকি;
- developer UID override শুধুমাত্র test-specific build flag/explicit unsafe mode-এ;
- registry ownership `root:root`, permissions restrictive;
- multi-process stress test, crash injection এবং duplicate app-ID test।

## ৮.৪ Permission broker

Permission broker-এর role হবে package manifest-এ requested permission, user/admin grant এবং real capability mediator-এর মধ্যে policy enforcement। `manifest.json`-এ `camera`, `network`, `storage.write`, `telephony`, `microphone` লেখা থাকলেই syscall নিজে থেকে restricted হয় না। প্রতিটি permission কী operation allow করে তার mapping দরকার।

উদাহরণ:

- `camera`: শুধু `camerad`/approved broker IPC; `/dev/video*` direct access নয়;
- `microphone`: `audiod` capture route-এ grant; `/dev/snd` direct raw access নয়;
- `network`: policy অনুযায়ী proxy/broker বা network namespace access;
- `storage.read`: নির্দিষ্ট app-private/shared path, whole `/data` নয়;
- `storage.write`: নিজের app data এবং explicit shared-storage grant;
- `location`: location service output; raw sensor/GNSS node নয়;
- `telephony`: call/SMS request user confirmation and OS/service policy-র অধীনে;
- `notifications`: অন্য app-এর private notification content read নয়;
- `clipboard`: explicit clipboard API, silent polling সীমিত।

`PermissionBroker` persistent grants-এর format versioned হওয়া উচিত। Corrupt database quarantine করা ভালো; তবে quarantine-এর পরে implicit permission grant করা যাবে না। Clock skew, expiry, permission revoke, app downgrade/upgrade, reinstall এবং package signature change-এর semantics নির্দিষ্ট করো। UI grant/revoke action-এর পরে daemon authorization cache invalidate হবে। Permission model-এর unit test-এর পাশাপাশি real sandbox test-এ denied operation সত্যিই ব্যর্থ হতে হবে।

## ৮.৫ Seccomp

`runtime/nilrt/src/seccomp.rs`-এর x86_64 ও AArch64 syscall allowlist-এর জন্য architecture-specific positive এবং negative tests দরকার। Syscall allowlist-এর সংখ্যা (যেমন “110 syscalls”) নিজে নিরাপত্তার প্রমাণ নয়। App-এর runtime ও libraries যে syscall ব্যবহার করে তার tracing থেকে list তৈরি করো; প্রতিটি allow rule-এর rationale লিখো। অতিরিক্ত broad `ioctl`, `socket`, `clone`, `ptrace`, `mount`, `bpf`, `perf_event_open`, `keyctl` বা privileged syscall থাকলে risk review করো।

Seccomp filter activation failure হলে app unconfined অবস্থায় execute হতে পারবে না। App-specific feature-এর জন্য syscall দরকার হলে policy capability profile দিয়ে অনুমোদন হবে, developer-নির্ভর ad-hoc string নয়। `seccomp` audit mode test environment-এ ব্যবহার করা যায়; release mode-এ enforcement required হলে fail-closed policy enforce করো। ARM64-এর test অবশ্যই ARM64 kernel/VM-এ চালাও; x86 host-এ pure decision logic test ARM64 kernel enforcement প্রমাণ করে না।

## ৮.৬ SELinux বাস্তবায়ন

`runtime/nilrt/src/selinux.rs`-এ exec context লেখার path যোগ হয়েছে; তবু `nilinit::load_selinux()`-এ policy-load file open mode ও error handling যাচাই করা দরকার। Policy compiler failure `security/selinux/build.sh`-এ success হিসাবে গ্রহণ করা চলবে না। `security/selinux/ci/audit.sh`-এর regex scan real policy `neverallow` semantics প্রমাণে সীমাবদ্ধ; compiler validation ও runtime enforcement test চাই।

প্রস্তাবিত কাজের ক্রম:

1. `secilc`-এর version pin বা supported range;
2. policy CIL compile করে artifact তৈরি;
3. `secilc` failure হলে non-zero exit;
4. generated policy version ও policy checksum manifest-এ রাখা;
5. rootfs-এ file context ও label rules install;
6. init-এ policy load success/failure আলাদা log;
7. `/sys/fs/selinux/enforce` বা সমমানের runtime state read;
8. actual process label query;
9. denied operation trigger করে AVC evidence;
10. expected allowed/denied test suite;
11. policy update backward compatibility ও migration test।

Policy audit script-কে static text matcher নয়, test runner হিসেবে উন্নত করো। Test policy-তে ইচ্ছাকৃত violation inject করে actual compiler/failure detector তা ধরে কি না পরীক্ষা করবে। Temporary file সবসময় cleanup হবে; concurrent CI job-এর shared policy tree-তে test fixture inject করলে parallel job ভেঙে যেতে পারে—তাই temporary copy ব্যবহার করা ভালো।

## ৮.৭ Publisher key ও package verification

`nilpkg`-এ `.nilax` signature/hash verification আছে। Publisher public key trusted store-এ independently install করতে হবে; package-এর ভিতরের self-signed key-কে trust anchor ধরে নেওয়া যাবে না। Trusted key rotation, key revocation, key expiry (যদি ব্যবহার করা হয়), package version rollback এবং installed binary modification সব test করতে হবে।

Package install flow:

1. remote/download input size cap;
2. archive header/path validation;
3. traversal/symlink rejection;
4. manifest parse/field validation;
5. payload size/hash validation;
6. signature verification against trusted publisher key;
7. permission request parse ও policy;
8. staged install dir create;
9. atomic install rename;
10. install journal recoverable;
11. installed package signature/hash re-verify;
12. first launch permission prompt;
13. uninstall data retention choice ও cleanup policy।

Corrupt/truncated package, path traversal ID, duplicate path, oversized archive, wrong key, invalid signature, signature of old manifest/new payload mismatch, unknown permissions, version downgrade ও interrupted install সব negative test হবে।

## ৮.৮ Verified boot, update signing ও rollback

বর্তমান `mkvbmeta.py` custom descriptor format ব্যবহার করে। এটি prototype integrity metadata, কিন্তু standard Android AVB-এর সমতুল্য নয়। Native phone-এ boot chain enforce হবে কি না তা সংশ্লিষ্ট bootloader-এর capability-র ওপর নির্ভর করে। S25-এ hosted APK ব্যবহার করলে এই native Onuron verified boot chain প্রযোজ্য নয়; host Android-এর boot security আলাদা।

প্রোটোটাইপ পর্যায়:

- immutable release manifest;
- kernel/initramfs/rootfs hash;
- Ed25519 signature;
- offline release signing key;
- public key pinning;
- update artifact download verification;
- atomic staging/rollback test;
- tampered kernel/system artifact reject।

Native device stage:

- bootloader trust root;
- boot image signature verification;
- rootfs/system integrity (dm-verity বা equivalent design);
- rollback index storage;
- signed A/B metadata;
- recovery path trust;
- boot-success watchdog ও failed-slot fallback;
- key rotation/revocation design;
- verified boot state UI।

A/B update শুধু `system_a.img`/`system_b.img` file তৈরি করা নয়। Boot-control metadata-তে active slot, tries remaining, successful boot flag এবং rollback index থাকতে হবে। Failed update-এর পরে known-good slot-এর বাস্তব reboot test করতে হবে। Filesystem marker-কে bootloader-enforced measured/verified state হিসেবে উপস্থাপন করবে না।

## ৮.৯ Security acceptance criteria

- [ ] sandbox setup error production mode-এ unconfined execution ঘটায় না;
- [ ] per-app UID atomic registry collision ও crash-এর বিরুদ্ধে পরীক্ষা;
- [ ] network-denied app-এর real network access blocked;
- [ ] permission grant/revoke actual broker operation-এ enforce;
- [ ] seccomp x86_64 ও ARM64 real kernel-এ negative/positive test;
- [ ] SELinux compile/load/enforce/label runtime evidence;
- [ ] self-signed untrusted package install reject;
- [ ] tampered `.nilax` payload reject;
- [ ] wrong kernel/image signature build/boot chain-এ reject;
- [ ] release signing private key repository/CI logs-এ যায় না;
- [ ] OTA failure rollback test পাস;
- [ ] security docs বাস্তব implementation-কে অতিরঞ্জিত করে না।

---

# অংশ ৯ — Milestone 5: NilLang → NilUI → actual app runtime

## ৯.১ একটি vertical slice আগে, ভাষা বড় করা পরে

NilLang-কে পূর্ণাঙ্গ করতে অনেক language feature যোগ করা যাবে; কিন্তু বাস্তব OS-এ app চালানোর জন্য আগে একটি end-to-end path প্রমাণ করা জরুরি। `runtime/nillang/tests/native_app_slice.rs` source compile, bytecode serialization, `.nilax` package/signature, install verification এবং NilVM reload পর্যন্ত যায়। এরপর `nilrt::lifecycle::LaunchSpec` বানানো হলেও test বাস্তব `nilrt-launch` process invoke করে sandbox enforce করছে না। `NilVM::render_scene()` একটি string tree বানায়; তা compositor-এ draw হচ্ছে বা UI action চালাচ্ছে এমন নয়।

প্রথম accepted app হবে `HelloNative`: কয়েকটি Text element, Column/Row container, Button, `@State count`, button click-এ count বৃদ্ধি এবং close/reopen lifecycle। API limited হলেও end-to-end হতে হবে।

## ৯.২ Compiler ও bytecode format

Compiler stage-এ `Parser`, AST validation, type checking, name resolution, import resolution, bytecode serialization এবং source diagnostics আলাদা রাখো। বর্তমানে JSON serialized `CompiledPackage`-এ magic/version header থাকলেও এটি machine-native executable নয়। Documentation-এ এটিকে portable NilLang bytecode হিসেবে বর্ণনা করা উচিত। `.nib`/`.nilax` extension এবং container format এক নয়; naming/format specification লিখিত হবে।

Bytecode format-এ অন্তত:

- magic bytes;
- format version;
- compiler version;
- target-independent bytecode version;
- package ID ও entry point;
- string table/resource refs;
- state schema;
- widget tree/instructions;
- declared capabilities;
- size bounds;
- integrity hash/signature metadata (package envelope-এ);
- unsupported opcode handling;
- migration policy।

Deserializer untrusted bytes পড়ে; তাই maximum package size, nesting depth, string length, node count, duplicate keys, invalid Unicode/JSON এবং unknown version reject করতে হবে। JSON-based prototype থাকলেও malicious nested content memory/CPU exhaustion ঘটাতে পারে; parser limit চাই। Later compact binary bytecode নিলে versioning ও backward compatibility test আগে থেকেই থাকবে।

## ৯.৩ NilVM execution semantics

বর্তমান VM state variable initial value map করে, root UI tree string-এ render করে। পরবর্তী ধাপে UI properties-কে runtime value references, event handlers, state updates, lifecycle events এবং async host operations-এর সঙ্গে bind করতে হবে। শুধু string attribute resolve করার বদলে typed expression/value system দরকার। User code operation deterministic এবং bounded হতে হবে; runaway loop/recursive tree render resource exhaustion ঘটাতে পারে।

প্রথম runtime capability সীমিত রাখো:

- static Text/Row/Column/Button;
- layout, color, spacing এবং accessibility label;
- `@State` primitive values;
- `onClick` event;
- synchronous arithmetic/string operations;
- navigation between minimal screens;
- app lifecycle init/pause/resume/destroy;
- error boundary ও safe fallback screen।

প্রথম version-এ arbitrary native code plugin loading, unrestricted filesystem, direct socket, reflection কিংবা dynamic machine-code execution দরকার নেই। Capability model স্থির হওয়ার আগে ভাষাকে privileged system programming language হিসেবে ব্যবহার করার চেষ্টা করো না।

## ৯.৪ NilUI compositor bridge

`NilVM` থেকে scene representation actual NilUI runtime-এ পৌঁছাবে। একটি `SceneAdapter` তৈরি করা যেতে পারে যা VM widget graph-কে NilUI element tree-তে convert করে। Adapter-এর test থাকবে: widget count, type mapping, property mapping, state mapping, invalid widget fallback এবং accessibility metadata। NilUI compositor target framebuffer-এ rasterize করবে।

Frame pipeline-এর acceptance:

1. installed bytecode থেকে VM load;
2. scene tree build;
3. layout constraints resolution;
4. paint command generation;
5. compositor frame present;
6. display backend success response;
7. touch event hit-test;
8. button callback dispatch;
9. VM state mutate;
10. next frame state visible;
11. app pause/resume-এর পরে state policy apply;
12. app exit করলে process/resources release।

Headless test-এ rendering screenshot image compare বা deterministic scene snapshot compare ব্যবহার করা যায়। বাস্তব GPU present test-এর সঙ্গে screenshot/compositor evidence থাকতে হবে। A screenshot from the desktop simulator proves only simulator UI, not native phone hardware support.

## ৯.৫ nilrt-launch, installed app directory ও lifecycle

Package install-এর পরে app path কী হবে তা এক canonical spec-এ স্থির করো। `nilpkg` install root, `nilrt-launch` app root, manifest-এর `exec`, payload path এবং `LaunchSpec.rootfs` যেন একই layout বোঝায়। বর্তমানে `nilrt-launch` payload binary হিসেবে `exec`/`bin/<app_id>`/`.nib` খোঁজে; NilLang bytecode executable host OS binary নয়। Launcher-কে বুঝতে হবে `nil` bytecode কোন runtime process-এ load হবে এবং interpreter path কী।

দুটি সম্ভাব্য design:

- `nilrt-launch <app_id>` manifest থেকে format detect করে `nilvm-runner`/runtime worker শুরু করে;
- অথবা একটি generic runtime launcher সব NilLang package load করে, আর `nilrt-launch` sandbox/lifecycle policy enforce করে।

দুটোর মধ্যে একটি বেছে নিয়ে protocol লিখতে হবে। `nilrt-launch`-এর নামে host native executable চালানো এবং `.nib` file-কে kernel `execve`-এ পাঠানো যাবে না। Interpreter app payload untrusted bytes হিসেবে load করবে; filesystem path, `exec`, arguments এবং runtime ABI trust boundary-এর অংশ।

## ৯.৬ Alap-এর ভূমিকা

README-তে “NilLang + Alap” ecosystem বলা হলেও repository tree-তে Alap-এর পূর্ণ implementation path স্পষ্ট নয়। এই gap-কে architectural debt হিসেবে ধরো। Alap যদি cross-platform UI/framework layer হয়, তবে তার দায়িত্ব, version, dependency graph, app API এবং NilUI-এর সঙ্গে সম্পর্ক নির্দিষ্ট করতে হবে। `Alap` নাম উল্লেখ করা থাকলেই framework সম্পূর্ণ আছে দাবি করা যাবে না।

প্রস্তাবিত কাজ:

- ADR লিখে Alap = app lifecycle? state management? navigation? platform services? এর সীমানা নির্ধারণ;
- প্রথমে minimal API: `App`, `View`, `State`, `Event`, `Navigation`, `Permissions`;
- NilLang standard UI module ও Alap interface version synchronize;
- Android-host ও native Linux backend conformance test;
- unsupported platform feature capability-query দিয়ে report;
- documentation ও hello app একই syntax/API ব্যবহার করে;
- architecture-এ unnecessary wrapper layer এড়ানো।

## ৯.৭ Native app test suite

Unit test ছাড়াও একটি process-level e2e test যোগ করো। Linux test runner-এ `nilpkg install` দিয়ে sample package install করবে, test publisher key trust store-এ থাকবে, `nilrt-launch` invoke হবে, sandbox setup পরীক্ষা হবে, VM UI tree তৈরি করবে এবং scripted button event-এর পরে state পরিবর্তন assertion হবে। CI container-এর permissions sandbox operation permit না করলে test-কে security pass হিসেবে গণ্য করা যাবে না; unprivileged developer fallback test আলাদা tag-এ চলবে।

Native app acceptance:

- source syntax validation;
- type checking/diagnostic;
- deterministic bytecode format;
- signed package creation;
- trusted install;
- signature/hash re-verification;
- denied permissions enforced;
- launcher lifecycle start/stop;
- UI tree render;
- input/action round-trip;
- app data isolation;
- crash recovery;
- update/rollback;
- malformed bytecode rejection।

---

# অংশ ১০ — Milestone 6: প্রথম native reference phone নির্বাচন

## ১০.১ একটি ফোন কেন, অনেক ফোন নয়

প্রথম native port-এর জন্য একটি নির্দিষ্ট model/codename বেছে নিতে হবে। “ARM64 phone” target যথেষ্ট নির্দিষ্ট নয়। একটি SoC family-র মধ্যেও device tree, panel, touch controller, PMIC, storage layout, bootloader version, modem firmware, camera sensor এবং vendor partition আলাদা হতে পারে। একই ফোনের বিভিন্ন regional variant-ও আলাদা হতে পারে। একসঙ্গে S25, OnePlus 6T, OnePlus 6 ও PinePhone সমর্থন করার চেষ্টা করলে প্রতিটি bring-up অসম্পূর্ণ থাকবে।

তোমার ক্ষেত্রে তিনটি সম্ভাবনা আছে:

- **Samsung Galaxy S25:** এখনই আছে, hosted Android runtime পরীক্ষা করার সবচেয়ে সস্তা উপায়। Native Linux boot-এর জন্য এটি reference phone নয়; bootloader/firmware/security restrictions আলাদা করে যাচাই না করে flash route ধরে নেওয়া যাবে না।
- **OnePlus 6T (`fajita`):** তুলনামূলকভাবে mature community mainline Linux work ও postmarketOS-এ testing category-তে রয়েছে। postmarketOS-এর ২০২৬ সালের hardware-testing update-এ OnePlus 6T-কে real-device test runner হিসেবে ব্যবহারের কথা বলা হয়েছে। তাই Linux hardware bring-up workflow শেখার একটি যুক্তিযুক্ত প্রার্থী। এটি OnuronOS compatibility নিশ্চিত করে না। [postmarketOS hardware testing update](https://postmarketos.org/blog/2026/01/21/hw-ci-mvp/), [postmarketOS v26.06 release](https://postmarketos.org/blog/2026/06/21/v26.06-release/)।
- **PinePhone:** Linux-first hardware ecosystem-এর সুবিধা আছে, কিন্তু নতুন করে কেনা/পাওয়া, performance ও স্থানীয় availability বিবেচনা করতে হবে। Reference target হিসেবে community ecosystem, current image, display/network/audio/camera status এবং hardware test access পরীক্ষা করে সিদ্ধান্ত নিতে হবে।

দ্বিতীয় হাতের OnePlus 6T-তে ২০২৬ সালে battery wear, storage health, broken display/USB, carrier/region variant এবং bootloader unlock state বিশেষভাবে যাচাই করতে হবে। মডেলটি community-তে আছে বলে প্রতিটি ব্যবহৃত ইউনিট bootloader unlocked থাকবে না। কেনার আগে বিক্রেতার কাছ থেকে bootloader screen/fastboot identity এবং model number যাচাই করার ব্যবস্থা নিতে হবে।

## ১০.২ Device selection scorecard

কোন ফোন কিনবে তা অনুভূতি বা “স্পেসিফিকেশন বেশি” দিয়ে নয়, scorecard দিয়ে ঠিক করো। প্রতিটি বিষয়ে ০–৫ score দাও এবং evidence link রাখো:

| মানদণ্ড | কী যাচাই করবে | ওজনের প্রস্তাব |
|---|---|---:|
| Unlockability | bootloader unlock বাস্তবে সম্ভব, region/operator restriction নেই | ২০% |
| Existing Linux port | mainline/kernel/firmware/device tree source ও বর্তমান install guide | ২০% |
| Recovery | stock firmware, fastboot/recovery route, known-good image ফেরানো যায় | ১৫% |
| Display/touch | Linux port-এ বাস্তব touch ও readable display | ১০% |
| Network/audio | Wi-Fi, BT, mic/speaker status documented | ১০% |
| Storage/charging | persistent data, USB, battery ও charging | ১০% |
| Community activity | recent commits/issues/releases/test evidence | ১০% |
| Local cost/condition | দাম, battery health, spare parts ও return policy | ৫% |

Scorecard target-phone decision ADR-এ সংযুক্ত হবে। কোনও feature “works” লেখা থাকলে তার source page-এর update date ও test evidence নথিবদ্ধ করো; পুরোনো wiki snapshot-এর status current release-এর সঙ্গে মিলিয়ে দেখো।

## ১০.৩ Device Support Policy

একটি `docs/devices.md` বা `devices/README.md`-এ device support-এর চারটি স্তর সংজ্ঞায়িত করো:

- `candidate`: device নির্বাচন আলোচনায় আছে, test শুরু হয়নি;
- `bring-up`: developer kernel/initramfs পরীক্ষা করছে, supported নয়;
- `experimental`: কিছু capability বাস্তবে validated, known issues আছে;
- `supported`: documented release image, recovery, repeatable install/boot, key hardware tests ও update strategy সফল;
- `deprecated`: পুরোনো profile আর release gate পাস করে না, কিন্তু archival test evidence রয়েছে।

“Supported” শব্দের জন্য ন্যূনতম gate: correct model variant identification, installable image checksum, boot repeatability, recovery success, persistent storage, display/touch, battery/charging, Wi-Fi or a documented networking limitation, audio route, suspend/resume status, update/rollback status এবং known-issues list। Call/SMS/camera কাজ না করলে তা স্পষ্ট করে বলো; all-hardware claim করো না।

## ১০.৪ Device profile structure

একটি device profile-এ build settings এবং evidence থাকবে, source code copy নয়:

```text
devices/
└── oneplus-fajita/
    ├── device.toml
    ├── README.md
    ├── kernel/
    │   ├── defconfig.fragment
    │   └── source-lock.toml
    ├── boot/
    │   ├── format.toml
    │   └── partition-map.toml
    ├── firmware/
    │   └── manifest.toml
    ├── hal/
    │   ├── capabilities.toml
    │   └── quirks.toml
    ├── install/
    │   ├── preflight.md
    │   ├── install.md
    │   └── recovery.md
    └── tests/
        ├── smoke.toml
        └── matrix.md
```

এটি ভবিষ্যৎ proposed structure; repository-তে এই directory এখন আছে বলে ধরে নিও না। `device.toml`-এ device codename, compatible model strings, CPU architecture, SoC, RAM range, storage type, boot image format/version, kernel source revision, device-tree source, firmware provenance, supported bootloader state এবং required tools থাকবে। Secrets বা proprietary firmware blob-এর private key/redistribution permission ছাড়া blob commit করবে না। Firmware licensing, source URL, hash ও redistribution condition নথিবদ্ধ করতে হবে।

## ১০.৫ Kernel strategy: fork না upstream port

Native device-এর kernel strategy তিনটির একটিতে স্থির করতে হবে:

1. **Mainline-oriented kernel:** upstream Linux driver ও community patches ব্যবহার; দীর্ঘমেয়াদি maintainability লক্ষ্য।
2. **Vendor/Android kernel compatibility:** Android vendor kernel/module ব্যবহারের কিছু অংশ; দ্রুত boot সম্ভাব্য, কিন্তু vendor ABI/module/firmware coupling ও maintenance debt বেশি।
3. **Halium/libhybris-style userspace bridge:** Android vendor userspace/library reuse; boot path ও GPU/camera/audio support সহজতর হতে পারে, কিন্তু dependency ও security model জটিল।

প্রথম phone bring-up-এ community Linux port যেখানে ইতিমধ্যে কাজ করে, সেই kernel path-কে baseline হিসেবে ব্যবহার করা সবচেয়ে কম-risk। নতুন করে সব driver লেখা নয়; আগে existing port-এর reproducible setup-কে Onuron rootfs boot-এ যুক্ত করো। পরে প্রয়োজনীয় patch upstream বা documented patchset-এ রাখো। `kernel/`-এ বর্তমানে কিছু defconfig fragments আছে, সম্পূর্ণ pinned kernel source/build pipeline নয়—তাই একটি config fragment থাকা kernel support-এর প্রমাণ নয়।

## ১০.৬ Kernel source pinning

Device kernel build-এ source repository, branch/tag এবং exact commit pin করো। `main`/`master`-এর head থেকে build করলে একই target পরের সপ্তাহে অন্য kernel তৈরি করতে পারে। `source-lock.toml`-এ:

- repository URL;
- exact revision;
- patch list/hash;
- toolchain version;
- defconfig/fragment hashes;
- DTB/DTBO source;
- compiler flags;
- firmware prerequisites;
- source license/provenance;
- reproducibility status;

থাকবে। Kernel source/patches fetch করে cache করলে checksum ও commit verify হবে। Device tree-কে arbitrary blob ধরে blindly use না করে exact phone variant-এর compatible string ও kernel log যাচাই করবে।

## ১০.৭ Boot chain ও partition discovery

Native boot path-এর আগে Android boot image spec, bootloader slot scheme, AVB/vbmeta behavior, `boot_a`/`boot_b`, `vendor_boot`, `dtbo`, `super` dynamic partitions, `metadata`, `userdata` এবং recovery path সম্পর্কে target-specific evidence চাই। এই partition names সব ফোনে এক নয়; OnePlus 6T-এর জন্য একটি model guide থেকে নেওয়া instruction S25-এ প্রয়োগ করা যাবে না।

Preflight tool শুধু read-only query করবে:

- `fastboot devices` এবং exact serial confirm;
- `fastboot getvar product`/`current-slot`/bootloader state, supported হলে;
- device codename/model string compare;
- output directory ও image manifest target match;
- partition size/slot metadata read;
- signed artifact hash validation;
- recovery image path ও known-good restore package present;
- AC power/battery threshold check;
- user-data backup confirm;
- destructive command preview।

কোনও command target ID বা serial ছাড়া generic `fastboot flash` চালাবে না। Unknown partition layout হলে script fail করবে। Userdata wipe default নয়; wipe কখন ও কেন লাগবে তা installation guide স্পষ্ট করে এবং user confirmation নেয়।

---

# অংশ ১১ — Milestone 7: Native phone bring-up-এর ধাপ

## ১১.১ প্রথম ধাপ — console ও boot-to-init

Native target-এ প্রথম লক্ষ্য GUI নয়। প্রথম লক্ষ্য হলো trusted build থেকে kernel boot, serial/early console evidence, initramfs extraction, `/init` execution এবং `nilinit` PID 1। যদি screen black থাকে কিন্তু serial console-এ kernel ও init logs আসে, তা গুরুত্বপূর্ণ debug evidence। যদি logs-ও না আসে, আগে boot image/entry point, kernel header, DTB, console driver এবং bootloader handoff যাচাই করো।

Bring-up image-এ logs persistent বা USB/serial console দিয়ে সংগ্রহ করার ব্যবস্থা রাখো। `/dev/kmsg`, pstore/ramoops থাকলে kernel crash evidence, watchdog reset reason এবং previous boot log preserve করো। `nilinit`-এর recovery logic boot failure counter ব্যবহার করে; counter write/read path সত্যিই persistent `/data` বা `/metadata`-এ আছে কি না এবং recovery trigger ভুলভাবে normal install-কে block করছে কি না পরীক্ষা করতে হবে।

প্রথম native boot image minimal হবে: essential userspace, `nilinit`, diagnostics shell, storage mount, required core services। Camera, telephony, browser, Android compatibility container ও distributed bus-এর সব daemon একসঙ্গে চালু করার দরকার নেই। Missing optional daemon-এ boot fail হবে না; mandatory core service unavailable হলে normal boot success হবে না।

## ১১.২ দ্বিতীয় ধাপ — root filesystem ও `/data`

Native device storage-এ partition layout target profile থেকে আসবে। Initramfs-এ block device enumeration, filesystem UUID/label, encryption state এবং mount options যাচাই করা হবে। `/data` writeable করার আগে expected filesystem identity যাচাই করবে; wrong partition ভুল করে format করা চলবে না। Data encryption enabled বলার জন্য শুধু settings text বা `fscrypt` binary presence যথেষ্ট নয়। Actual key lifecycle, unlock flow, key storage, directory encryption policy, recovery behavior এবং boot-time availability test করতে হবে।

Data test cases:

- first boot creates config/data dirs;
- reboot survives user settings and app data;
- filesystem read-only হলে UI/service warning;
- missing/corrupt `/data` enters recovery/degraded mode;
- encryption key wrong/missing হলে data inaccessible, not silently blanked;
- filesystem full হলে write reports error;
- power loss after file write/rename recovers consistent state;
- update rollback preserves user data version/schema;
- factory reset deletes intended app/user data only after explicit confirmation;
- recovery tools never format an unrecognized partition.

## ১১.৩ তৃতীয় ধাপ — display ও compositor

Phone display bring-up-এর কাজ হলো panel driver, DRM/KMS connector, mode setting, framebuffer format, page flip/vsync, rotation, brightness এবং suspend/resume interaction। `nilui-gpu`-এ compositor code থাকলেও target device-এর DRM node ও modeset test ছাড়া সেটিকে working phone display বলা যাবে না। `DisplayHal::present_frame()` real buffer backend-এ present করতে হবে; no-op return success acceptable নয়।

ক্রম:

1. kernel DRM device enumerate;
2. `modetest`/equivalent diagnostic tool দিয়ে connector/mode list;
3. stable mode নির্বাচন;
4. simple test pattern present;
5. page flip/vsync loop;
6. touch-to-frame coordinate calibration;
7. brightness sysfs/backlight controller;
8. screen off/on and resume;
9. orientation/rotation behavior;
10. compositor UI launch;
11. frame time/drop telemetry;
12. long run burn-in।

শুরুতে hardware-accelerated GPU rendering বাধ্যতামূলক নয়; software/dumb-buffer renderer দিয়ে pixels screen-এ দেখা সহজ হতে পারে। GPU acceleration পরে আলাদা milestone। Display resolution 1080×2340 হার্ড-কোড করা যাবে না; panel mode query ও logical scale policy ব্যবহার করতে হবে। Dynamic refresh rate, notch/status area, rounded corners এবং usable insets UI layer-এ host/device profile থেকে expose হবে।

## ১১.৪ চতুর্থ ধাপ — touch, buttons ও input

Screen-এ image আসার পর touch controller ও physical keys bring up করো। Kernel evdev nodes, device permissions, udev rules, axis calibration, multitouch tracking, pointer orientation এবং hotplug test লাগবে। `inputd` service-কে event node discovery ও reconnect করতে হবে; node path `/dev/input/event0` ধরে রাখা যাবে না। Device order boot-এর মধ্যে বদলাতে পারে।

Test:

- single finger tap/drag;
- multitouch pinch/gesture;
- power/volume buttons;
- portrait/landscape transform;
- touch down then screen suspend;
- device removed/reinitialized;
- compositor restart;
- invalid/out-of-range coordinates;
- rapid event bursts and queue overflow;
- touch event to UI Button callback latency।

Coordinate mapping-এ panel coordinates, evdev raw ranges, display rotation এবং logical UI dimensions-এর transform এক জায়গায় রাখো। প্রতিটি application-এ আলাদা করে hard-coded scaling করলে একই input বিভিন্ন app-এ আলাদা কাজ করবে।

## ১১.৫ পঞ্চম ধাপ — power, battery, charging ও thermal

Mobile Linux-এর battery/performance stability desktop Linux-এর চেয়ে আলাদা কাজ। PMIC drivers, battery charger, fuel gauge, thermal zones, CPU frequency, regulator, wakeup source ও suspend state target kernel-এ available কি না যাচাই করতে হবে। `powerd`-এর UI value sysfs থেকে আসবে; missing node-এ `85% battery` বা `29.5°C` fake default থাকলে সেটিকে production backend-এ ব্যবহার করা যাবে না।

ফিচারগুলিকে আলাদা করো:

- capacity/status;
- USB/AC charger type;
- charge current/voltage (যদি পাওয়া যায়);
- battery temperature;
- SoC thermal zones;
- CPU frequency/performance mode;
- screen timeout;
- wake lock acquire/release;
- suspend/resume;
- low-battery shutdown behavior;
- thermal throttling telemetry।

Test-এ চার্জার যুক্ত/খোলা, low battery, high thermal load, screen off, Wi-Fi activity, incoming call, idle 8-hour drain এবং resume test থাকবে। Battery percentage oscillation বা stale reading হলে UI last-known value-এর timestamp/quality report করবে। Battery temperature sysfs ইউনিট milli-degrees Celsius হতে পারে; unit conversion ভুল হলে values বিভ্রান্তিকর হবে।

## ১১.৬ ষষ্ঠ ধাপ — storage, USB ও diagnostics

Phone-এর storage controller support, eMMC/UFS driver, partition enumeration ও data mount যাচাই করা হবে। USB-C role, OTG, ADB-like recovery access এবং external storage feature আলাদা; কোনও একটি কাজ করলেই সব কাজ করবে না। প্রথম release-এ data partition read/write ও recovery path প্রাধান্য পাবে।

Diagnostics shell-এ `ps`, `services`, `net`, `storage`, `hal`, `security`, `boot` command-এর output বাস্তব data source থেকে আসবে। বর্তমানে কিছু diagnostic output fabricated হতে পারে; native target-এ fake rows দেখা গেলে user ভুলভাবে system health বুঝবে। প্রতিটি command-এর output-এ source এবং timestamp থাকলে ভালো। Missing daemon-এ “Not running” এবং unsupported query-তে “Unavailable” দেখাবে, invented service count নয়।

## ১১.৭ সপ্তম ধাপ — Wi-Fi ও Bluetooth

Native Linux Wi-Fi-এর জন্য firmware, kernel driver, `cfg80211`, regulatory database, supplicant/connection manager, DHCP, DNS, route ও DNS resolver design দরকার। Bluetooth-এ kernel HCI, firmware, BlueZ stack, pairing, profiles, audio route এবং permission model আছে। `netd` daemon বা `btd` binary আছে মানেই Wi-Fi scan/connect এবং Bluetooth pairing কাজ করছে না।

প্রথম network acceptance:

- interface discovered;
- link state বাস্তব sysfs/netlink থেকে আসে;
- Wi-Fi scan real access point list দেয়;
- connect attempt success/failure truthful;
- DHCP address obtained;
- DNS resolution;
- external connectivity check;
- disconnect/reconnect;
- saved credential protection;
- airplane/flight mode policy;
- no network interface scenario;
- denied app network blocked by sandbox/network policy।

Test SSID/BSSID/password log-এ প্রকাশ করবে না। CI-তে hardwareless QEMU network test virtual connectivity validate করতে পারে, real radio support নয়। Physical Wi-Fi test-এর জন্য device logs ও actual connection evidence আবশ্যক।

## ১১.৮ অষ্টম ধাপ — audio

Native phone audio-তে kernel codec/DAI, SoC audio routing, DSP firmware, ALSA controls এবং userspace sound server/`audiod` integration লাগবে। “Audio HAL returns Ok” বা zero-filled recording কাজের প্রমাণ নয়। প্রথমে simple beep playback, পরে mic capture, volume, headphone, speaker/earpiece routing, mute, Bluetooth route এবং incoming notification priority পরীক্ষা করো।

Audio test fixture known signal ব্যবহার করবে; capture RMS threshold অন্ধভাবে set করা যাবে না কারণ microphone gain/noise floor ভিন্ন হতে পারে। Test metadata-তে sample rate, channels, PCM format, mixer settings ও test environment লিখতে হবে। দীর্ঘ recording-এ buffer overrun, power use, thermal and privacy indicator পরীক্ষা করো। App recording permission revoke হলে capture promptly stop হতে হবে।

## ১১.৯ নবম ধাপ — camera

Camera system pipeline-এ sensor driver, I2C controls, CSI/ISP, firmware, media controller graph, V4L2, image format, focus/exposure এবং JPEG/codec path জড়িত থাকতে পারে। OnePlus 6T বা PinePhone-এ camera support আংশিক হলে তা initial native prototype-এর blocker নাও হতে পারে, কিন্তু supported feature list-এ স্পষ্ট লিখতে হবে। Samsung S25-তে Android-host camera API ব্যবহার করা আর native Linux camera bring-up একই বিষয় নয়।

প্রথমে inspect করো:

- `/dev/video*` ও media controller nodes;
- driver bind status;
- kernel logs;
- camera sensor/ISP topology;
- expected firmware files and licenses;
- available community patches;
- actual V4L2 capture capability;
- image decode test;
- torch controls;
- suspend/resume and camera release।

Test pattern JPEG fallback production backend-এ থাকবে না। যদি বাস্তব camera available না হয়, `Camera app`-এ clear unsupported message দেখাও অথবা feature hide করো। Capture action success শুধুমাত্র valid frame source থেকে হওয়া চাই।

## ১১.১০ দশম ধাপ — modem, SMS, calls, GPS

Telephony সবচেয়ে শেষে আনো, কারণ modem firmware, Qualcomm QMI/MBIM/AT interface, SIM detection, RF calibration, carrier configuration, audio route, IMS/VoLTE এবং emergency-call restrictions জটিল। `telephonyd` process চালু থাকা বা `dial()` synthetic ID বানানো real mobile call নয়। প্রথম native port-এর success gate কল/VoLTE না-ও হতে পারে; কিন্তু release status-এ unsupported বলা আবশ্যক।

Bring-up-এ:

1. modem hardware/interface enumerate;
2. firmware readiness;
3. SIM inserted/ready;
4. signal/regulatory state;
5. data-only test through supported modem stack;
6. SMS send/receive tests on a test SIM, consentসহ;
7. call dial/ring/answer/hangup state machine;
8. audio route to earpiece/mic;
9. VoLTE/IMS separately documented;
10. GPS location/time/permission tests;
11. failure/reconnect/reboot after modem crash;
12. user-visible network registration/error status।

Emergency call-কে development test case হিসেবে বাস্তব emergency number-এ কখনও test করা যাবে না। Carrier provisioning ও SIM cost বিবেচনা করতে হবে। Call test-এ consented test number/controlled network, time limit এবং privacy sanitization ব্যবহার করো।

## ১১.১১ Physical bring-up acceptance matrix

| Area | Minimum evidence | Release status if not met |
|---|---|---|
| Boot | clean boot logs; correct PID 1; 10 consecutive boots | unsupported/experimental |
| Storage | persistence, fs error handling, recovery | no user data reliance |
| Display | test pattern + compositor + sleep/wake | headless prototype only |
| Touch | event mapping + gestures + resume | touch not supported |
| Battery/charge | real readings, charger transitions | values unavailable or partial |
| Wi-Fi/network | scan/connect/DHCP/DNS/route | network partial/unsupported |
| Bluetooth | pairing and documented profiles | partial/unsupported |
| Audio | playback and recording path, route test | audio partial/unsupported |
| Camera | true captured image and lifecycle | camera unsupported/partial |
| Telephony | actual SMS/call state and modem evidence | no calls/SMS claim |
| Suspend/resume | repeated cycles, data integrity | sleep unsupported |
| Update/recovery | failed update rollback, stock restore | not releaseable as phone OS |

---

# অংশ ১২ — Milestone 8: boot, init, service lifecycle ও recovery

## ১২.১ Boot chain-এর canonical description

Boot documentation-এ প্রথমে তিনটি আলাদা chain আলাদা diagram-এ দেখাতে হবে। x86_64 QEMU-তে QEMU সরাসরি kernel/initramfs load করে; Android-host mode-এ Android boot chain চালু থাকে এবং Onuron APK launch হয়; native phone mode-এ phone bootloader/vendor boot structure থেকে Onuron-compatible kernel ও ramdisk load করতে হবে। এই chain-গুলি পরস্পরের বিকল্প নয়। `mkbootimg.py` দিয়ে Android boot image বানানো যায় বলে সব Android device সেই image গ্রহণ করবে, এমন নয়—header version, vendor_boot, DTB/DTBO, signature, AVB এবং partition naming আলাদা।

Native phone documentation-এ প্রতিটি stage-এ trust boundary থাকবে:

1. ROM/bootloader hardware-specific state নেয়;
2. bootloader image/slot নির্বাচন করে;
3. trusted signature/integrity verify হয়;
4. kernel ও DTB memory-তে load হয়;
5. kernel command line ও initramfs load হয়;
6. kernel initramfs extract করে `/init` exec করে;
7. `nilinit` early mounts, storage, recovery decision করে;
8. critical services start এবং ready হয়;
9. OOBE/lock screen/shell শুরু হয়;
10. boot success persistent metadata-তে লেখা হয়।

প্রকৃত target-এ কোন stage OnuronOS নিয়ন্ত্রণ করে এবং কোন stage vendor bootloader-এ থাকে, তা নির্দিষ্ট করে লিখো। Unsupported bootloader verification-কে Onuron verified boot claim হিসেবে দেখিও না।

## ১২.২ `nilinit`-কে predictable PID 1 বানানো

PID 1-এর failure behavior সাধারণ user process-এর মতো নয়। Unhandled panic বা exit হলে system freeze/reboot হতে পারে। `nilinit`-এর job হলো early mount, `/data`, recovery decision, service startup, supervision, shutdown coordination এবং logging। Desktop-style service manager-এর সমস্ত feature একসঙ্গে implement করার দরকার নেই; প্রথমে core semantics সঠিক করো।

Core startup sequence:

- early console establish;
- `/proc`, `/sys`, `/dev`, `/run`, `/tmp` mount result validate;
- boot reason/failed-boot count read;
- persistent storage discover and mount;
- encryption/key prerequisites (যদি configured);
- SELinux load result;
- environment/config generation;
- core service dependency order;
- per-service readiness handshake;
- shell/first-run system start;
- boot success marker/slot success update;
- supervision event loop;
- shutdown/reboot request process।

Mount failures সব একসঙ্গে ignore করা যাবে না। `/proc` বা `/dev` mount ব্যর্থ হলে service manager স্বাভাবিক চলছে বলা যাবে না। `/data` fallback policy আলাদা: temporary `tmpfs` দিয়ে diagnostic shell চালানো যেতে পারে, কিন্তু boot mode degraded হবে এবং settings UI persistent storage unavailable দেখাবে। SELinux failure enforcing security profile-এ boot block করতে পারে; developer recovery profile আলাদা রাখা যেতে পারে।

## ১২.৩ Service metadata schema

`etc/nilos/services.toml`-এ এখন name/exec/restart এবং কিছু socket activation field আছে। Service definition versioned schema-এ নিতে হবে। উদাহরণ:

```toml
schema_version = 1

[[services]]
name = "nild"
exec = "/usr/bin/nild"
critical = true
restart = "on-failure"
ready = "unix:/run/onuron/nild.sock"
ready_timeout_ms = 3000
dependencies = ["logd"]
user = "root"
capabilities = []
```

এটি কেবল schema-এর ধারণা; actual field names ও security model ADR-এ final করবে। `critical`/`ready` flags-এর semantics testable হবে। System service-এর user UID/GID এবং Linux capabilities explicit হবে; privileged service-কে root-এ চালানো বাধ্যতামূলক হলে rationale লিখবে। `exec` parsing quote support থাকলেও shell-like expansion দরকার নেই; direct executable + argument array বেশি নিরাপদ। Untrusted `exec` string shell command হিসেবে pass করবে না।

## ১২.৪ Dependency ordering ও readiness

`nilinit` service গুলি sequentially spawn করলেই dependency readiness নিশ্চিত হয় না। Process create করা এবং socket bind করা/IPC protocol accept করা আলাদা state। Service dependency graph cycle detect করবে। `logd`/`nild`/`nilbus`-এর মতো services dependency-র ক্রমে start হবে; optional UI helper ব্যর্থ হলে core OS boot চলতে পারে, কিন্তু shell unavailable status দেখাবে।

Readiness protocol কয়েকভাবে হতে পারে:

- UNIX socket bind + expected handshake message;
- readiness file atomic write (শুধু process identity/permissions সঠিক থাকলে);
- daemon health request/response;
- service supervisor notification file descriptor;
- system bus announcement।

শুধু PID alive বা socket path file exists readiness নয়। Stale socket file থাকতে পারে। Handshake response-এ protocol version, backend identity, capability set ও degraded flags থাকতে পারে। `nilinit` process health periodic check করলেও service internal deadlock বা request failures ধরতে separate health probe প্রয়োজন হতে পারে।

## ১২.৫ Restart policy ও crash loop

Current `Supervisor` exponential backoff এবং 60-second healthy uptime পর attempts reset করে। এটি ভালো ভিত্তি। আরও দরকার:

- `always`, `on-failure`, `never`, `oneshot` semantics স্পষ্ট;
- exit code ও signal record;
- repeated failure threshold;
- crash-loop circuit breaker;
- dependency failure propagation;
- service stderr/stdout logging policy;
- restart-এর আগে socket/PID/state cleanup;
- shutdown-এ graceful signal → timeout → kill;
- service restartের পরে readiness পুনরায় wait;
- user notification/diagnostic event;
- crash dumps-এ secrets redact;
- watchdog/reset reason।

Test-এ test daemon দ্রুত exit করবে, deterministic fake clock/backoff ব্যবহার করা হবে এবং stuck process timeout হবে। Crash loop হলে `nilinit` নিজে starvation-এ পড়বে না। Runtime-এ service restart হচ্ছে বলে boot success marker বারবার লেখা যাবে না। `nilinit`-এর fatal condition-এ recovery/safe mode অথবা controlled reboot policy থাকতে হবে।

## ১২.৬ Socket activation

`SocketActivationManager` listener register/check_pending করতে পারে এবং supervisor-এ socket FD handoff-এর প্রচেষ্টা আছে। বাস্তব socket activation contract-এর জন্য listener bind, accepted client connection, file descriptor handoff এবং daemon receive semantics যাচাই করতে হবে। Service-কে যে FD দেওয়া হয় তা listening FD না accepted client FD—এটি systemd-style activation convention-এর সঙ্গে মেলাতে হবে। `LISTEN_FDS`/`LISTEN_FDNAMES` environment যথাযথভাবে set হলে daemon code তা consume করছে কি না নিশ্চিত করতে হবে।

`check_pending()` শুধু listener readable দেখলে service start হতে পারে, কিন্তু pending connection নিজে handle না করলে request lost হতে পারে। Test client connect করবে, service start হবে, service request accept করে response ফেরাবে, দ্বিতীয় requestও কাজ করবে; inactive service-এর duplicate spawn হবে না। Activation service start fail করলে backlog/timeout policy থাকতে হবে। Socket file permissions, owner/group এবং peer credentials IPC policy-র সঙ্গে সামঞ্জস্য রাখতে হবে।

## ১২.৭ Shutdown, reboot ও recovery

Current shutdown handler supervisor shutdown, `sync()` এবং “halted safely” log লেখে; এটি সত্যিকারের Linux poweroff/reboot syscall ও unmount sequence সম্পূর্ণ implement করে কি না যাচাই করতে হবে। Service terminate order dependency-র উল্টো দিকে হবে। Dirty storage buffers sync হবে; critical persistent state atomic commit হবে; mounts unmount attempt হবে; তারপর `reboot(RB_POWER_OFF/RB_AUTOBOOT)` বা target-supported control path ব্যবহার করা হবে। Syscall fail হলে PID 1 parked থাকা ও diagnostic message acceptable bring-up behavior হতে পারে, কিন্তু production shutdown-এর জন্য যথেষ্ট নয়।

Recovery system-এর objective হলো user data রক্ষা করে bootable state-এ ফিরে আসা। Recovery image-এ package management, log export, partition verify, signed update reinstall এবং known-good slot switch থাকবে। Recovery shell এলে unrestricted root shell accessible security risk; developer builds-এ password/physical confirmation policy, production recovery image-এ controlled operations প্রয়োজন। Boot failure counter persistent storage unavailable থাকলে counter ভুলভাবে reset হয়ে endless bad boot হতে পারে; metadata location ও fallback policy লিখতে হবে।

## ১২.৮ OOBE, lock screen ও initial security

প্রথম boot-এ OOBE user name/PIN বা config লিখতে পারে। PIN-hash design, lock-screen throttle, failed attempts, recovery/reset, device encryption unlock এবং account model আলাদা করে দেখো। বর্তমান prototype-এ salted/stretched SHA-256 উল্লেখ আছে; mobile production credential storage-এ memory-hard password hashing, secure hardware/keystore integration (যদি supported), rate limiting, hardware-backed attestation assumptions এবং lock-screen bypass threat review প্রয়োজন। Security UI-তে “encrypted” বা “SELinux enforcing” লেখা কেবল runtime source query verified হলে দেখাতে হবে।

OOBE-তে app permissions auto-grant নয়। Network setup/terms/diagnostics data collection/backup policies explicit consent পাবে। Offline first boot supported হতে পারে; cloud dependency ছাড়া local account/system setup complete হবে। বাংলা locale, Bengali numerals/date, accessible labels, font fallback এবং on-screen keyboard-এর test matrix OOBE থেকেই অন্তর্ভুক্ত হবে।

---

# অংশ ১৩ — Milestone 9: storage, encryption, package এবং OTA updates

## ১৩.১ Filesystem layout

Prototype-এ `/system`, `/vendor`, `/data`, `/cache`, `/recovery`, `/metadata` তৈরি করার code আছে। শুধু directories create করা actual partition/mount layout নয়। Native phone-এ কোন path পৃথক partition, read-only image, bind mount বা subdirectory—একটি canonical layout spec-এ থাকবে। Rootfs immutable/read-only রাখা, writable `/data`, logs/config, app packages এবং updates-এর staging partition আলাদা করলে rollback ও integrity analysis সহজ হয়।

প্রথম release-এ layout অপ্রয়োজনীয় জটিল করো না। Minimal target layout হতে পারে:

- kernel/initramfs boot artifact;
- immutable system/rootfs artifact;
- read-only or integrity-verified base files;
- persistent `/data`;
- optional metadata/boot-control state;
- recovery path।

`/system`/`/vendor` directories rootfs-এ থাকলেই Android-style system/vendor partition বাস্তবায়িত হয়েছে বলা যায় না। Android boot image/partition model থেকে concept ধার করা আর একই implementation থাকা আলাদা। Documentation-এ actual mounted source/partition mapping দেখাও।

## ১৩.২ Persistence এবং crash consistency

User settings, OOBE marker, contacts, SMS, app state, permissions database, UID registry, boot counter এবং update state—প্রতিটি persistent file-এর owner/mode, schema version, atomicity ও recovery rule দরকার। JSON file-এ direct truncate-write crash হলে corrupted state হতে পারে। Atomic write pattern: temp file same filesystem, write all, fsync temp, rename, fsync parent directory। File lock প্রয়োজন হলে writer concurrency policy থাকবে।

Test:

- kill process before temp rename;
- kill after rename before parent fsync;
- truncated JSON;
- unexpected schema version;
- permission database corrupt;
- disk full;
- read-only remount;
- reboot after each staged step;
- system update while `/data` schema migration pending;
- concurrent app writes;
- clock skew affecting expiry;
- recovery from last-known-good record।

Persistence test harness-এ শুধু test marker থাকা নয়; real builder-produced disk image, real mount code এবং actual reboot sequence ব্যবহার করতে হবে। QEMU smoke-এ `nilos.img` attach করা আর formatted disk write/read যাচাই করা এক কাজ নয়।

## ১৩.৩ Data encryption

`fscrypt enabled` বা encryption-ready wording ব্যবহার করার আগে implementation evidence চাই। fscrypt-এর জন্য kernel support, policy configuration, key management, unlock timing, password/PIN integration, recovery reset, app-specific encryption key এবং data migration policy দরকার। FDE/block-level encryption চাইলে dm-crypt/key management design ভিন্ন। কোনটি ব্যবহার হচ্ছে তা এক বাক্যে স্পষ্ট করে বলো।

কী প্রশ্নের উত্তর নথিতে থাকতে হবে:

- encryption data-at-rest কোন threat আটকায়;
- key কোথা থেকে আসে;
- boot key hardware-backed কি না;
- PIN change করলে কী হয়;
- PIN ভুলে গেলে data recoverable কি না;
- recovery image কি user data read করতে পারে;
- app-per-app key কীভাবে isolate হয়;
- OTA update policy key access পরিবর্তন করে কি না;
- factory reset cryptographic erase কীভাবে করে;
- logs/cache/thumbs encryption scope-এর মধ্যে আছে কি না।

Encryption test-এ শুধু file readable/writable দেখা যথেষ্ট নয়; raw image offline mount করলে plaintext data পাওয়া যায় কি না এবং key unavailable হলে behavior কী, তা পরীক্ষা করো। Key material Git repository, test artifact বা plain environment variable-এ থাকবে না।

## ১৩.৪ nilpkg package database

`nilpkg` install/verify/upgrade/rollback functionality আছে। Installed state-এর source of truth (manifest, payload, trust key id, install path, permissions grants, installed version) canonical database/file layout-এ রাখতে হবে। `verify_installed` rechecks signature/hash করে—এটি useful। কিন্তু package installed হওয়ার পরে payload change হলে launcher প্রতিবার verify করবে নাকি verified manifest cache trusted হবে, তার policy নির্ধারণ করতে হবে। Package modification race (TOCTOU) এড়াতে verified file descriptor বা immutable staged file use করা উত্তম।

Package lifecycle:

1. download to bounded temp file;
2. verify transport/TLS (without treating TLS as publisher trust);
3. verify package signature and hash;
4. validate manifest schema and app ID;
5. ask user about high-risk permissions;
6. stage files and fsync;
7. atomically install;
8. record installed version/trusted signer;
9. launch through nilrt;
10. audit state after failure;
11. update/rollback atomicity;
12. uninstall data cleanup/review।

Package manager network fetch, metadata refresh, signature revocation and offline install পৃথক feature হিসেবে test হবে। A package that signs with trusted key but requests unsupported permission should not silently receive broad access. Unknown manifest keys policy version অনুযায়ী reject/warn হবে; silently interpreting unsafe defaults নয়।

## ১৩.৫ OS update strategy

`nilupd`-এ signed update staging, inactive slot এবং rollback code থাকলেও bootloader slot-switching ও measured boot integration অসম্পূর্ণ হতে পারে। Update state machine আলাদা করে implement করতে হবে:

`Idle → Downloading → Verifying → Staged → Applying → PendingBoot → BootedUnconfirmed → Confirmed`;

failure branch: `Rejected`, `ApplyFailed`, `BootFailed`, `RollbackPending`, `RolledBack`। প্রতিটি transition atomic persistent journal-এ লিখতে হবে। Invalid transition হলে state machine fail করবে। একই update পুনরায় apply করা idempotent কি না, disk full, reboot, power loss এবং corrupted journal test হবে।

A/B strategy-র acceptance:

- active slot never overwritten during staging;
- inactive slot full image verify;
- boot metadata signed/authenticated;
- new slot has bounded boot attempts;
- `nilinit`/boot success signal থেকে active slot confirmed;
- service health failure boot success confirmation আটকায়;
- failed attempts exhausted হলে previous slot boots;
- user data schema backward compatibility আছে;
- downgrade/replay blocked by monotonic version/rollback index;
- manual recovery can restore known-good image;
- release manifest binds target device/codename and image hash।

## ১৩.৬ Update channel ও release manifest

Stable, beta, dev channel থাকলেও প্রথম release-এ একটাই development channel যথেষ্ট। Signed manifest-এ target, version, source revision, kernel image hash, initramfs hash, rootfs/system hash, required bootloader, compatible device profiles, min updater version, rollback index, signing key ID, release notes এবং known issues থাকবে। App `.nilax` package signing key এবং OS release signing key আলাদা রাখা উচিত; compromise impact ছোট হবে।

Update download endpoint HTTPS হওয়া উচিত, কিন্তু TLS certificate verification package signature-এর বদলি নয়। Mirror compromised হলেও trusted key signature mismatch update reject করবে। Update client timeouts, retry/backoff, partial downloads, disk quota, battery/charger preconditions এবং network interruption handle করবে। User interface “Downloading”, “Verifying signature”, “Staging”, “Reboot to finish”, “Rolled back” state সত্যিকারের updater status থেকে দেখাবে; static animation নয়।

## ১৩.৭ Recovery এবং stock restore

প্রথম native phone experimental release-এ recovery plan optional documentation নয়, mandatory artifact হবে। Exact stock firmware source, version, checksum, bootloader unlock implications, partition backups, slot state, anti-rollback version এবং restore command documented থাকবে। Recovery procedure অন্য model-এ reuse করা যাবে না।

Preflight tool-এ destructive actions আলাদা phase:

- `inspect` — read-only;
- `backup` — selected data/partition backup with hash;
- `verify-image` — local artifact only;
- `dry-run` — exact plan print;
- `flash-boot-test` — if device supports a temporary/non-persistent boot path;
- `install` — target-specific partition write, explicit confirmation;
- `verify-after-flash` — read-back or boot evidence where feasible;
- `recover` — documented stock/known-good restore।

`userdata` wipe কখনও implicit step হবে না। If wiping is required, tool will stop, explain affected partition, request the exact device identity confirmation, and log that consent occurred. A second confirmation can be required for irreversible storage changes. If image target profile mismatch, partition too small, bootloader locked or trusted vbmeta key missing, flash abort হবে।

---

# অংশ ১৪ — Milestone 10: UI shell, native UX ও system apps

## ১৪.১ Demo UI থেকে বাস্তব system UI

বর্তমান simulator ও shell-এ home, lock screen, settings, phone, messages, files, browser, terminal, notifications ইত্যাদি screen আছে। এগুলির কিছু interactive হলেও status values, notification history, phone call state, battery/signal, weather/date বা network details কোথাও কোথাও simulated। Maturity ledger-এ simulation চিহ্নিত করা একটি ভালো অভ্যাস। পরবর্তী কাজ হলো UI-কে backend status-এর সঙ্গে যুক্ত করা, যাতে user বুঝতে পারে কোন action বাস্তব, কোনটি unavailable এবং কোনটি demo।

একটি system UI-এর data source architecture স্পষ্ট হওয়া দরকার:

- clock/date: system time service ও timezone/locale;
- battery/charging: PowerHal real capability;
- network/signal: NetworkHal/telephony modem backend;
- notifications: actual notification IPC store, seeded demo data নয়;
- calls: actual telephony state machine অথবা host dialer handoff state;
- SMS: message storage + provider events;
- storage: mounted filesystem and actual free capacity;
- security status: queried runtime enforcement state;
- OS update: nilupd state machine;
- camera preview: actual frames from CameraHal;
- settings: service API-তে write-through with result/error; UI-only local toggle নয়।

যদি backend unavailable, UI-তে `Unavailable`, `Not connected`, `Permission required`, `Not supported on this target` বা `Demo data` ব্যবহার করো। Static default value production UI-তে দেখিয়ে “Connected” বা “Secure” বলা যাবে না।

## ১৪.২ Design system ও adaptive layout

NilUI/Alap UI একই application API বজায় রাখবে কিন্তু physical device-এর dimensions, density, safe inset, rotation এবং input type অনুযায়ী adapt করবে। Android-hosted S25-এ fullscreen Activity height, camera cutout, gesture navigation ও system bars behavior থাকতে পারে; native phone-এ display resolution, notch, edge radius এবং status area আলাদা হবে। 1080×2340 pixel ধরে সব coordinates hard-code করলে অন্য phone-এ layout ভাঙবে।

Design tokens:

- semantic colors: background/surface/text/primary/error/warning/success;
- typography scale এবং script-aware font fallback;
- spacing/radius/elevation tokens;
- responsive breakpoints logical dp-তে;
- safe area/insets;
- logical vs physical pixel transform;
- touch target minimum size;
- keyboard navigation focus order;
- screen reader/accessibility labels;
- reduced motion/animation limits;
- theme (light/dark/high-contrast);
- locale-aware date/time/number formatting।

বাংলা-first interface চাইলে শুধু string translate করলেই হবে না। বাংলা যুক্তাক্ষর shaping, Noto Bengali fallback, baseline/line height, Bengali numerals toggle, mixed Bengali/English technical text, keyboard IME composition, cursor movement এবং text selection পরীক্ষা করতে হবে। Search/input field-এ Unicode normalization এবং bidi/emoji behavior বিবেচনা করবে। Screen screenshot-এ বাংলা font fallback missing থাকলে তা release regression হিসেবে ধরা উচিত।

## ১৪.৩ UI component contract

প্রতিটি component-এর input/output behavior নির্দিষ্ট হওয়া দরকার। `Button`-এ enabled/disabled, loading, error, accessibility label, focus state, press state, click callback এবং debounce semantics থাকবে। `TextField`-এ IME action, selection, paste, password type, max length ও validation। `List`-এ stable key, scroll restoration, incremental load। `Dialog`-এ focus trapping, back button, cancel/confirm actions। `StatusBar`/notification shade-এ system gesture conflict এবং display insets।

NilLang UI compiler/VM-কে component semantics জানাতে হবে। Rendered label string-এ “Button Launch” দেখা আর actual Button click handler invoke হওয়ার মধ্যে ব্যবধান আছে। First vertical slice-এ Button press target ID dispatch করবে, VM event map-এ handler খুঁজবে, state update transaction চালাবে, UI re-render হবে। Unknown action safe error হবে; arbitrary function name invoke হবে না।

## ১৪.৪ Home/launcher

Launcher install manifest থেকে app list পড়বে; hard-coded app tile list নয়। App icon, display name, version, permissions, trust status ও launch availability package database থেকে আসবে। Invalid signature বা incompatible architecture app launch করা যাবে না। App update/upgrade-এ icon/name caching invalidate হবে। App uninstall করলে launcher stale shortcut দেখাবে না।

Launcher test:

- empty app list;
- many apps/paging;
- package install/remove;
- untrusted package hidden/blocked;
- crash loop indicator;
- resumed app state;
- display size/DPI adaptation;
- screen reader labels;
- orientation/insets;
- missing app icon fallback;
- app with unavailable capability;
- profile/user separation (যদি implemented)।

## ১৪.৫ Lock screen, OOBE ও security settings

OOBE state persistent config থেকে আসবে। OOBE complete marker atomic write হবে এবং filesystem data reset-এর পরে false হবে। Lock screen PIN check secure verifier use করবে; plain PIN বা reusable hash log file-এ থাকবে না। Attempt count/rate limit persistence reboot দিয়ে bypass করা যাবে না। Recovery reset-এর behavior user data encryption design-এর সঙ্গে consistent হবে।

Security settings screen-এ per-feature details দেখাও:

- device encryption: Enabled / Disabled / Unknown;
- app sandbox: Enforced / Degraded / Test mode;
- SELinux: Enforcing / Permissive / Disabled / Unavailable;
- verified boot: Verified / Unverified / Unsupported / Unknown;
- OS image revision/hash;
- release signing key ID;
- last successful update;
- current slot and rollback state (if applicable);
- developer options status;
- permission grants/revocations।

এগুলির status trusted system service থেকে signed/authorized response-এ আসবে। UI string hard-coded “Secured” হওয়া security guarantee নয়। Unknown state-কে green success badge দেওয়া যাবে না।

## ১৪.৬ Settings বাস্তবে কাজ করানো

প্রত্যেক settings control-কে service method-এ bind করো। Brightness toggle click → command request → HAL returns accepted/request ID → backend response → actual brightness query → UI display. যদি hardware interface unavailable হয়, switch rollback করে error দেখাবে। UI state আগে বদলে পরে backend fail হলে rollback বা `pending` status থাকবে।

Settings backend API-এর জন্য read-modify-write semantics, authorization, persistence ও validation প্রয়োজন। Screen timeout negative/oversize হলে reject; unknown Wi-Fi backend হলে toggle disable with reason; security setting permission ছাড়া change করা যাবে না। Settings config file edit করলেও service running state update না হলে displayed settings truth claim করা যাবে না।

## ১৪.৭ Notifications

বর্তমান notification prototype fabricated notification history এবং SMS items মিশিয়ে দেখাতে পারে। Real notification model-এ unique ID, origin service/app, timestamp, priority, text, read/dismiss state, action list ও privacy visibility থাকবে। Notification permission policy থাকবে। App user data-র sensitive notification lock screen-এ redact হবে। Seeded messages development-only fixture হিসেবে test profile-এ থাকবে; release builds-এ real empty state দেখাবে।

Notification daemon-এর test:

- post/update/dismiss;
- app uninstalled/permission revoked;
- reboot persistence policy;
- duplicate ID handling;
- timestamp/timezone correctness;
- lock-screen privacy;
- excessive notification flood rate limiting;
- accessibility actions;
- process restart recoverability।

## ১৪.৮ Files, browser ও terminal

Files app-এ rootfs-এর protected path দেখানো UI-only permission boundary নয়। Files app নিজেই service API দিয়ে allowed paths access করবে; path traversal, symlink escape, mount namespace mismatch, permission denial এবং write-to-system behavior পরীক্ষা করতে হবে। System root immutable হলে user action দিয়ে `/usr/bin` edit করা যাবে না। App-private storage ও shared media path আলাদা দেখানো হবে।

Browser app WebView বা standalone browser engine ব্যবহার করলে network permission, TLS errors, download path, safe browsing/security policy, mixed content, JavaScript interface এবং bridge exposure threat model-এ থাকবে। WebView-এর সাথে privileged native bridge expose করলে arbitrary page code Onuron commands trigger করতে পারে; origin allowlist ও minimal bridge API দরকার। Android-host mode-এ webview host Android API-র অধীন, native phone-এ আলাদা engine ও kernel integration হতে পারে।

Terminal app diagnostic command চালালে shell injection এড়াতে command allowlist/argument array ব্যবহার করবে। User-provided string shell-এ concatenate করবে না। “ps/services/net” output real process/IPC state থেকে আসবে; simulator-only fake commands clearly marked. Recovery/privileged diagnostics developer-only controlled access পাবে।

## ১৪.৯ Accessibility ও input methods

Accessibility production requirement, later polish নয়। Touch target, semantic label, keyboard traversal, contrast, text scaling, screen reader, reduced animation, locale/font fallback এবং error announcement-এর test matrix থাকবে। Bengali IME/keyboard integration target platform অনুযায়ী আলাদা হতে পারে: Android-host mode system IME ব্যবহার করবে; native Linux mode-এ input method daemon, layout configuration ও Unicode composition দরকার। `nilttsd` text-to-speech prototype থাকলে real voice backend, locale availability ও fallback status আলাদা থাকবে।

## ১৪.১০ UI quality gate

UI feature release-ready ধরা হবে যখন:

- actual data source আছে;
- empty/loading/success/error/unsupported state আছে;
- screen size/density adaptation আছে;
- touch/keyboard/accessibility semantics documented;
- app resume/rotate/destroy lifecycle pass;
- no fake data in production profile;
- string resources/localization complete;
- screenshot/e2e tests cover key flows;
- service failure UI freeze বা false success ঘটায় না;
- maturity table evidence path updated।

---

# অংশ ১৫ — Milestone 11: System service catalogue ও dependency management

## ১৫.১ সব daemon একসঙ্গে চালু করার প্রয়োজন নেই

রিপোজিটরিতে অনেক daemon আছে—যেমন `nild`, `nilkeyd`, `nilbus`, `netd`, `audiod`, `powerd`, `notifyd`, `nilimed`, `nilttsd`, `nilandroidd`, `camerad`, `telephonyd`, `nilupd` এবং আরও tooling। সব process initramfs-এ copy হয়েছে কি না, services.toml-এ আছে কি না এবং boot-time এ required কিনা—এগুলি এক তালিকায় মিলতে হবে। Binary list-এ camera/telephony আছে বলে service boot-এ start হয় না; config-এ entry না থাকলে তা অনুপস্থিত। একইভাবে optional service না থাকা core boot failure হওয়া উচিত নয়।

Service catalogue একটি source of truth-এ থাকবে:

- service ID;
- executable path;
- target availability;
- criticality;
- dependency list;
- run user/group;
- Linux capabilities;
- sockets/ports;
- permissions;
- readiness probe;
- restart policy;
- graceful shutdown signal/timeout;
- config file;
- backend capability dependencies;
- logging destination;
- maturity tier।

এটি generate করে `services.toml`, initramfs required binary list, docs এবং maturity checks update করা যেতে পারে। অতিরিক্ত generator maintain করা কঠিন হলে অন্তত CI test লিখে config-এর exec paths rootfs manifest-এর সঙ্গে compare করো।

## ১৫.২ Core service definition

প্রথম core service list কম রাখো। `nild` (system IPC/daemon coordination), `nilkeyd` (key services), `nilbus` (internal bus), `netd`, `powerd`, `audiod` এবং `nilshell` বর্তমানে core health check-এ আছে। এই list বাস্তব dependency-র ভিত্তিতে রাখো। যদি `nilshell` না থাকলেও headless boot valid হতে পারে, তাহলে shell-কে critical service ধরা উচিত কি না পুনর্বিবেচনা করো। Core list requirement হিসেবে documented হবে।

Core service health check-এর 50ms settle time brittle হতে পারে। Slow device-এ service 50ms-এ socket bind না করে পরে healthy হতে পারে; current live-process handle check তা আলাদা করতে পারে না। Readiness timeout per service, bounded retry/backoff এবং total boot deadline রাখো। Boot duration performance target হবে, কিন্তু correctness timeout-এর চেয়ে আগে। Slow bootকে success marker তাড়াতাড়ি দিয়ে লুকানো যাবে না।

## ১৫.৩ IPC protocol versioning

Daemon IPC socket path, request/response schema, authorization policy ও timeout versioned হবে। `/run/nilos` compatibility path ও `/run/onuron` canonical path একসঙ্গে থাকলে migration strategy লিখতে হবে; path alias-এর ফলে permission mismatch বা duplicate daemon connection হতে পারে। নতুন API client server version probe করবে। Unsupported protocol version হলে explicit error।

IPC message schema-এ:

- message type;
- schema/protocol version;
- request correlation ID;
- caller identity/peer UID;
- payload length bound;
- timeout/deadline;
- response/error enum;
- idempotency key for write operations;
- audit metadata without secrets।

Unix socket peer credentials (`SO_PEERCRED`) use হলে policy file-এ allowed caller UID/service ID mapped থাকবে। Unknown UID বা malformed peer identity default deny হবে। Socket file mode `0666`-এর মতো broad access avoid করো। User-facing app-কে root service socket-এর direct access না দিয়ে broker/control API ব্যবহার করতে দাও।

## ১৫.৪ Network service, VPN, DNS এবং updates

`netd`, `dnsd`, `vpnd`, `ntpd`, `nilsr` ইত্যাদি নাম থাকা একাই service completeness প্রমাণ নয়। প্রথমে network architecture স্থির করো: কোন daemon interface configure করে, কোন daemon DNS policy, VPN routing, time sync, captive portal এবং update download control করে। একই interface `netd` ও NetworkManager দুজন একসঙ্গে পরিবর্তন করবে না। QEMU user-mode NAT network-এ real public internet integration সীমিত; CI-তে DNS/external network flaky dependency কমাতে local test service ব্যবহার করা যেতে পারে।

Security requirements:

- DNS changes authenticated service request থেকে;
- VPN failure policy (fail-open/fail-closed) explicit;
- network namespace permission integrated;
- trusted time absent হলে certificate verification behavior;
- TLS cert errors fail closed;
- firmware/kernel downloads checksum verify;
- proxy credentials secure storage;
- captive portal redirects not treated as success;
- Wi-Fi password logs redact;
- network daemon reboot/reconnect test।

## ১৫.৫ Audio/media service dependencies

`audiod`/`mediad`/`camerad`/`nilimed` service names config ও build image-এ consistent রাখতে হবে। `audiod` ready হওয়ার আগে camera/video notification service sample playback request করলে behavior defined হবে। Audio route changes during phone call, notifications, headset insert/remove এবং suspend/resume service bus event হিসেবে model হবে। Backend operation fail হলে service error report করবে; mock audio provider silently use করবে না।

`mediad`/codec processing sandboxed app payload থেকে request নেয়; codec libraries untrusted media parse করতে পারে বলে privilege separation দরকার। Camera daemon camera ownership/permission, buffer pool and lifecycle manage করবে; app crash হলে camera resource release হবে। `nilimed` keyboard/input method service process isolation ও app text privacy handle করবে।

## ১৫.৬ Crash reporter, logging ও metrics

`logd` ও `crashd` থাকলে তাদের data schema, rotation, retention, redaction ও permission define করতে হবে। Production phone-এ logs বড় হয়ে `/data` fill করলে boot, update বা camera capture fail করতে পারে। Log retention policy storage quota-র সঙ্গে মিলবে। Crash dumps-এ passwords, SMS bodies, keys, location বা user app memory sensitive হতে পারে—default collection সীমিত, opt-in diagnostic export এবং sanitization চাই।

Metrics-এর উদ্দেশ্য performance/debug; user tracking নয়। Per-device test metrics local evidence-এ থাকতে পারে: boot time, service start time, frame latency, queue overflow, memory peak, battery drain, thermal state, crashes/restarts, network reconnect. Telemetry opt-in, privacy policy এবং retention rule ছাড়া remote upload করা হবে না। No-telemetry philosophy দাবি করলে actual code path/documentation তার সঙ্গে consistent থাকতে হবে।

## ১৫.৭ Service observability interface

Diagnostics service-এ একটি structured status endpoint রাখো, যেমন JSON:

```json
{
  "service": "netd",
  "state": "degraded",
  "ready": false,
  "backend": "linux-sysfs",
  "last_transition": "2026-10-09T00:00:00Z",
  "capabilities": ["link-state", "dns-state"],
  "missing_capabilities": ["wifi-scan", "cellular-toggle"],
  "last_error": {"code": "unsupported", "message": "..."}
}
```

এটি format illustration; actual timestamp UTC standard ও schema version declare করবে। Diagnostics UI status endpoint থেকে render করবে। Static `service active` label নয়; `active`, `ready`, `degraded`, `failed`, `not-installed`, `unsupported` আলাদা state থাকবে।

---

# অংশ ১৬ — Milestone 12: performance, reliability ও soak testing

## ১৬.১ Functional tests-এর পরে performance

OS-কে “lightweight” বলা হলে target hardware-এ measurable memory/CPU/boot/storage metrics থাকতে হবে। QEMU-র 1 GB RAM ও 2 vCPU configuration baseline হতে পারে, কিন্তু এটি S25 বা OnePlus-এর real performance measure নয়। Phone-specific performance target hardware selected হলে স্থির করতে হবে।

Measure:

- cold boot and warm boot time;
- kernel/initramfs size;
- rootfs size and installed binary count;
- idle memory/RSS per daemon;
- daemon start latency;
- home screen first-frame time;
- touch input-to-render latency;
- frame drops and average/p95/p99 frame time;
- app launch time;
- app package verification time;
- update download/stage time;
- filesystem write latency;
- idle battery drain;
- screen-on battery drain;
- temperature under sustained load;
- crash/restart frequency।

Metric threshold hardware baseline ছাড়া set করা যাবে না। প্রথমে measurement artifact capture, তারপর target-specific threshold define। CI hosted runners timing noisy হতে পারে; hard gate শুধু stable metrics-এ, noisy performance trends regression warning দিয়ে দেখানো যায়।

## ১৬.২ Boot reliability test

QEMU-তে 10–50 repeated boot test দিয়ে শুরু; native phone-এ serial console/USB logs capture করতে পারলে একই test methodology ব্যবহার করো। Cold boot, reboot, recovery boot, previous failed boot count, `/data` absent, service missing, disk read-only, malformed config, no network এবং slow service startup cover করো। Failure injection controlled test image-এ হবে, user's data partition-এ নয়।

Boot success criteria include all mandatory services ready, `/data` persistent or explicitly degraded, verified security state known, UI service launched if expected, and boot metadata confirmed. Boot marker print হলেই test success নয়। Test harness should wait for a structured readiness marker then query services. If boot hangs, capture logs until timeout and terminate QEMU cleanly. `panic=-1` বা `no-shutdown` options failure context capture-এ helpful but need timeout guard.

## ১৬.৩ App lifecycle and stress

Native app runtime-এ app launch/close/update/permission revoke/power suspend/service crash tests থাকবে। Stress test random UI events generate করতে পারে, কিন্তু app-এর privileged API-তে destructive action যেন invoke না করে। VM bytecode fuzzing, manifest parser fuzzing, package archive fuzzing এবং IPC parser fuzzing untrusted input hardening-এর জন্য উপযোগী।

Property-based test examples:

- accepted package ID never contains path traversal;
- invalid signature never installs;
- unknown permission never grants privilege;
- invalid bytecode never crashes PID 1 or system daemon;
- repeated start/stop leaves no leaked process/FD;
- random touch event cannot create memory growth without bound;
- update rollback restores prior verified image state;
- registry corruption never allocates duplicate live UID;
- permission revocation prevents next operation;
- timeout returns bounded time rather than hanging forever।

## ১৬.৪ Memory and queue limits

`GLOBAL_CAMERA_FRAMES`, audio buffers, event queue, command queue, IPC payloads এবং frame buffers bounds ছাড়া রাখা যাবে না। Current camera queue size limit of 8 frames, audio ~2 seconds of 48 kHz mono samples ইত্যাদি prototype limits আছে; এগুলোর memory budget/usage documented হবে। Queue full হলে drop-oldest/drop-newest/backpressure policy use-case অনুযায়ী স্থির করো। Silent loss diagnostics counter বাড়াবে। Camera frame queue item size maximum enforce; malicious oversized frame bytes memory exhaustion ঘটাতে পারে।

Android-host frame copy (full buffer cloning) memory pressure তৈরি করতে পারে। Reusable buffers/ring buffer বা shared memory design measurement-এর পরে বেছে নাও। Shared memory introduction করলে lifetime, synchronization, stale frame, lifecycle destruction এবং security access checks দরকার। Correctness আগে, optimization পরে।

## ১৬.৫ Reliability test environments

তিনটি test environment রাখো:

- **host unit tests:** fast/pure logic, parser, schema, policy planning;
- **QEMU integration:** Linux process, namespace where privileged runner permits, initramfs, mount, service, persistent disk, network interface, boot/reboot;
- **physical hardware:** display/touch, battery/charge, radios, camera/audio, suspend/resume, boot/recovery/thermal।

এক environment-এর ফল অন্য environment-এর নামে claim করা যাবে না। Host Windows test `spawn_sandboxed` non-Linux passthrough path-এ গেলে সেটি Linux namespace enforcement test নয়। CI container unshare EPERM পেয়ে development fallback-এ গেলে তা production sandbox successful test নয়। Test report-এ execution environment field রাখো।

## ১৬.৬ 24-hour soak test

24-hour soak test native phone bring-up-এর প্রথম দিনেই নয়, basic boot/storage/display/network stable হওয়ার পরে। Test profile documented করবে:

- brightness level;
- Wi-Fi state;
- screen-on/off schedule;
- background service set;
- app launch/close cycle;
- memory/log monitoring frequency;
- CPU/thermal sampling;
- power source and battery condition;
- network workload;
- expected restart policy;
- stop-on-critical-failure conditions।

Soak শেষে process count/memory growth, log size, CPU wakeups, service restarts, battery drain, filesystem errors, kernel warnings, camera/audio crashes ও data integrity report দেবে। 24h soak pass করলেও call/camera বা verified boot supported প্রমাণিত হয় না; এটি stability evidence-এর একটি মাত্র স্তর।

## ১৬.৭ Reliability release gate

Release candidate-এর জন্য minimum:

- repeated clean boot/reboot;
- app install/uninstall/update/rollback;
- permission revoke after app has started;
- core daemon kill/restart;
- persistent data integrity;
- low storage/invalid package;
- unsupported hardware backend behavior;
- no false success on command failure;
- 24-hour stability run for declared supported device;
- test logs and known issue list;
- recovery procedure tested on exact device/variant।

---

# অংশ ১৭ — বাস্তবায়নের সময়রেখা ও কাজের অগ্রাধিকার

## ১৭.১ সময়কে calendar promise নয়, milestone-এ ধরো

একজন developer-এর available time, existing coding experience, hardware access এবং build failures না জেনে নির্দিষ্ট release date দেওয়া দায়িত্বশীল হবে না। তাই নিচের সময়রেখা **প্রস্তাবিত কাজের ক্রম**, নিশ্চয়তাপ্রাপ্ত সময়সীমা নয়। প্রতি সপ্তাহের শেষে acceptance criteria অনুযায়ী milestone complete/blocked/partial status নথিবদ্ধ করতে হবে। যদি কোনও phase fail হয়, পরের phase শুরু করার বদলে failure root cause ঠিক করা হবে।

### Stage A — Foundation cleanup

Scope: Android-host flaky test, build artifact cleanup, host/target binary validation, kernel hash enforcement, image builder error behavior, docs correction।

Expected output:

- stable Cargo unit/integration tests;
- no tracked generated `.gradle`/APK output;
- explicit build manifest;
- no placeholder image described as bootable;
- test documentation reflects current CI;
- `docs/reference-board.md` no longer claims ARM64 script does not exist when it now does, but still labels ARM64 boot as unvalidated until evidence exists।

Exit gate: clean repository; x86_64 CI all green; wrong-arch and bad-hash negative tests pass.

### Stage B — ARM64 QEMU boot

Scope: toolchain, `aarch64-unknown-linux-musl` build, target rootfs, pinned kernel, formatted data disk, boot script, smoke tests, CI matrix, persistence test।

Expected output: one command from clean clone builds and boots ARM64 QEMU; manifest identifies exact kernel/userspace; services become ready; `/data` persistence works across reboot; CI artifact captures log.

Exit gate: ARM64 QEMU run repeatedly passes. Not just shell opens; service-health, mount and persistence criteria must be met.

### Stage C — Hosted Android vertical slice

Scope: reproducible Gradle+NDK build, native `.so`, JNI integration, surface render, touch input, lifecycle, actual battery state, permission errors. Camera/audio/telephony can be separated after display/input. First release should avoid privileged host actions not ready for safe testing.

Expected output: user installs APK on S25; about page accurately says Android-hosted; native handshake success visible; a Rust-rendered sample frame appears on screen; touch event round-trip changes a test state; missing permissions produce clear errors.

Exit gate: clean APK build in CI; device smoke test repeated; no queue race or fake hardware success in production profile.

### Stage D — Security and native app pipeline

Scope: sandbox positive/negative tests, per-app UID registry, SELinux loading, trusted publisher install, NilVM widget event path, nilrt-launch process integration, OS update manifest integrity, release key management plan.

Expected output: Hello app really installed/verified/launched in sandbox, UI responds; untrusted/modified package refused; denied permission cannot perform operation; security status is queryable rather than static text.

Exit gate: tests on Linux runtime with security-relevant syscalls allowed; unprivileged CI fallback explicitly separated; security claim table updated.

### Stage E — Phone port preflight

Scope: choose one device, source-lock, kernel build instructions, boot image layout, partition map, recovery and backup, target profile, hardware matrix, kernel console boot test. No broad production release claims.

Expected output: reproducible device kernel/initramfs artifact and documented recovery. First milestone can be early-console boot rather than GUI.

Exit gate: same exact device variant identified; developer can restore stock/known-good image; no generic partition writes; kernel/initramfs boot into `nilinit` or collect diagnostic evidence if blocked.

### Stage F — Hardware enablement and experimental release

Scope: storage, display/touch, power/charging, networking, audio, camera/telephony as feasible; OTA, rollback and long-run stability. Each feature is added only after driver/backend test and UI integration.

Expected output: one device profile with honest status table and reproducible image. Not every phone is supported; only target profile is claimed. Known limitations documented before public sharing.

Exit gate: device matrix evidence, installation/restore instructions, tested rollback, release artifact signing, known issues, no fake-success path.

## ১৭.২ সর্বোচ্চ অগ্রাধিকার — P0

এই কাজগুলি অন্য feature-এর আগে শেষ করতে হবে:

1. `test_command_and_audio_queues` regression guard এবং test state isolation;
2. build script-এর “success” message আর real artifact verification এক করা;
3. host binary ARM64 rootfs-এ ঢোকা বন্ধ;
4. kernel SHA mismatch-এ hard failure;
5. ARM64 blank `data.img` formatting ও mount validation;
6. `qemu-aarch64.sh` flag bug fixed;
7. ARM64 build + QEMU CI job;
8. stale docs/README maturity claim update;
9. generic flasher real phone-এ ব্যবহার নিষিদ্ধ/guarded রাখা;
10. tracked generated artifacts পরিষ্কার।

P0 সম্পূর্ণ না হলে camera UI, app store polish, more NilLang syntax, browser features বা Android compatibility container-এর মতো feature যোগ করা development focus ছড়িয়ে দেবে।

## ১৭.৩ উচ্চ অগ্রাধিকার — P1

- ARM64 service health and readiness protocol;
- real persistent storage plus reboot test;
- build artifact manifest/reproducibility;
- Android native library build integrated with Gradle/CI;
- actual display + touch JNI round-trip;
- permission runtime UX;
- `nilrt-launch` + NilVM e2e pipeline;
- SELinux compile/load failure correctness;
- signed publisher trust and package installation negative tests;
- device target decision ADR;
- phone-specific read-only preflight tool।

## ১৭.৪ P2 — hardware features

- Linux evdev input backend;
- DRM/KMS presentation on target;
- power/battery sysfs integration;
- network manager integration;
- audio playback/capture;
- camera V4L2/native or Android Camera2 hosted bridge;
- Bluetooth service;
- telephony modem stack;
- suspend/resume and thermal test;
- crash log export/diagnostics;
- UI accessibility and Bengali input polish।

P2 features-এর মধ্যে device support অনুযায়ী অগ্রাধিকার বদলাতে পারে। ফোনটিতে camera driver না থাকলে camera implementation অনন্তকাল blocker করে রাখা নয়; profile-এ unsupported বলা যায়। কিন্তু UI fake JPEG-কে camera success দেখানো চলবে না।

## ১৭.৫ P3 — release, ecosystem এবং extended support

- signed OTA distribution channel;
- stable/beta release promotion;
- app publisher key registration and revocation;
- app update/rollback UI;
- Android compatibility container;
- developer SDK/emulator integration;
- multiple device profiles;
- localization/accessibility completion;
- performance tuning;
- user documentation and support triage;
- long-term kernel updates and firmware inventory;
- reproducible build attestation।

Android compatibility layer (Waydroid/LXC-like/container idea) native boot, security and basic hardware support stable হওয়ার আগে primary milestone হওয়া উচিত নয়। Android apps চালানো সম্পূর্ণ পৃথক subsystem; hosted Android APK-তে Onuron UI দেখানো Android compatibility container নয়।

## ১৭.৬ Issue template

প্রতিটি task-কে এই ফরম্যাটে লেখা উচিত:

**Title:** target + behavior + failure condition।

**Current behavior:** নির্দিষ্ট file/path এবং observed result।

**Desired behavior:** externally observable change; শুধু “improve” নয়।

**Target:** Windows host / Linux host / x86_64 QEMU / ARM64 QEMU / Android-host / device codename।

**Preconditions:** kernel/build/toolchain/image/device state।

**Implementation notes:** সংশ্লিষ্ট crate/daemon/config, migration risk, security boundary।

**Positive tests:** সঠিক inputs-এ expected result।

**Negative tests:** missing device, malformed input, permission denied, timeouts, corrupted data।

**Evidence:** CI URL, log artifact, screenshot/video (যেখানে দরকার), manifest/checksum।

**Docs touched:** README, maturity, hardware matrix, security notes, install guide।

**Done criteria:** checkbox list; সব pass না হলে issue closed নয়।

## ১৭.৭ Pull request sequence

একটি sensible PR sequence হতে পারে:

1. P0 test isolation + artifact cleanup;
2. binary architecture validator + kernel hash fail-closed;
3. target configuration model refactor;
4. ARM64 toolchain/build stage;
5. ARM64 filesystem creation;
6. ARM64 QEMU script fix and tests;
7. ARM64 CI job;
8. services readiness schema;
9. persistent-data integration on AArch64;
10. Android NDK reproducibility and Gradle integration;
11. JNI instrumentation tests;
12. Surface rendering slice;
13. touch/input slice;
14. Android permission-state slice;
15. NilLang launch/runtime slice;
16. sandbox/SELinux negative tests;
17. device profile ADR;
18. device kernel/build source pin;
19. native boot console image;
20. storage, display/touch hardware gates;
21. power/network/audio capabilities;
22. final release/recovery workflow।

প্রতিটি PR ছোট রাখতে হবে। Build system overhaul এক বড় PR-এ করলে bisect ও review কঠিন হবে। Shared interface change হলে affected all backend implementations/tests একই PR-এ update করো যাতে interface contract compile error-এ ধরা পড়ে।

---

# অংশ ১৮ — Repository file-by-file action list

## ১৮.১ Build system

### `build/mkinitramfs.py`

Action list:

- architecture config validate, unknown target default না করে error;
- target-release binary-only rules;
- target ELF validator;
- kernel digest mismatch fatal;
- kernel download failure fatal unless explicit pre-fetched mode;
- initramfs manifest includes target triple and binary list;
- rootfs required binary map derives from services schema;
- `/data` mount support expected-device naming documented;
- output path canonicalized per target;
- reproducibility test per architecture;
- no hidden fallback to `target/release` for cross target;
- `--check-reproducible` works without network when rootfs supplied;
- stale kernel file with wrong digest does not pass on size alone।

### `build/qemu-aarch64.sh` and `.ps1`

- `--no-rebuild` value fixed and unit-tested;
- clean behavior explicit: build missing outputs or always rebuild based on flag;
- if build requested, compile target binaries first;
- `--no-disk` skips disk argument without accidental mount fallback ambiguity;
- disk formatter creates real filesystem;
- `--no-net` tests no network device, reports expected degradation;
- GUI serial option works on supported QEMU versions;
- invalid memory/SMP/CPU input rejected;
- QEMU version recorded;
- output log save mode available;
- exit code forwarded and child process cleanup on Ctrl-C;
- smoke mode non-interactive and timeout-controlled;
- command line manifest emitted।

### `build/build.py` and `build/build.sh`

- one canonical target schema;
- build return code checked at every stage;
- placeholder `system_a.img` and text `vbmeta_a.img` removed from real build path;
- OS image generation explicitly uses target architecture;
- image file system creation and partition table creation separated;
- `parted` failure handled; no `|| true` on required image operation;
- package list generated/validated;
- system/rootfs image actually includes intended files;
- release label only printed after post-build validation;
- build output not destroyed unexpectedly without clear warning or output dir scoped to target;
- build command has `--clean`, `--rebuild`, `--keep-output`, `--target` semantics documented;
- image checksums generated and verified;
- test image and flashable image file naming distinct।

### `build/mkimage-x86.sh`

- clarify x86-only helper name;
- verify GPT label and partition sizes;
- do not report success if `parted` absent/fails;
- create actual filesystem only if helper owns it; otherwise split partition layout and filesystem formatting responsibilities;
- expose partition table dump for validation;
- avoid sparse disk success message without valid GPT;
- no unintended overwrite of existing developer image;
- image partition offsets and size test across supported tool versions।

## ১৮.২ Android-host

### `android-host/Cargo.toml` and `build-ndk.sh`

- toolchain target check;
- `cargo-ndk` version pin or explicit NDK linker path;
- host prerequisites message;
- no implicit stale `.so` accepted after failed build;
- output `.so` ELF architecture check;
- ABI/Android API compatibility check;
- build manifest generated;
- build script exits non-zero when output missing;
- clean option only removes `jniLibs/arm64-v8a/libandroid_host.so`, not unrelated files;
- Gradle build invokes Rust build or CI enforces ordering;
- APK artifact contains expected `.so` entry verified with `unzip -l`/APK analyzer।

### `NativeBridge.java` and `jni_bridge.rs`

- define JNI contract document;
- choose canonical native library name;
- symbol mapping test;
- null/invalid length and exception tests;
- per-test BridgeState injection where possible;
- command/event schema version;
- bounded queues with metrics;
- frame buffer size and overflow validation;
- test lock limited to global integration fixtures;
- avoid poisoned lock cascades hiding first error;
- asynchronous response ID rather than boolean success only;
- bridge handshake reports native library loaded + protocol version;
- feature availability query from host API।

### `MainActivity.java`

- surface lifecycle callbacks validated;
- touch coordinates mapped to compositor dimensions;
- runtime permissions requested in context;
- permission denial returns status to Onuron UI;
- Android host status panel distinguishes actual/simulated data;
- app lifecycle unregisters receivers and stops threads;
- WebView JS bridge limited to safe methods/origins;
- phone call state not inferred from “dialer opened”;
- battery/connectivity callback cleans up on destroy;
- user data export/logging redaction policy।

### `OnuronBridgeService.java`

- foreground service requirements checked for supported Android versions, if long-running behavior needs them;
- worker thread cancellation/join on destroy;
- command allowlist and schema validation;
- only expected commands dispatched;
- camera/audio command paths implemented before marked supported;
- permission checks per action;
- handler does not silently swallow command failure;
- sensitive action confirmation policy;
- response/event flow back to Rust;
- request timeout/retry/idempotency;
- avoid polling at fixed 50 ms forever if proper blocking queue is available;
- lifecycle/restart tests।

## ১৮.৩ Runtime and init

### `nilinit/src/main.rs`

- validate critical mount results;
- use write-capable SELinux policy load path, check error;
- service startup follows dependency graph;
- actual readiness handshake;
- core-health output comes from readiness not just process presence;
- boot success only after all release-gate conditions;
- `/data` fallback status records degraded mode;
- persist failed boot counter atomically;
- shutdown/reboot syscall behavior implemented and verified;
- no success log on failed service/policy mount;
- target-specific boot reason parsing tested।

### `nilinit/src/supervisor.rs` and `activate.rs`

- service states: defined, starting, ready, failed, restarting, stopping;
- restart policy tested with real child process;
- one-shot service semantics;
- bounded backoff and circuit breaker;
- readiness timeout and rollback;
- socket FD handoff semantics defined;
- activation request not dropped;
- stale socket cleanup and mode/owner;
- shutdown sends signal, awaits timeout and then kills;
- service environment/descriptor inheritance audit;
- process identity and logs redacted।

### `etc/nilos/services.toml`

- compare all exec paths with each target's image manifest;
- core vs optional services declared;
- socket activation entries only for services implementing matching FD protocol;
- remove or document alias `/run/nilos` and canonical `/run/onuron`;
- target feature condition supports hosted/virtual/native mode differences;
- service schema version validated;
- no missing binaries pass silently।

## ১৮.৪ Security, NilLang ও documentation

### `runtime/nilrt`

- sandbox unshare error handling;
- network namespace positive/negative test;
- UID registry concurrency/atomic write;
- `NIL_APP_UID` restrict to test mode;
- actual SELinux context application and read-back;
- strict permission test under privileged Linux CI;
- `nilrt-launch` understands NilVM bytecode payload;
- app process lifecycle stop/restart;
- device access through approved service;
- seccomp filters test on actual architecture।

### `security/selinux`

- remove `secilc ... || true` for required build;
- audit with real compiler semantics;
- isolated temp policy fixtures;
- policy load error visible;
- runtime enforcing check;
- process label validation;
- `neverallow` violation test proves actual compiler/audit failure;
- policy status documented by target।

### `runtime/nillang`, `runtime/nilui-gpu`, `Alap`

- formal bytecode/schema version;
- UI tree adapter;
- actual event handling;
- state update loop;
- trusted package lifecycle e2e;
- sandbox actual launch integration;
- Alap repo path/contract defined;
- compositor frame evidence;
- UI performance and memory budget;
- accessibility semantics test।

### Docs

- `docs/reference-board.md` updated: ARM64 scripts exist, but ARM64 boot still `NOT RUN` until evidenced;
- `docs/improvement-plan.md` current CI status updated;
- `docs/maturity.toml` truth table updated;
- `docs/completion-checklist.md` no stale “no green QEMU” phrase;
- `docs/s25-hosted-runtime.md` distinguishes hosted vs native and doesn't call S25 production hardware lab without measured feature evidence;
- README “100% Rust userspace” wording qualified because Android host includes Java/Kotlin side;
- hardware support table includes exact device/profile status;
- verified boot claims distinguish custom signed descriptor from bootloader-enforced verified boot;
- installation docs warn flashing is destructive and target-specific।

---

# অংশ ১৯ — পরীক্ষার পূর্ণ matrix ও test command catalogue

## ১৯.১ Test ID নীতি

প্রতিটি test case-কে stable ID দাও যাতে pull request, logs, issue, release checklist ও hardware matrix একই test-কে উল্লেখ করতে পারে। উদাহরণ:

- `BLD-001`: clean x86_64 build;
- `BLD-010`: ARM64 binary architecture validation;
- `KRN-003`: pinned kernel checksum mismatch fails;
- `QEMU-X64-001`: x86_64 boot + core readiness;
- `QEMU-A64-001`: ARM64 `virt` boot + readiness;
- `STO-010`: persistent write/reboot/read;
- `SEC-NS-001`: unprivileged namespace setup failure denies production launch;
- `SEC-NET-003`: network denied means no external access;
- `PKG-007`: untrusted publisher rejected;
- `NIL-VM-004`: click event updates state and redraws;
- `AND-HOST-005`: Rust JNI library loads in actual APK;
- `AND-DISP-010`: Rust-produced test frame appears on Surface;
- `DEV-BOOT-001`: chosen phone reaches `nilinit`;
- `DEV-REC-002`: recover to known-good image;
- `DEV-PWR-004`: suspend/resume repeated cycles.

Test ID names are illustrative; actual registry repository-তে একবার স্থির করবে। একই ID-র expected behavior নীরবে বদলানো যাবে না। Contract change হলে test definition ও version update হবে।

## ১৯.২ Clean checkout matrix

| Stage | Command / action | Pass condition |
|---|---|---|
| Rust host tests | `cargo test --workspace --all-targets` | all required tests pass, no ignored security test without reason |
| Clippy | `cargo clippy --workspace --all-targets -- -W clippy::all` | no lint failure |
| Format | `cargo fmt --all -- --check` | no formatting drift |
| Python builder tests | `python -m unittest ...` | required test suite completes successfully |
| Maturity drift | `python build/gen-maturity.py --check` | README generated table matches source file |
| Android UI build | `gradlew clean assembleDebug` | APK created from source |
| Android native build | `build-ndk.sh` then APK inspect | `.so` present with ARM64 ELF machine |
| x86 QEMU | `build/qemu-boot.sh` / smoke workflow | core services ready, actual storage validated |
| ARM64 QEMU | `build/qemu-aarch64.sh` after target build | ARM64 kernel/userspace boot evidence |
| package e2e | `cargo test -p nillang --test native_app_slice` plus process test | installed package launches real runtime path |
| device preflight | target-specific read-only inspect | exact codename and image profile match |

Project names and test commands should be confirmed with current workspace members; the table describes desired gate structure, not a claim that every command is already implemented as written. Avoid copying ellipses into mandatory workflow commands without expanding the actual test file list.

## ১৯.৩ Builder negative test matrix

Build scripts deserve negative tests because many prior OS bring-up problems come from “success-shaped” output despite missing artifact. At least these cases must be automated:

1. cargo not installed;
2. target Rust toolchain missing;
3. target binary missing;
4. wrong ELF architecture placed in `target/aarch64.../release`;
5. only host `target/release/nilinit` exists;
6. kernel download 404;
7. kernel download timeout;
8. kernel hash mismatch;
9. cached kernel hash mismatch;
10. initramfs contains no `/init`;
11. `services.toml` references missing binary;
12. output directory unwritable;
13. disk formatter missing;
14. formatter returns non-zero;
15. QEMU binary missing;
16. QEMU exits before readiness;
17. QEMU panics but prints a previous stale success log;
18. ARM64 target image receives x86_64 binary;
19. build interrupted midway and rerun;
20. manifest references missing artifact;
21. checksum file has wrong value;
22. target argument typo;
23. `--no-rebuild` with no output;
24. `--no-rebuild` with stale output from another target;
25. stale `out/` directory contains a previous valid kernel but changed userspace;
26. data image is blank/unformatted;
27. disk image has wrong filesystem type;
28. rootfs includes an Android `local.properties` file;
29. rootfs includes a private key fixture;
30. placeholder 1 MiB image reaches “success” branch।

প্রতিটি negative test-এ expected non-zero code এবং explicit reason থাকবে। “Script did not crash” pass condition নয়। Test wrapper-এ success log থাকলেও artifact validation fail করালে job red হবে।

## ১৯.৪ ARM64-QEMU boot matrix

ARM64 boot test-এ শুধু normal happy path নয়, device/boot environment failuresও থাকবে:

- `virt` CPU profile `cortex-a57`;
- 1 GB RAM / 2 vCPU baseline;
- কম RAM exploratory case;
- no disk;
- valid formatted data disk;
- unformatted disk negative case;
- no network;
- QEMU user-mode network;
- corrupt initramfs;
- wrong-arch userspace binary;
- missing `nilinit`;
- malformed service config;
- core service binary missing;
- core service start failure;
- core service starts but does not become ready;
- reboot persistence;
- boot counter and recovery mode;
- console routing and timeout;
- clean shutdown and forced kill।

কিছু failure test expected to boot into degraded/recovery mode rather than full system; test assert করবে exact expected state। Invalid rootfs-এ fail-closed behavior pass হতে পারে—একটি failure boot success নয়, কিন্তু intended safety outcome যদি test হয় তবে expected refusal-ই pass।

## ১৯.৫ Android-host device test matrix

Android test-এ app API level ও OS version record করতে হবে। S25-এ actual hardware path test হলেও অন্য Android host-এ সব API একইভাবে available ধরে নেওয়া যাবে না। Minimum scenarios:

- first launch, native library present;
- first launch, native library absent;
- wrong ABI library;
- JNI symbol mismatch;
- Activity pause/resume;
- screen off/on;
- rotation;
- permission first request;
- permission denied;
- permission revoked in system settings;
- camera hardware busy/unavailable;
- microphone permission denial;
- AudioTrack initialization error;
- Wi-Fi disconnected/connected;
- mobile data off;
- no SIM / restricted telephony;
- SMS permission unavailable;
- external dialer cancelled;
- app force-stop and relaunch;
- service recreated by Android;
- queue overflow and long-running session;
- device low-memory kill;
- screen render resize;
- native library missing after app upgrade।

Results device-specific হবে। Test logs-এ Android build fingerprint বা exact serial number public issue-তে শেয়ার করার আগে privacy review করতে হবে। Build fingerprint diagnostics-এ local থাকতে পারে; published artifact-এ sanitization policy প্রয়োগ করো।

## ১৯.৬ Test data discipline

Tests-এ বাস্তব ব্যক্তির phone number, contacts, actual SMS body, Wi-Fi credentials, IMEI, GPS traces বা signing keys ব্যবহার করা যাবে না। Fake/test fixtures clearly marked থাকবে। Telephony test-এর জন্য controlled test number, consented SIM, limited duration এবং actual operation-এর আগে human approval থাকবে। SMS/call action automation production bridge-এ test fixture দিয়ে enable করা যাবে না। Camera/audio recording-এর instrumentation consent ও local storage policy মেনে হবে।

## ১৯.৭ CI failure response

CI ব্যর্থ হলে:

1. প্রথম failed step-এর log দেখো, পরের skipped steps দিয়ে cause অনুমান কোরো না;
2. exact commit SHA এবং workflow/job ID note করো;
3. failure deterministic না flaky তা sequential/parallel test দিয়ে দেখো;
4. environment regression (toolchain/runner) আর code regression আলাদা করো;
5. reproducer command local/Linux container-এ চালাও;
6. minimum fix commit করো;
7. added test bug reproduce করুক, শুধু symptom mask নয়;
8. same SHA-তে required workflows green;
9. docs/status update;
10. failed experiment মুছে না রেখে root cause note রাখো।

A green workflow after a later fix means the latest SHA passed; it does not retroactively mean the prior SHA was green. Test report should state which SHA was validated. If a required workflow is missing, mark test `not run`, not pass.

---

# অংশ ২০ — Safe flashing workflow ও command-level operator guide

## ২০.১ কেন generic flash script আপাতত ব্যবহার করা উচিত নয়

বর্তমান `build/flash-device.sh` generic path-এ `boot`, `system` এবং `vbmeta` flash করার চেষ্টা করে এবং `userdata` format/erase করার code আছে। কিন্তু current builder সব ফোনের partition layout অনুসারে complete bootable image তৈরি করছে না; `system` vs `system_a` namingও device target-specific। তাই script-টি native phone installation interface হিসেবে verified নয়। User confirmation prompt থাকা মানেই command safe নয়।

Native flashing শুরু করার আগে flash script-কে device profile-aware এবং preflight validated করতে হবে। Script-এর target argument শুধু `aarch64-generic`-এর মতো generic string নেবে না; exact model profile ও codename match করবে। `fastboot getvar product` target profile-এর accepted identities-এর সঙ্গে compare হবে। Unknown var/empty response false match হিসেবে accept হবে না। Partition layout check complete না হলে `flash` command allowed নয়।

## ২০.২ Read-only preflight flow

প্রস্তাবিত operator flow:

```text
1. build manifest validate
2. target device profile load
3. adb/fastboot device identity inspect
4. model/codename/variant match
5. bootloader state read
6. partition map/slot data read
7. kernel/boot/system artifact hashes verify
8. trusted release signature verify
9. recovery artifact availability check
10. backup status confirm
11. dry-run plan print
12. explicit operator approval
13. write only target-specific partitions
14. read-back/reboot verification
15. recovery checklist and logs store
```

এই flow-তে actual command option ও partition names device profile থেকে আসবে। User manual-এর copy-pasted command বা README default flash command সরাসরি `fastboot flash system` চালাবে না। Dry-run output-এ serial/target, exact partition name, local image path, image size/hash, whether user data will be erased, expected slot state এবং recovery route দেখাতে হবে।

## ২০.৩ Backup policy

Native phone experimentation-এর আগে গুরুত্বপূর্ণ data backup হবে। Backup শুধু “অন্য কোথাও কপি করেছি” নয়; backup archive readable ও checksummed কি না যাচাই করতে হবে। Device-specific partitions backup করতে গেলে licensing/partition access/legal implications ও storage capacity বুঝে করতে হবে। Modem calibration, EFS/persist-like partitions, device identity/key material, secure storage partitions-এর সঙ্গে পরীক্ষা করা যাবে না; ভুল write-এ radio/identity নষ্ট হতে পারে। User data backup এবং boot image backup আলাদা বিবেচনা।

Device-এর stock firmware restore package public/official source থেকে নিলে exact region/version/codename মিলিয়ে hash verify করো। Bootloader unlock-এর সময় factory reset হতে পারে; এটি install script নয়, bootloader-এর behavior হতে পারে। Restore guide-এ exact version ও slot caveat থাকবে। Private user data বা proprietary blobs public repo-তে upload নয়।

## ২০.৪ First boot: low-risk stage

প্রথম native image-এ camera/modem/network stacks intentionally disable করা যেতে পারে। Minimal rootfs, console, block storage, `nilinit`, shell diagnostics এবং boot-health marker যথেষ্ট। Bootloader যদি temporary boot command support করে, প্রথমে permanent partition flash না করে temporary boot path ব্যবহার করা হবে; exact device অনুযায়ী support যাচাই করতে হবে। Temporary boot supported না হলে first write-এর আগে recoverability evidence mandatory।

First boot-এ log সংগ্রহ:

- host terminal stdout/stderr;
- serial console (যদি সম্ভব);
- kernel log/pstore on reboot;
- `nilinit` boot log;
- service health JSON;
- partition/slot pre-state;
- boot time, current image hash;
- observed UI/console photo (যেখানে উপযোগী)।

Boot success না হলে প্রথমে evidence সংগ্রহ, তারপর একবারে একটিই পরিবর্তন। একই সঙ্গে kernel, dtb, boot header, initramfs আর rootfs পাল্টালে causality হারিয়ে যাবে।

## ২০.৫ Device recovery drill

একটি device target support claim করার আগে recovery drill এমন developer করবেন যিনি image builder লিখেছেন এমন একজনই হতে হবে না। Independent step-by-step instructions ব্যবহার করে known-good image/stock state-এ ফিরতে হবে। Recovery drill-এ bad kernel, bad initramfs, boot loop, incomplete update, corrupted user data partition এবং slot mismatch scenario থেকে অন্তত safe subset পরীক্ষা করো। Protected identity/calibration partitions-এ corruption simulate করে real device risk তৈরি কোরো না; simulation image বা spare board ব্যবহার করো।

Recovery report লিখবে:

- starting device state;
- failure injected;
- visible symptom;
- recovery entry method;
- used artifact hash/version;
- data loss outcome;
- successful reboot evidence;
- manual intervention needed;
- unresolved risk।

## ২০.৬ Public install guidance

Public installation guide-এ prominent warnings:

- unsupported device-এ image flash নয়;
- S25 hosted APK এবং native image আলাদা;
- flashing user data erase করতে পারে;
- banking/DRM/carrier features ক্ষতিগ্রস্ত হতে পারে;
- bootloader unlock security/guarantee status পরিবর্তন করতে পারে;
- test image daily-use OS নয়;
- battery পর্যাপ্ত ও USB stable রাখতে হবে;
- target codename verify করতে হবে;
- checksum/signature verify ছাড়া download flash নয়;
- recovery route না থাকলে flash বন্ধ করো।

User interface বা README “Flashed Successfully” বলার আগে actual fastboot write return status, read-back/verification, device re-enumeration এবং expected boot result confirm করতে হবে। শুধু command successful exit করলেই OS boot হয়েছে নয়।

---

# অংশ ২১ — Architecture Decision Records (ADR)

## ২১.১ ADR কেন প্রয়োজন

OnuronOS-এ কয়েকটি foundational decision বদলালে বহু crate, script ও documentation প্রভাবিত হবে। সিদ্ধান্তগুলি chat বা commit message-এ scattered থাকলে পরের developer একই বিষয় আবার খুলবে। `docs/adr/`-এ প্রতি decision-এ Context, Options, Decision, Consequences, Security implications, Testing plan এবং Revisit criteria থাকবে। Accepted ADR পরে পরিবর্তন করা যায়, কিন্তু superseding ADR যোগ করতে হবে।

## ২১.২ ADR-001 — Native OS ও Android-host-এর সীমানা

**Context:** S25-এ Onuron UI চালানো সম্ভব হলেও Android OS-কে replace করা আর Android-এর ভিতরে hosted UI চালানো আলাদা।

**Options:** (A) hosted runtime-কে OnuronOS native বলে উপস্থাপন; (B) আলাদা mode ও product label; (C) Android kernel-এ partial userspace replace করার চেষ্টা।

**Decision:** B. `Hosted Runtime` এবং `Native OS` আলাদা mode, build target, security boundary ও maturity। S25 build-এ `Android host` visible metadata থাকবে। Native phone support claim hardware matrix-এ evidence ছাড়া হবে না।

**Consequences:** UI/code reuse হতে পারে, কিন্তু hardware API implementation আলাদা। App permission Android-এর অনুমতির বাইরে যেতে পারে না। Native OS release-এ boot chain ও recovery requirement পৃথক।

**Tests:** APK About screen target declaration, build manifest target string, native image target profile compare।

## ২১.৩ ADR-002 — Target identity and artifact naming

**Context:** `aarch64-generic`, `arm64-generic` এবং `aarch64-qemu` alias confusion তৈরি করতে পারে।

**Decision:** Internal canonical target ID: `qemu-x86_64`, `qemu-aarch64`, `android-host-arm64`, পরে `oneplus-fajita`। Legacy target aliases থাকলে deprecated map থাকবে। Rootfs/kernel/image output folder, manifest, CI artifact এবং docs একই canonical ID ব্যবহার করবে।

**Security consequence:** Target mismatch flash/build failure trigger করবে। Generic ARM64 artifact native phone flash করার জন্য automatically accepted হবে না।

**Tests:** invalid/legacy target alias, manifest/profile mismatch, wrong architecture artifact reject।

## ২১.৪ ADR-003 — Kernel source strategy

**Context:** বর্তমানে QEMU build-এ pinned-source strategy ও physical phone kernel source strategy আলাদা।

**Decision:** QEMU initial bring-up-এ known upstream distribution kernel may be used if checksum and provenance pinned; physical phone profile-এ exact source commit/patch/DTB/firmware lock হবে। Long-term kernel update strategy release support-এর অংশ।

**Alternatives:** Full custom kernel from scratch, upstream kernel with patches, vendor kernel reuse, Halium/libhybris. Selection will be recorded per device, not forced globally. OnePlus 6T candidate should build on existing Linux/community evidence rather than reinvent its boot path. S25 hosted mode Android kernel uses; it does not imply native S25 kernel source availability.

**Consequences:** Kernel version and features differ by target; userspace capability detection prevents assuming every HAL feature available. Patch maintenance responsibility becomes visible. Build reproducibility must pin source and toolchain.

## ২১.৫ ADR-004 — HAL error and capability semantics

**Decision:** HAL methods do not silently simulate success. Actual success, queued request, permission denied, unavailable device, unsupported capability, timeout and error are distinct. Fake backend is named/test-only or explicitly demo mode. Feature availability is queried and emitted to diagnostics. UI never infers a successful operation from UI button press alone.

**Consequences:** Existing prototype screens may show more “Unavailable” until underlying backend exists. This is an improvement, not a regression; it replaces false-positive claims with actionable truth.

## ২১.৬ ADR-005 — NilLang runtime model

**Context:** current NilVM parses/loads bytecode and renders a textual scene tree. It does not execute installed bytecode as a native machine executable.

**Decision:** `.nilax` is a signed package container; contained NilLang bytecode executes in NilVM/interpreter, not via native `execve`. `nilrt-launch` remains entry point for permission/lifecycle/sandbox; it invokes the configured runtime. The UI adapter connects VM scene and events to NilUI. Native machine code extensions, if ever added, require a separate threat model and signing policy.

**Consequences:** The term “native app” should be used carefully; “Onuron native NilLang app” means first-party runtime app, not necessarily CPU-native code. Documentation will clarify bytecode format and runtime boundary.

## ২১.৭ ADR-006 — Verified boot claim

**Decision:** Custom signed metadata is labelled integrity descriptor/prototype until a bootloader-enforced trust chain is validated. Release key must be pinned out-of-band; embedded self-signed public key is insufficient. Phone images require target-specific boot chain and recovery. Standard Android AVB compatibility may be claimed only after an interoperable artifact and real supported bootloader test.

**Consequences:** Maturity table may show cryptographic signing implementation as prototype, but cannot claim secure verified boot production-ready. Flash tool blocks native installation until trusted key / target profile conditions are met.

## ২১.৮ ADR-007 — Reference phone policy

**Decision:** One native reference device at a time. Initial candidate is OnePlus 6T if a confirmed unlockable, healthy unit and current community baseline can be obtained at an acceptable price; otherwise reconsider PinePhone or another phone with stronger openly documented Linux support. Galaxy S25 remains hosted-runtime device, not the native reference target unless a separate verified boot/device-port analysis changes the decision.

**Evidence basis:** postmarketOS has listed OnePlus 6T in the v26.06 release/testing ecosystem and described a real-device testing runner in its January 2026 update. That indicates useful community tooling, not automatic Onuron compatibility. Official/current install instructions and the precise hardware status must be rechecked before purchase or flash.

**Revisit criteria:** local price/availability, bootloader unlock state, battery health, exact variant, current kernel status, recovery route, camera/audio/modem needs and maintainer responsiveness. Decision is not irreversible; it becomes an explicit ADR revision.

## ২১.৯ ADR-008 — Release artifacts and signing

OS release signing key, app publisher keys and development/test keys will be separate. Private signing key will never be checked into Git, stored in an APK, emitted in logs or uploaded to public CI artifact. Build workflow can create unsigned test image; release signing happens in a controlled process. Public manifest has target ID, version, revision, hashes, signing key ID, min updater version, recovery instructions and known issues. Artifact provenance records the toolchain and source-lock references.

---

# অংশ ২২ — Risk register ও mitigation plan

## ২২.১ Technical risks

### R1 — ARM64 script exists but target build is not self-contained

**Likelihood:** High until the ARM64 build job is in CI. **Impact:** High; wrong binary or missing `nilinit` can prevent boot. **Mitigation:** architecture-specific build stage, ELF validator, no host fallback, ARM64 CI artifact. **Owner:** build system. **Exit:** clean build and wrong-arch negative test pass.

### R2 — Blank data disk silently becomes temporary storage

**Likelihood:** High in current ARM64 launcher. **Impact:** High; apparent settings/data vanish after reboot. **Mitigation:** format filesystem during image creation, verify UUID/type, boot logs mark persistent vs ephemeral. **Exit:** write/reboot/read test passes.

### R3 — Kernel pin exists but mismatch is not fatal

**Likelihood:** Current source demonstrates this gap. **Impact:** High; unverified kernel could be booted. **Mitigation:** checksum mismatch non-zero exit; negative tests for cached and downloaded artifacts. **Exit:** tampered kernel cannot reach QEMU command.

### R4 — Android native `.so` absent or wrong ABI

**Likelihood:** Medium/high until integrated build exists. **Impact:** Medium for demo, high for claims about hardware pipeline. **Mitigation:** Gradle/CI build dependency, APK inspection, instrumentation test. **Exit:** a real S25 APK loads native bridge and exposes runtime handshake.

### R5 — JNI queue race or poison state recurs

**Likelihood:** Medium because process-global queues shared among tests. **Impact:** Medium; flaky CI and false regression. **Mitigation:** isolate testable BridgeState, global test lock for true integration tests, repeated parallel test. **Exit:** stress/repeat passes with no unrelated test contamination.

### R6 — HAL returns success while hardware did nothing

**Likelihood:** High in prototype code. **Impact:** High for user trust and release readiness. **Mitigation:** typed backend results, remove silent no-op success, capability query, actual hardware test evidence. **Exit:** all production status values tied to source data.

### R7 — Security claims stronger than enforcement

**Likelihood:** Medium/high due incomplete boot chain/SELinux/network service policies. **Impact:** Very high. **Mitigation:** threat model, negative tests, pinned trust root, actual enforcing mode, update rollback. **Exit:** security release gate independently reviewed.

### R8 — Generic flashing damages device or erases user data

**Likelihood:** Medium, impact very high. **Mitigation:** do not use current generic flasher; per-target partition schema; read-only preflight; backups; recovery drill; explicit destructive action confirmation. **Exit:** recovery on exact target proven.

### R9 — Too many device targets split effort

**Likelihood:** High if new devices are added early. **Impact:** High schedule and support fragmentation. **Mitigation:** one native reference device; QEMU ARM64 and S25 hosted remain separate tracks. **Exit:** reference device ADR approved before target #2.

### R10 — Generated artifacts/private paths pollute repo

**Likelihood:** Present historical condition. **Impact:** Medium; noisy diffs, binary bloat, path disclosure. **Mitigation:** tracked-file cleanup, clean-clone CI, .gitignore regression test. **Exit:** post-build git status clean.

## ২২.২ Product and UX risks

### R11 — Fake status makes prototype look more complete than it is

UI may show VoLTE, connected Wi-Fi, a battery percentage, security enabled, or camera photo while backend is simulated. This reduces user trust and obscures engineering gaps. All simulated values get explicit source/mode labels; production mode blocks fallback success. README maturity and UI should agree.

### R12 — Bengali UI looks correct only on developer screen

Font, shaping, numeral/timezone, layout and IME can vary. Create visual regression screenshots for supported resolutions and use real Bengali text with যুক্তাক্ষর, punctuation, Latin tech terms, emoji and mixed digits. Test user font size and text scale. Native and hosted screens may use different rendering engines, so tests need to cover both.

### R13 — Daily-use claim before calls/data/charging works

A phone OS that boots but cannot reliably call, charge, resume from sleep or recover from update is not a daily driver. Keep “experimental development image” label until hardware matrix says otherwise. Do not ask users to rely on it as their main phone during bring-up.

## ২২.৩ Process risks

### R14 — Documentation drift

`docs/reference-board.md`, `docs/improvement-plan.md`, `docs/completion-checklist.md` and README previously differed about latest CI runs and ARM64 support. Use maturity-generation check, one source of truth, CI drift test and commit-specific evidence. Documentation is part of acceptance criteria, not optional cleanup.

### R15 — Big PRs hide root causes

Split bootloader, kernel, rootfs, HAL, UI and security changes. Each PR should add tests and make one measurable change. For a regression, bisectable commits help to identify which layer introduced failure.

### R16 — Toolchain drift breaks reproducibility

Unpinned Android Gradle plugin/NDK, host kernel download, Rust toolchain or QEMU runner version can change output. Pin versions where necessary, record versions in manifest, cache only content-addressed artifacts, and provide explicit upgrade PRs. Pinning every third-party tool without maintenance plan is not enough; schedule updates with security review.

### R17 — Firmware licensing or provenance unclear

Native mobile hardware often needs firmware. Do not redistribute blobs before license/provenance check. Keep external firmware optional/target-specific where possible; record hashes, source and applicable license. If users must extract firmware from existing Android firmware, write accurate instructions and avoid committing proprietary binaries without permission.

## ২২.৪ Risk escalation rules

- Any risk with potential phone brick/data wipe blocks flashing until mitigation documented.
- Any security control silently failing blocks release promotion.
- Any build artifact target mismatch blocks release.
- Any fake success state in production backend blocks feature maturity promotion.
- Any memory/queue growth that can crash essential service blocks native app runtime promotion.
- Any mismatch between device profile and hardware identity blocks install.
- A risk accepted for prototype remains in `known-issues.md`; it cannot disappear because the demo “looks good”.

---

# অংশ ২৩ — হাতে-কলমে প্রথম ৩০টি কাজ

এই তালিকা বিশেষ করে বর্তমান repo-তে কাজ শুরু করার জন্য। প্রতিটি task আগে branch-এ করো এবং প্রয়োজন হলে পৃথক PR বানাও। নিচের command-গুলি PowerShell বা Bash-এর উদাহরণ; local path ও tool installation অনুযায়ী adjust করতে হবে।

## কাজ ১–৫: বর্তমান অবস্থার snapshot

**১. Latest commit record করো।** `git rev-parse HEAD`, `git status --short`, `git log -5 --oneline` চালিয়ে output local developer note-এ রাখো। Audit-এর সময় branch বদলালে analysis-এর ভিত্তি বদলে যাবে। `main`-এর current remote SHA local tree-র সঙ্গে মিলিয়ে নাও।

**২. বর্তমান workflows inspect করো।** GitHub Actions-এ latest run SHA এবং প্রতিটি required workflow conclusion নোট করো। Failing old run এবং latest successful run আলাদা করো। “green now” এবং “historically had failure but fixed” একই অবস্থা নয়।

**৩. Generated artifacts তালিকাভুক্ত করো।** `git ls-files android-host/app/.gradle android-host/app/build android-host/Onuron.apk android-host/app/local.properties` চালাও। Tracked files-কে `.gitignore` দিয়ে বাদ যাবে ধরে নিও না।

**৪. Target release binaries তালিকাভুক্ত করো।** `target/x86_64-unknown-linux-musl/release` এবং `target/aarch64-unknown-linux-musl/release`-এ কোন executable আছে, একটি manifest বানাও। Host `target/release`-এর binary architecture record করো।

**৫. Reproducible baseline artifact রাখো।** Clean x86_64 build, manifest, QEMU log, persistent-data test result এবং workflow links `docs/evidence/qemu-x86_64/` অথবা CI artifact-এ রাখো। Personal SDK paths ও secrets বাদ দাও।

## কাজ ৬–১০: Build correctness

**৬. `ensure_kernel()` hash check fail-closed করো।** Wrong digest test যোগ না করে শুধু code rewrite করো না; test fixture-এ known bytes ও wrong bytes ব্যবহার করো।

**৭. Cross-target fallback নিষিদ্ধ করো।** ARM64 build-এ architecture-specific release directory missing হলে failure। `target/release` fallback release-mode-এ বন্ধ।

**৮. ELF architecture validator যোগ করো।** শুধু file name দিয়ে binary target চেনা যাবে না। Wrong ELF machine type reject করো এবং failure output-এ expected/actual architecture দেখাও।

**৯. Build success path verify করো।** Missing QEMU binary, missing `nilinit`, failed cargo command এবং image-creation failure non-zero exit দেয়। Any `|| true` required build operation-এ আছে কি না audit করো।

**১০. Maturity docs synchronize করো।** `docs/reference-board.md` ARM64 script-এর existence reflect করবে কিন্তু boot validation না হলে `NOT RUN` থাকবে। `docs/improvement-plan.md`-এর no-green-CI statements update করতে হবে।

## কাজ ১১–১৫: ARM64 disk ও launcher

**১১. `--no-rebuild` bug fix করো।** Script parse tests যোগ করো যাতে flag-এর state assert করা যায়। Existing kernel/initramfs থাকলে script কী করবে তা documentation-এ clear।

**১২. Data image formatter যোগ করো।** Blank `truncate`-এর পর filesystem format; expected `mkfs` utility missing হলে failure।

**১৩. Filesystem probe যোগ করো।** `blkid`, superblock parse বা equivalent method দিয়ে image filesystem signature verify; test image host-এ mounted করতে হলে privilege requirement documented।

**১৪. Rootfs architecture manifest যোগ করো।** Kernel architecture, userspace triple, installed binary architecture এবং target ID যাচাই।

**১৫. Launcher script-এ log capture option যোগ করো।** Interactive script ও automated smoke runner আলাদা। Serial console failure diagnostics output file-এ থাকবে। QEMU process timeout/cleanup behavior test করো।

## কাজ ১৬–২০: ARM64 CI

**১৬. Cross-build step যোগ করো।** Required Rust target ও linker toolchain setup; native dependencies থাকলে explicit packages।

**১৭. Kernel fetch/cache verify করো।** CI cache hit হলে hash আবার যাচাই হবে; old wrong file accepted হবে না।

**১৮. Initramfs package ও inspect করো।** Archive extraction/list, `/init`, required binaries, ELF machine type এবং config paths validate।

**১৯. ARM64 QEMU boot job যোগ করো।** Workflow-তে `qemu-system-aarch64` package ও invocation; x86 job-এর success-কে ARM64 job-এর বিকল্প হতে দিও না।

**২০. Data reboot test চালাও।** Same image attach করে write/reboot/read; tmpfs fallback হলে test fail হবে, “degraded but booted” বলে persistence test pass নয়।

## কাজ ২১–২৫: S25 host APK

**২১. Demo APK clean build।** Gradle output path, signing, version এবং actual installed application ID record করো।

**২২. NDK Rust target setup।** `cargo-ndk`, NDK, Rust target এবং linker configuration version pin; missing dependency-তে build fail।

**২৩. Native library package validate।** APK contents-এ `lib/arm64-v8a/libandroid_host.so` আছে কি না; ELF AArch64; expected JNI symbols export হয়েছে কি না।

**২৪. Native handshake page।** UI-তে `native library: loaded / missing`, protocol version, backend type এবং last error show করো। Fake data/production data পৃথক label।

**২৫. Display + touch vertical slice।** Rust থেকে deterministic test frame পাঠিয়ে real S25 surface-এ render; tap করলে test counter update। Camera/audio/call before this slice needs not start।

## কাজ ২৬–৩০: OS security ও phone decision

**২৬. `nilrt-launch` sandbox e2e।** Privileged Linux runner-এ namespace/seccomp path execute; unprivileged CI fallback case আলাদা expected status।

**২৭. SELinux load path fix।** Open mode, compiler result, runtime enforce value, process context যাচাই; test fixture দিয়ে wrong policy failure simulate।

**২৮. NilLang launch route।** Installed `.nilax` package `nilrt-launch` through actual VM worker starts, button event changes state; string renderer test যথেষ্ট নয়।

**২৯. Device target ADR।** S25 hosted vs OnePlus/PinePhone native roles, local cost, unlockability and recovery evidence লিখে decision approve। Phone purchase decision stage-gate pass হওয়ার আগে নয়।

**৩০. Flash safety gate।** Generic `flash-device.sh` device identity/profile match না হলে refuse করবে। `userdata` destructive action default disabled; recovery route and image checksum required।

---

# অংশ ২৪ — Troubleshooting playbook

## ২৪.১ Kernel download fails

প্রথমে URL response, network access, local cache path, file size এবং expected SHA-256 দেখো। Download error-এর পরে old cached kernel থাকলে সেটি স্বয়ংক্রিয়ভাবে use করা যাবে কি না policy অনুযায়ী decide করতে হবে; release mode-এ exact pinned checksum ছাড়া use নয়। Manual download allowed হলে source/provenance এবং hash independent channel দিয়ে যাচাই করতে হবে। Browser থেকে download করার পরে file rename করলেই trusted হয় না।

## ২৪.২ “ARM64 binary missing” error

এটি script-এর bug ঢেকে fallback binary নেওয়ার সংকেত নয়, বরং toolchain sequence অসম্পূর্ণ হতে পারে। `rustup target list --installed`, `cargo build --target aarch64-unknown-linux-musl --release --workspace` result, output path এবং ELF machine type যাচাই করো। `target/release/nilinit` x86_64 হলে সেটি ARM64 rootfs-এ কপি করবে না। Cargo workspace crate Android-only target dependency থাকলে Android-host crate আলাদা exclude/build configuration পেতে পারে; Linux AArch64 workspace build ও Android AArch64 `cdylib` এক target নয়।

## ২৪.৩ QEMU black screen

একেবারে প্রথমে serial console দেখো, GUI screenshot নয়। Kernel boot করেছে কি না, console device name সঠিক কি না, `-append`-এ `console=ttyAMA0`/`earlycon` আছে কি না এবং QEMU serial routing duplicate হয়েছে কি না যাচাই। Kernel log থাকলেও screen blank হলে virtual GPU driver/framebuffer path, mode setting, compositor start ও shell entry আলাদাভাবে inspect করো। ARM64 `virt` serial console চলা আর GPU display render হওয়া পৃথক test gate।

## ২৪.৪ QEMU boots but no data after reboot

প্রথমে log-এ `/data` কোন device থেকে mount হয়েছে তা দেখো। `tmpfs` fallback warning থাকলে persistent data expected নয়। এরপর data image-এর filesystem signature, QEMU block device ID, `nilinit` candidate device list, mount type এবং image hash পরীক্ষা করো। Reboot test-এ নতুন image তৈরি করে পুরোনো image accidental replace হয়েছে কি না manifest check করো। Persistent disk path canonical, absolute এবং target-specific হতে হবে।

## ২৪.৫ Core service health fails

“not running” এবং “not ready” আলাদা করো। Binary missing হলে rootfs manifest/build list; spawn fails হলে executable permission/ABI/dynamic libraries; starts then exits হলে service log; process remains but no response হলে IPC bind, socket permissions, protocol mismatch, dependency order ও initialization deadlock দেখো। Health check timeout বাড়িয়ে issue লুকানো যাবে না; service readiness semantics সঠিক করতে হবে।

## ২৪.৬ Android APK opens but native features are dead

`adb logcat` থেকে `NativeBridge` load log, `UnsatisfiedLinkError`, ABI mismatch, missing symbols এবং permission denial আলাদা করো। APK contents inspect করে `.so` আছে কি না দেখো। `android-host` Rust unit test pass মানে Android Dalvik/ART-এর JNI invocation পাস নয়। Native library load না হলে demo mode label দেখাও; feature claims off থাকবে।

## ২৪.৭ Screen draws but touch does nothing

Android host Activity `onTouchEvent` গ্রহণ করছে কি না; callback native function-এ পৌঁছেছে কি না; queue event type correctly mapped কি না; Rust input poll loop চলছে কি না; coordinate mapping; UI hit-test; state update; redraw invalidation—এই ক্রমে trace করো। Event queue-তে event পাওয়া মানে button callback কাজ করা নয়। প্রতিটি boundary-তে trace ID বা counters রাখলে failure location স্পষ্ট হয়।

## ২৪.৮ Camera returns JPEG but no real photo

Rust test fallback constant JPEG ফেরত দিচ্ছে কি না, Android `Camera2` session start হয়েছে কি না, frame queue-তে host-origin timestamp আছে কি না, frame size/device camera ID সঠিক কি না দেখো। Sample JPEG-এর SOI/EOI marker পরীক্ষা camera capture validation নয়। Production mode-এ test fallback disable করো এবং no-frame timeout error দেখাও।

## ২৪.৯ Phone screen/UI claims “VoLTE HD” but call cannot be made

UI label, Rust state machine ও host Android intent আলাদা করো। `Intent.ACTION_DIAL` dialer খোলে, actual call control নয়। Real Android call state query/telecom callback না থাকলে active-call status দেখাবে না। Native Linux phone-এ modem registration, SIM, voice network, audio route এবং IMS/VoLTE carrier support আলাদা capability। Status bar-এর text দিয়ে hardware support প্রমাণ করা যায় না।

## ২৪.১০ Sandbox test fails in CI but works locally

Linux CI runner/container-এর `CAP_SYS_ADMIN`, user namespace, seccomp restrictions ও `unshare` availability পরীক্ষা করো। CI unprivileged fallback চালালে test result security test হিসেবে গণ্য করা যাবে না। Unit test “production setup failure denied” branch test করতে পারে, কিন্তু actual isolated child process test আলাদা needs privileges. Security gate-এর environment preconditions explicit; precondition না থাকলে test `not run`/blocked রিপোর্ট করবে, pass নয়।

---

# অংশ ২৫ — Release engineering, versioning ও public status

## ২৫.১ Version scheme

OnuronOS-এর system image, runtime ABI, NilLang bytecode, `.nilax` package, NilUI widget schema, HAL protocol এবং device profile একই version counter ব্যবহার করতে বাধ্য নয়। এগুলি আলাদা compatibility surface। Versioning policy-তে system version, build revision, runtime ABI version, package format version, protocol version এবং device profile revision পৃথক থাকবে। একটি NilLang app compiler নতুন version ব্যবহার করলেও পুরোনো runtime support করবে কি না তা explicit compatibility table-এ থাকবে।

Example manifest fields:

```json
{
  "product": "OnuronOS",
  "channel": "development",
  "version": "0.2.0-dev.4",
  "source_revision": "<git-sha>",
  "target_id": "qemu-aarch64",
  "kernel_revision": "<source-or-package-revision>",
  "userspace_triple": "aarch64-unknown-linux-musl",
  "nilvm_abi": 1,
  "nilax_format": 1,
  "hal_protocol": 1,
  "release_signing_key_id": "<key-id>",
  "artifacts": {
    "kernel": {"sha256": "...", "size_bytes": 0},
    "initramfs": {"sha256": "...", "size_bytes": 0}
  }
}
```

এই example schema-এর প্রতিটি field final করার আগে source of truth ঠিক করো। `size_bytes` example-এ zero রাখা শুধু schema demonstration; বাস্তব manifest validator zero-size kernel/initramfs reject করবে। Build manifest ও signed release manifest আলাদা হতে পারে: build manifest unsigned local build evidence; release manifest trusted signature-এ bind করা হবে।

## ২৫.২ Release channel

প্রথমে `development` channel যথেষ্ট। `beta`/`stable` channel তখনই তৈরি করো যখন supported device matrix, update signature/rollback, recovery guide ও release notes established হয়েছে। Channel আলাদা file hosting path নয়; channel metadata, signing policy ও promotion gate থাকবে। Stable channel-এ developer debug fallback, fake hardware backend, permissive sandbox, unpinned kernel বা unsigned image allow করা যাবে না।

Release promotion-এর ক্রম:

1. development build passes CI;
2. image reproducibility/checksum verification;
3. target integration tests;
4. security review of changed surfaces;
5. candidate build signed by controlled key;
6. recovery/update rollback smoke test;
7. release notes/known issues;
8. publish artifact and signature;
9. verify download artifact independently;
10. update maturity table and hardware matrix।

## ২৫.৩ SemVer এবং breaking changes

NilLang source syntax, bytecode, HAL IPC, service config schema ও package manifest-এর compatibility policy লিখতে হবে। App runtime API breaking change হলে semver major/versioned API namespace অথবা bytecode ABI bump। Service config schema bump হলে migration/test। Existing `.nilax` packages-এর permission field interpret বদলে গেলে backward compatibility review দরকার। “0.x” version থাকার কারণে arbitrary breaking behavior accept করা যাবে না; installed apps এবং persistent data safe থাকা জরুরি।

## ২৫.৪ Release signing key custody

Release key management-এর জন্য আলাদা doc থাকবে:

- development key বনাম production release key;
- private key generation and offline storage;
- trusted public key fingerprint independent channel-এ publish;
- key access roles/backup/rotation;
- signing operation audit log;
- compromise procedure;
- public key revocation/replacement;
- test key শনাক্ত করার prefix/metadata;
- build server secret scope;
- no key in repo/artifact/log;
- reproducible unsigned build plus controlled signing step।

Custom Python Ed25519 implementation prototype-এ self-contained function হিসাবে আছে, কিন্তু release signing-এর cryptographic library interoperability, RFC test vectors, malformed point/signature tests, independent implementation verification এবং side-channel review ছাড়া production secret key signing-এর জন্য অগ্রাধিকার পাবে না। Trusted, audited crypto library পাওয়া গেলে তার integration/availability ও target requirements যাচাই করো। Custom algorithm claim বা “verified boot secure” বলার আগে independent review প্রয়োজন।

## ২৫.৫ Public status page ও known issues

একটি `docs/status.md` বা repository README section-এ প্রতিটি target-এর বর্তমান অবস্থা থাকবে:

- x86_64 QEMU — CI validated / exact tests;
- ARM64 QEMU — only after dedicated CI boots successfully;
- S25 Android-host — APK build/installed; native bridge status and tested features;
- selected native phone — candidate/bring-up/experimental/supported;
- system services — production vs prototype;
- calls/SMS/camera/audio/network — actual hardware evidence;
- security/verified boot — measured scope and missing pieces;
- known bugs and workaround;
- last validated commit SHA and date।

Status page প্রতিটি commit-এ hand-edit করলে drift হতে পারে। Maturity table থেকে generic status generate করা যায়, কিন্তু hardware evidence-এর current result separate structured file-এ রাখলে ভালো। Documentation claims automatically generated হলেও source evidence human-reviewed হতে হবে।

---

# অংশ ২৬ — Documentation ও research source policy

## ২৬.১ Internal documents একে অপরের সঙ্গে মিলানো

Repository-র architecture ও boot documents stale হলে নতুন contributor ভুল target build করবে। এই plan ধরে অন্তত নিচের documents একে অপরের সঙ্গে reconcile করতে হবে:

- `README.md` — actual product definition ও maturity;
- `docs/reference-board.md` — active reference targets ও exact invocation;
- `docs/hardware-support.md` — HAL layers ও device strategy;
- `docs/s25-hosted-runtime.md` — Android-host architecture, not native boot;
- `docs/boot-process.md` — actual current boot path;
- `docs/improvement-plan.md` — current CI and blockers;
- `docs/completion-checklist.md` — current gates and evidence;
- `docs/security.md` — threat model, permission enforcement, verified-boot caveats;
- `docs/maturity.toml` — source of truth for feature tiers;
- `docs/roadmap.md` / `docs/master-plan.md` — phase order and dependencies;
- `docs/reproducibility.md` — kernel/target/toolchain pinning।

যে document বলছে “ARM64 script নেই” অথচ `build/qemu-aarch64.sh` আছে, সেটি ভুল; যে document বলছে “ARM64 boot validated” অথচ CI test শুধু x86_64 চালায়, সেটিও ভুল। Documentation update-এর লক্ষ্য optimistic বা pessimistic হওয়া নয়; exact evidence-এর সঙ্গে মিল রাখা।

## ২৬.২ Upstream research ব্যবহার

Phone-specific support state দ্রুত বদলাতে পারে; তাই device port শুরু করার সময় official/current install instructions, kernel tree, firmware requirements, issue trackers ও latest release যাচাই করতে হবে। এই plan-এ OnePlus 6T-কে প্রাথমিক candidate হিসেবে উল্লেখের কারণ current Linux community ecosystem ও hardware CI evidence, Onuron compatibility নয়। postmarketOS ২০২৬ সালের v26.06 release page-এ OnePlus 6T device list-এ রয়েছে; January 2026 hardware testing update-এ তাদের real-device runner workflow-এর উল্লেখ আছে। [v26.06 release](https://postmarketos.org/blog/2026/06/21/v26.06-release/), [hardware-testing update](https://postmarketos.org/blog/2026/01/21/hw-ci-mvp/)।

Device acquisition-এর আগে আবার যাচাই করতে হবে: exact codename, device variant, current port category, latest kernel, install guide, working features, known issue, bootloader unlock condition, firmware provenance ও recovery instructions। Community status page-এ “works” লেখা থাকলেও latest release test result ও user report মিলিয়ে দেখো। OnuronOS test matrix-এ upstream support status copy-paste করে নিজের support status বানানো যাবে না।

## ২৬.৩ Source provenance

Kernel, toolchain, firmware, Android build tools এবং third-party Rust packages-এর provenance documented হবে। Version pinning reproducibility বাড়ায় কিন্তু security update delay করতে পারে। তাই pinned dependency update regular PR-এ হবে; vulnerabilities track হবে এবং upgrade test থাকবে। `rustls` advisory fix-এর মতো dependency security update CI regression gate পেরোতে হবে। Dependency lockfile update-এর সঙ্গে changed version ও security reason record করতে হবে।

## ২৬.৪ License compliance

Kernel sources/patches, firmware, fonts, icons, Android libraries, Alap/NilLang components এবং generated assets-এর license inventory রাখতে হবে। GPL/LGPL/MIT/BSD/Apache requirements অনুযায়ী source availability/notice রাখা হবে। Proprietary camera/modem firmware redistributable না হলে repository-তে copy করা যাবে না। App marketplace/packager third-party license metadata preserve করবে। Build scripts vendor binary pull করলে license/provenance note ছাড়া release artifact তৈরি হবে না।

---

# অংশ ২৭ — Glossary: গুরুত্বপূর্ণ ধারণা এক জায়গায়

**ARM64/AArch64:** 64-bit ARM instruction-set architecture. ফোনে বহুল ব্যবহৃত, কিন্তু সব ARM64 device একই boot chain/driver ব্যবহার করে না।

**`aarch64-unknown-linux-musl`:** Rust target triple, Linux ARM64 userspace-এর জন্য musl libc build। Android native library target `aarch64-linux-android`-এর সঙ্গে এক নয়।

**`arm64-v8a`:** Android NDK/APK native ABI label; সাধারণত AArch64 ARMv8-compatible native library packaging বোঝায়।

**QEMU `virt`:** generic virtual machine platform; hardware drivers ও kernel boot architecture পরীক্ষা করতে কাজে লাগে, বাস্তব phone-specific devices emulation নয়।

**Kernel:** OS-এর core যা process, memory, scheduler, device driver, syscall, filesystems ও security enforcement-এর অনেক অংশ পরিচালনা করে।

**Initramfs:** kernel start হওয়ার পর প্রথম userspace root filesystem, যেখানে `/init`/PID 1 শুরু হয়।

**PID 1 / `nilinit`:** প্রথম userspace process, service startup/supervision ও shutdown-এর কেন্দ্র। PID 1-কে boot marker print করার চেয়ে বেশি দায়িত্ব নিতে হয়।

**Rootfs:** OS-এর directory tree ও executable/config layout। Initramfs rootfs এবং persistent system/rootfs image পৃথক হতে পারে।

**Device tree (DTB/DTBO):** kernel-কে hardware topology, interrupt, memory ও device configuration জানায়; ফোনভেদে গুরুত্বপূর্ণ।

**Bootloader:** device-এর early boot stage যা kernel/image load করে এবং কিছু ক্ষেত্রে signature/slot verification করে।

**AVB / verified boot:** boot chain-এ trusted integrity/signature verification framework; custom signed text file নিজে থেকে standard AVB নয়।

**A/B update:** দুটি system slot ব্যবহার করে update apply করে; inactive slot verify, boot attempts ও rollback metadata দরকার।

**HAL / NilHAL:** common interface যা system service-কে target-specific hardware/backend থেকে পৃথক করে। Trait থাকা বাস্তব driver চালু থাকার প্রমাণ নয়।

**Backend:** HAL interface-এর নির্দিষ্ট implementation, যেমন Linux native, Android-host, QEMU virtual বা fake test backend।

**JNI:** Java/ART ও native library-র মধ্যে bridge interface। Native symbol, type signature ও memory lifetime মিলতে হবে।

**NDK:** Android native code development toolchain।

**ELF machine type:** executable/library কোন architecture-এর তা বোঝায়; target rootfs-এ wrong architecture executable ঢোকা আটকাতে যাচাই করা হয়।

**Namespace:** process/resource isolation features in Linux kernel. Namespace alone full security sandbox নয়।

**Seccomp:** syscall filtering; architecture অনুযায়ী syscall number/policy পরীক্ষা করতে হয়।

**SELinux:** Mandatory Access Control framework; policy compiler, loader, mode ও process labels বাস্তবে কার্যকর হতে হবে।

**UID/GID:** Linux user/group IDs; per-app UID DAC file/process isolation-এ সাহায্য করে, তবে allocator registry atomic ও policy-safe হতে হবে।

**Permission broker:** app manifest request এবং user-approved capability-কে real privileged operation-এর সঙ্গে bind করে। Manifest permission লেখা alone enforcement নয়।

**NilLang bytecode:** VM/interpreter-এ চালানোর portable package representation; native CPU machine code বলে ধরে নেওয়া যাবে না।

**`.nilax`:** signed NilLang app package container; manifest, bytecode/resources, integrity and signature policy থাকতে পারে।

**NilVM:** NilLang program load/execute/state management environment। বর্তমান scene-string output actual graphics rendering-এর সমান নয়।

**NilUI:** UI element/layout/rendering system। VM scene, compositor এবং input events-এর সঙ্গে integration প্রমাণ করতে হবে।

**Alap:** প্রকল্পে cross-platform framework হিসেবে ঘোষিত ecosystem অংশ; repo-তে actual implementation boundary/entry point স্পষ্ট ও testable করতে হবে।

**Readiness:** service/process শুধু alive নয়; তার API/socket/protocol কাজ করার জন্য প্রস্তুত।

**Maturity:** নির্দিষ্ট environment-এ বাস্তব evidence অনুযায়ী feature status।

**Recovery:** broken image/update থেকে safe known-good state-এ ফেরার পদ্ধতি।

**Hosted runtime:** Android/iOS host-এর ভিতরে app/framework হিসেবে চালানো environment; host OS-কে replace করে না।

**Native port:** device boot chain দিয়ে Onuron kernel/userspace boot করানো এবং device-specific hardware enablement।

---

# অংশ ২৮ — Final acceptance gate: কোন অবস্থায় কী দাবি করা যাবে

## ২৮.১ Claim level 1 — “Desktop/QEMU prototype boots”

এই দাবি করার ন্যূনতম evidence: exact target architecture ও kernel version record করা, clean build, QEMU serial boot log, `nilinit` PID 1, essential mounts, core services readiness, error-handling test এবং persistent-data test (যদি persistence বলা হয়)। x86_64 QEMU-এর green CI শুধু x86_64 profile-কে এই claim দিতে পারে। ARM64 script থাকা যথেষ্ট নয়; ARM64 QEMU test নিজে pass করতে হবে।

## ২৮.২ Claim level 2 — “ARM64 QEMU prototype works”

Required evidence: ARM64 binaries, kernel hash, initramfs hash, manifest target ID, QEMU `virt` invocation, service health, persistent data write/reboot/read এবং failure cases. Log-এ `/data` temporary `tmpfs`-এ fallback করেছে কি না স্পষ্ট হতে হবে। ভুল-architecture binary থাকলে image build fail করেছে এমন negative test থাকতে হবে। ARM64 QEMU pass করলেও এটি physical phone support নয়।

## ২৮.৩ Claim level 3 — “Onuron hosted runtime runs on Galaxy S25”

Required evidence: APK source build, installed APK version/hash, Android version/device model metadata, native library status, actual display/input test, actual battery/connectivity sources, permission behavior এবং supported hardware operations-এর list। যে operations শুধু dialer intent বা simulated data ব্যবহার করে, সে operations native hardware control হিসেবে গণ্য হবে না। About screen hosted mode বলে; Settings-এ backend status accurate।

## ২৮.৪ Claim level 4 — “OnuronOS boots on device X”

Required evidence: exact model variant/codename, bootloader state, kernel/device tree/firmware provenance, native image manifest, boot logs, `nilinit` and service readiness, persistent storage, recovery route এবং repeated boot tests. Device image flash করা হয়েছে কিন্তু screen/UI আসে না—তবুও early boot-to-init claim হতে পারে, কিন্তু full mobile OS claim নয়।

## ২৮.৫ Claim level 5 — “Device X is supported experimentally”

Required evidence: reproducible install guide, verified artifact, recovery/restore instructions, display/touch, storage, power/charging, networking capability status, app lifecycle, known limitations, update/recovery state এবং hardware matrix। Calls/camera/audio/suspend না চললে সেগুলিকে `partial`, `unsupported` বা `not validated` বলা হবে। Experimental label ব্যবহারকারীর daily-driver expectation তৈরি না করে।

## ২৮.৬ Claim level 6 — “Device X is supported”

এটি সবচেয়ে কঠিন দাবি। ন্যূনতম repeated boot, suspend/resume, storage durability, network reconnection, audio route, camera (যদি supported বলে), telephony (যদি supported বলে), update rollback, recovery drill, security model, battery/thermal stress, critical crash recovery, accessible UI, known issue list এবং exact release artifact verification চাই। Support claim feature-by-feature হবে; একটি model-এর support মানে অন্য model বা variant supported নয়।

## ২৮.৭ “Production-ready OS” claim

বর্তমান অবস্থায় এই দাবি করা যাবে না। Production readiness-এর জন্য architecture/security review, independent code review, privacy policy, update infrastructure, release key management, long-term kernel/firmware support policy, end-user installation/recovery, accessibility, user data migration, app compatibility contract, support process, incident response এবং real hardware test evidence একত্রে দরকার। Production readiness কোনও এক CI run, একটা সুন্দর UI, বা একটি device-এ একবার boot হওয়া দিয়ে অর্জিত হয় না।

---

# অংশ ২৯ — এখন কী করবে, কী করবে না

## ২৯.১ এখনই করো

1. `868fa19`-এর পরের current `main` fetch করে local checkout sync করো; কাজ শুরুর সময় branch SHA note করো।
2. `android-host` tests parallel mode-এ repeat করো এবং result stable কি না নিশ্চিত করো।
3. `.gitignore` change-এর পর tracked `.gradle`, APK এবং build output আসলে index থেকে বাদ পড়েছে কি না যাচাই করো।
4. `build/mkinitramfs.py`-এর target binary fallback ও kernel SHA mismatch behavior fix করো।
5. ARM64 QEMU launcher `--no-rebuild` semantics ঠিক করো এবং formatted disk image তৈরি করো।
6. First ARM64 manual run-এ output log save করে `docs/evidence/qemu-aarch64/`-এ evidence রাখো।
7. ARM64 CI matrix যোগ করো।
8. Rootfs/boot script/README/docs-এর target status একত্রে update করো।
9. Android NDK build-কে clean Gradle/CI path-এর সঙ্গে যুক্ত করো।
10. OnePlus/PinePhone native target selection এখনই purchase না করে read-only research/scorecard দিয়ে শুরু করো।

## ২৯.২ আপাতত করো না

- Current generic `flash-device.sh` দিয়ে S25 বা অন্য ফোনে flash নয়;
- `/data` format/erase command default-on নয়;
- embedded public key-কে trust root বলা নয়;
- custom text vbmeta-কে standard Android AVB দাবি করা নয়;
- SELinux enforcing status log-only string দিয়ে দেখানো নয়;
- `Ok(())` return করা backend-কে real hardware support ঘোষণা নয়;
- sample JPEG/zero audio/hard-coded Wi-Fi/VoLTE demo-কে genuine feature বলা নয়;
- NilLang bytecode-কে native CPU executable বলা নয়;
- QEMU ARM64 script যোগ হওয়াকে ARM64 QEMU validated বলা নয়;
- hosted Android APK-কে native Onuron kernel boot বলা নয়;
- নতুন ফোন কেনা বা daily-use switch, recovery ও hardware gates complete হওয়ার আগে নয়;
- একসঙ্গে একাধিক native phone target maintain করা নয়;
- all-device support-এর claim নয়।

## ২৯.৩ যদি শুধু একটি সপ্তাহ সময় থাকে

এই সপ্তাহে scope narrow রাখো:

**দিন/পর্ব ১:** current branch sync, tests rerun, tracked artifacts inspect।

**পর্ব ২:** architecture check ও kernel hash enforcement tests।

**পর্ব ৩:** ARM64 `data.img` filesystem তৈরি ও mount/persistence test।

**পর্ব ৪:** `--no-rebuild` flag bug fix, script unit tests।

**পর্ব ৫:** Linux/QEMU CI job-এর clone-এ ARM64 build/boot step যোগ করে initial run।

**পর্ব ৬:** failure log অনুযায়ী targeted fixes; ARM64 binary missing/incorrect target হলে নতুন feature add বন্ধ।

**পর্ব ৭:** README/Reference Board/Completion Checklist update এবং evidence commit।

এটি কঠোর calendar guarantee নয়, কাজের prioritization example। Toolchain setup বা cross compilation সমস্যা এলে সেই সমস্যাটিকেই milestone হিসেবে ধরো; জোর করে পরের ফিচারে এগিও না।

## ২৯.৪ যদি দুই মাস সময় থাকে

দুই মাসে realistically focus হবে build reproducibility, ARM64 QEMU, Android hosted display/input slice, security negative tests এবং device selection dossier। সম্পূর্ণ native phone OS with camera/VoLTE/secure boot production-ready হয়ে যাবে ধরে পরিকল্পনা কোরো না। Hardware bring-up community kernel dependency, firmware, bootloader, device trees এবং physical test access-এর ওপর নির্ভর করবে। Milestone completion ও actual evidence release date-এর চেয়ে গুরুত্বপূর্ণ।

---

# অংশ ৩০ — Summary: প্রকল্পের success definition

OnuronOS-এর দীর্ঘমেয়াদি success হবে **একটি core OS এবং language/runtime-এর চারপাশে device-specific, tested, maintainable backend তৈরি করা**। এক ডিভাইসের জন্য আলাদা করে পুরো OS লেখা নয়; আবার শুধু common trait/interface থাকলেই universal support claim নয়। Shared core এবং per-device port একসঙ্গে দরকার।

এখনকার সর্বোত্তম sequence:

1. Current CI এবং build scripts trustworthy;
2. ARM64 QEMU reproducible ও persistent-data validated;
3. S25 hosted runtime-এর UI↔Rust JNI path actual device-এ working;
4. NilLang signed package real launcher/sandbox/NilUI lifecycle-এ execute;
5. SELinux, permissions, seccomp, signing ও update/recovery verified;
6. একটি native reference phone select and profile;
7. native boot-to-init;
8. storage → display/touch → power → network → audio → camera/telephony;
9. recovery, OTA rollback, long-run tests;
10. documented experimental release with feature-by-feature support evidence।

ഈ পরিকল্পনার মূল নীতি হলো **যে feature এখন সত্যি কাজ করছে না, সেটিকে জোর করে কাজ করছে বলা হবে না; প্রতিটি ধাপের observable test থাকবে।** এতে UI-র সুন্দর demo ও বাস্তব mobile OS-এর engineering gap আলাদা থাকবে, এবং নতুন ফোন support যোগ করার সময় core architecture পুনর্লিখন না করে device profile ও backend উন্নত করা সম্ভব হবে।

---

# উৎস ও সম্পর্কিত নথি

## OnuronOS repository sources

- Main repository: https://github.com/joysriramsarkar/onuronOS
- Latest reviewed commit: https://github.com/joysriramsarkar/onuronOS/commit/868fa19b9cdf723702a8e7ecf4a8e19ae7ca811a
- Latest Linux/QEMU workflow: https://github.com/joysriramsarkar/onuronOS/actions/runs/37941913405
- `build/mkinitramfs.py`: https://github.com/joysriramsarkar/onuronOS/blob/main/build/mkinitramfs.py
- ARM64 QEMU Bash launcher: https://github.com/joysriramsarkar/onuronOS/blob/main/build/qemu-aarch64.sh
- ARM64 QEMU PowerShell launcher: https://github.com/joysriramsarkar/onuronOS/blob/main/build/qemu-aarch64.ps1
- Android NDK Rust build: https://github.com/joysriramsarkar/onuronOS/blob/main/android-host/build-ndk.sh
- Android-host native bridge: https://github.com/joysriramsarkar/onuronOS/blob/main/android-host/src/jni_bridge.rs
- Android-host activity: https://github.com/joysriramsarkar/onuronOS/blob/main/android-host/app/src/main/java/org/onuron/mobile/MainActivity.java
- Android bridge service: https://github.com/joysriramsarkar/onuronOS/blob/main/android-host/app/src/main/java/org/onuron/mobile/OnuronBridgeService.java
- NilHAL Linux backend: https://github.com/joysriramsarkar/onuronOS/blob/main/runtime/nilhal/src/backends/linux.rs
- NilLang vertical slice test: https://github.com/joysriramsarkar/onuronOS/blob/main/runtime/nillang/tests/native_app_slice.rs
- Reference board: https://github.com/joysriramsarkar/onuronOS/blob/main/docs/reference-board.md
- Hardware support architecture: https://github.com/joysriramsarkar/onuronOS/blob/main/docs/hardware-support.md
- Security documentation: https://github.com/joysriramsarkar/onuronOS/blob/main/docs/security.md
- Maturity source of truth: https://github.com/joysriramsarkar/onuronOS/blob/main/docs/maturity.toml

## External device-port research

- postmarketOS v26.06 release (device list and status categories): https://postmarketos.org/blog/2026/06/21/v26.06-release/
- postmarketOS hardware-testing automation update (OnePlus 6T hardware runner): https://postmarketos.org/blog/2026/01/21/hw-ci-mvp/

**Final note:** All command examples and directory layouts not already present in the repository are proposals. Before treating them as implemented, create the code, tests and evidence described in the acceptance criteria.
# অংশ ৩১ — Code review policy ও contributor checklist

## ৩১.১ Review-এ কী দেখা হবে

OnuronOS-এ একটি change review করার সময় শুধু compile success অথবা UI screenshot দেখলে চলবে না। Reviewer-কে প্রথমে target খুঁজতে হবে: code কোন environment-এ চলে, privilege level কী, data কোথায় persist হয়, error এলে কী হয়, test কোন বাস্তব layer exercise করে এবং maturity claim কোন evidence সমর্থন করে। HAL trait পরিবর্তন করলে Linux, Android-host, QEMU/fake backend সব compile/contract test পাস কি না দেখতে হবে। Build script বদলালে clean output path এবং stale artifact behavior পরীক্ষা করতে হবে। Security code বদলালে negative tests অপরিহার্য।

Review checklist:

- target/architecture explicitly specified;
- correct binary and dependency target;
- no host path/secret baked into artifact;
- errors propagate instead of ignored;
- no fake-success return on unavailable hardware;
- permissions and trust boundaries identified;
- input size/time/resource limits;
- state persistence atomicity;
- process/FD/thread cleanup;
- logs are useful but redact secrets;
- tests cover normal path and failure path;
- docs/maturity table update;
- build artifacts not tracked;
- no unreviewed `unsafe` or shell command concatenation;
- release artifact signature/target information accurate।

## ৩১.২ Unsafe Rust ও native boundary

`unsafe` ব্যবহার করলে reviewer-কে safety invariant comment, null/length checks, pointer lifetime, aliasing rules, thread safety এবং error behavior দেখতে হবে। JNI exports, raw syscalls, kernel-facing structures, FFI and frame buffers এই repository-র সবচেয়ে risky boundary-এর মধ্যে পড়ে। `#![allow(clippy::missing_safety_doc)]` global allow থাকলে production crate-এ প্রতিটি unsafe public function-এর safety comment আলাদাভাবে নিশ্চিত করা দরকার। Native ABI mismatch অনেক সময় host unit test-এ ধরা পড়ে না; তাই Android instrumentation/native integration test আলাদা gate।

## ৩১.৩ Build script-এ shell safety

Build scripts-এ user-provided target path, device identifier, image path, partition name এবং command arguments সরাসরি shell string-এ interpolate করলে injection/accidental destructive command ঝুঁকি তৈরি হয়। Bash array, explicit argument parsing এবং canonicalized paths ব্যবহার করো। Flash script-এ target profile থেকে অনুমোদিত partition list এসেছে কি না validate করো; arbitrary text input-কে partition name হিসেবে ব্যবহার করবে না। Destructive command আগে পুরো plan print করবে এবং user-এর device identity পুনরায় check করবে।

## ৩১.৪ State machine testability

Service lifecycle, telephony state, OTA state, permission grant, camera session, audio route এবং recovery decision সবই stateful system। State transition-কে pure function/explicit transition table-এ model করলে host tests-এ corner case সহজে ধরা যায়। কিন্তু pure transition test actual side effect proof নয়; এর পরে integration test real process, file, socket, Android API বা hardware backend exercise করবে। State machine invalid transition reject করবে; silently default state-এ ফিরবে না।

## ৩১.৫ Review-এর বাইরে থাকা দাবি

README-তে “production-grade”, “100% Rust”, “verified boot”, “real camera”, “VoLTE”, “all modern phones”, “Android compatibility” জাতীয় বড় claim থাকলে reviewer concrete implementation path চাবে। Java host component থাকা অবস্থায় userspace-কে “100% Rust” বললে সেটা platform-specific wording দিয়ে qualify করতে হবে। Container placeholder থাকলে Android app compatibility supported নয়। Signed metadata থাকলেও bootloader-enforced root of trust না থাকলে verified boot production-ready নয়। Reviewer-এর কাজ wording ভালো করা নয়; claim এবং evidence এক হওয়া নিশ্চিত করা।

# অংশ ३२ — අවසාන next-step decision tree

যখন পরের কাজ বেছে নিতে হবে, নিচের ক্রমে সিদ্ধান্ত নাও।

**প্রশ্ন ১: current main-এ সব required workflow green?** না হলে failing SHA ও job isolate করে fix; feature work pause। হ্যাঁ হলে পরের প্রশ্ন।

**প্রশ্ন ২: clean checkout থেকে ARM64 target binary build হয়?** না হলে toolchain/architecture/host-fallback problem ঠিক করো। হ্যাঁ হলে পরের প্রশ্ন।

**প্রশ্ন ৩: ARM64 initramfs correct architecture, verified kernel এবং valid `/data` filesystem নেয়?** না হলে builder/manifest/disk formatter fix। হ্যাঁ হলে QEMU boot চালাও।

**প্রশ্ন ৪: ARM64 QEMU core services ready এবং persistent write/reboot/read পাস?** না হলে boot, service readiness, storage fix। হ্যাঁ হলে ARM64 QEMU milestone documented।

**প্রশ্ন ৫: Android-host APK Rust `.so` package/load করে?** না হলে NDK/Gradle/JNI build path; camera/telephony feature work নয়। হ্যাঁ হলে real display/touch end-to-end।

**প্রশ্ন ৬: Native Hello app trusted install থেকে sandbox runtime-এ চলে এবং UI event respond করে?** না হলে NilLang/launcher/NilUI boundary ঠিক করো। হ্যাঁ হলে additional language/framework features।

**প্রশ্ন ৭: target phone unlockable, exact profile known এবং recovery tested?** না হলে native flash নয়। হ্যাঁ হলে console-only boot image ও `nilinit` startup; direct full replacement install নয়।

**প্রশ্ন ৮: storage, display/touch, power, network এবং recovery gates pass?** না হলে unsupported capabilities স্পষ্ট করে রাখো। হ্যাঁ হলে audio/camera/telephony capability-র next layer।

**প্রশ্ন ৯: verified update/rollback, long-run test এবং feature matrix evidence আছে?** না হলে experimental label। হ্যাঁ হলে নির্দিষ্ট device/feature subset-কে documented support status দেওয়া যেতে পারে।

এই decision tree-র উদ্দেশ্য engineering কাজের স্বাধীনতা কমানো নয়; বরং সবচেয়ে risk-reducing dependency আগে সম্পন্ন করা। নতুন feature-এর আনন্দের জন্য foundational build/safety debt জমতে দিলে পরের hardware port আরও কঠিন হবে।

# অংশ ৩৩ — App compatibility এবং API stability

NilLang ecosystem বাড়াতে গেলে app developer-রা যে API-র উপর ভরসা করবে তা স্থিতিশীল করতে হবে। শুরুতে supported API surface ছোট রাখো: layout, text, buttons, app state, app lifecycle, local storage, permission query, notification request এবং approved host-service calls। Experimental API-তে `experimental` namespace বা feature flag থাকবে। API change হলে changelog, deprecation period, compiler diagnostic এবং compatibility test দিতে হবে। একটি package target version specify করবে যাতে newer runtime পুরোনো package-এর behavior হঠাৎ বদলে না দেয়।

Cross-platform behavior-এর মানে প্রতিটি platform-এ সব feature একইভাবে available—এমন নয়। App API একটি operation support করে, কিন্তু target backend সেটি না পারলে `Unsupported` capability result দিতে হবে। Developer যেন capability query করে alternative UI দেখাতে পারে। উদাহরণস্বরূপ, native Linux port-এ camera নেই কিন্তু hosted S25-এ Camera2 available; একই app camera capability query করে capture option দেখাবে। Application code phone model name hard-code করে backend detect করবে না; platform capability interface ব্যবহার করবে।

App compatibility test suite-এ পুরোনো `.nilax` package newer runtime-এ launch, unknown optional UI property, unsupported permission, missing backend, state persistence schema migration এবং app update rollback অন্তর্ভুক্ত হবে। Runtime ABI break করতে হলে package manifest compatibility range যাচাই করবে। App developer-এর error message-এ exact API/target mismatch থাকবে—শুধু “App failed” নয়। এভাবে NilLang + Alap ecosystem device abstraction-এর আসল সুবিধা পাবে: app logic shared, hardware operation capabilities এবং implementation target-specific।

# অংশ ৩৪ — প্রতিদিনের engineering workflow

প্রতিদিন কাজের আগে `git status --short`, `git log -1 --oneline` এবং বর্তমান branch যাচাই করো। কাজের শুরুতে issue-তে নির্দিষ্ট acceptance criteria লিখে নাও, যাতে code change শেষে কী প্রমাণ করতে হবে তা পরিষ্কার থাকে। একটি PR-এর মধ্যে target, builder, runtime এবং UI-র একাধিক স্তর একসঙ্গে পাল্টাতে হলে তা ভাগ করা যায় কি না ভেবে দেখো। Implementation-এর পরে প্রথমে সংশ্লিষ্ট ছোট test, তারপর crate/workspace tests, এরপর target build এবং শেষে integration test চালাও। Test ব্যর্থ হলে error log সংরক্ষণ করো; ব্যর্থতার cause না জেনে retry করে সবুজ output পাওয়াকে fix মনে কোরো না।

দিনের শেষে নতুন known issue, implemented behavior, missing backend এবং next dependency নথিবদ্ধ করো। README বা progress note-এ “camera implemented” না লিখে “Camera2 request dispatch implemented; real frame delivery not validated” লিখলে পরবর্তী কাজ পরিষ্কার হবে। একই নিয়ম boot, storage, network, telephony এবং security feature-এ প্রযোজ্য। Test environment, commit SHA ও artifact hash ছাড়া screenshot বা console snippet বিচ্ছিন্নভাবে সংরক্ষণ করলে পরে কোন code-এ test চলেছিল তা বোঝা যাবে না।

সপ্তাহের শেষে milestone review-এ তিনটি ফল দাও: কী বাস্তবে কাজ করছে, কী এখনও অসম্পূর্ণ, এবং কোন blocker পরবর্তী সপ্তাহের পরিকল্পনা বদলাতে পারে। নতুন feature জমা হওয়া progress-এর মাপকাঠি নয়; reliable tests, reproducible build, truthful feature status এবং recovery-সহ একটি কার্যকর target তৈরি হওয়াই progress-এর মাপকাঠি।
