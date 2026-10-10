# অনুরণ ওএস (OnuronOS): উন্নতির পূর্ণাঙ্গ, প্রমাণভিত্তিক ৪০,০০০-শব্দের রূপরেখা

**দলিলের ধরন:** প্রকৌশল-পরিকল্পনা, অগ্রাধিকার তালিকা, বাস্তবায়ন-ধাপ ও গ্রহণযোগ্যতা-পরীক্ষা  
**ভাষা:** বাংলা  
**অডিটের সময়:** ১০ অক্টোবর ২০২৬, প্রায় ১০:১৪ IST-এ প্রাপ্ত সর্বশেষ পাবলিক অবস্থা  
**রিপোজিটরি:** <https://github.com/joysriramsarkar/onuronOS>  
**অডিট-করা `main` কমিট:** [`be64e988a9a2a9658b300496f3fb601dd4aeb798`](https://github.com/joysriramsarkar/onuronOS/commit/be64e988a9a2a9658b300496f3fb601dd4aeb798)  
**এর ঠিক আগের বড় ইন্টিগ্রেশন কমিট:** [`5108c44a4fcf6fe05838251c10e78f74f4606526`](https://github.com/joysriramsarkar/onuronOS/commit/5108c44a4fcf6fe05838251c10e78f74f4606526)

> **গুরুত্বপূর্ণ সতর্কতা:** এই দলিল একটি উন্নয়ন-পরিকল্পনা, সফলতার দাবি নয়। রিপোজিটরির কোড, নথি, সর্বশেষ কমিট এবং CI-তে দৃশ্যমান প্রমাণকে বর্তমান অবস্থা হিসেবে ধরা হয়েছে। যেখানে প্রোডাকশন-গ্রেড হার্ডওয়্যার পরীক্ষা নেই, সেখানে সেটিকে “যাচাই হয়নি”, “পরিকল্পিত” বা “প্রোটোটাইপ” বলা হয়েছে। নথিতে কোনো ফিচারের নাম থাকা মানেই ফিচারটি বাস্তবে কাজ করে—এমন ধরে নেওয়া হয়নি।

---

## সূচিপত্র

1. দলিলের লক্ষ্য, সীমা ও ব্যবহার-পদ্ধতি
2. বর্তমান অবস্থার সংক্ষিপ্ত অডিট
3. পণ্যের লক্ষ্য ও আর্কিটেকচারের ভিত্তি
4. অগ্রাধিকার কাঠামো এবং কাজের নিয়ম
5. প্রথম ৭২ ঘণ্টা: তাত্ক্ষণিক স্থিতিশীলতা
6. P0-1: NilHAL-এর ARM64 compilation ত্রুটি
7. P0-2: বিল্ড টার্গেট, নামকরণ ও আউটপুট ডিরেক্টরির একীকরণ
8. kernel উৎস, checksum pinning এবং reproducible build
9. initramfs ও root filesystem নির্মাণ
10. persistence: ext4 disk, mount failure এবং reboot test
11. QEMU x86_64-এর regression gate
12. QEMU ARM64-এর পূর্ণ bring-up gate
13. nilinit, supervision এবং সত্যিকারের service readiness
14. early boot, `/proc`, `/sys`, `/dev`, cgroups ও SELinux
15. `nilrt`: app isolation, UID/GID, permissions ও seccomp
16. `nilpkg` ও `.nilax`: package trust, update এবং rollback
17. NilLang compiler, bytecode ও VM-এর স্থিতিশীল contract
18. Alap framework-কে নকশা থেকে বাস্তব implementation-এ আনা
19. NilUI এবং NilLang → UI → compositor pipeline
20. DRM/KMS renderer ও frame-presentation correctness
21. Input, gesture, focus ও accessibility pipeline
22. Samsung S25 hosted runtime: APK/NDK/Graal নয়, বাস্তব JNI contract
23. Android host lifecycle, permissions এবং privacy
24. Android display: Rust frame buffer সত্যিই পর্দায় দেখানো
25. Camera pipeline: simulated frame বনাম real capture
26. Audio pipeline: PCM, AudioTrack/AudioRecord ও focus
27. Telephony, SMS, SIM, AT command এবং modem semantics
28. Network, Wi-Fi, DNS, Bluetooth ও system telemetry
29. Native phone port: OnePlus 6T `fajita`-র প্রস্তুতি
30. Device profile, DTB, kernel config, partition map ও recovery
31. Flashing safety, verified boot, AVB ও root of trust
32. A/B OTA, update transaction এবং rollback
33. Shell, launcher, Settings এবং demo data-এর সততা
34. System services, IPC, observability এবং debugging
35. Security architecture: threat model, defense in depth ও release gate
36. Performance, power, resource budget এবং reliability
37. বাংলা ভাষা, accessibility, localisation এবং ব্যবহারযোগ্যতা
38. SoftBus ও Android compatibility-এর বাস্তব সীমা
39. CI/CD, automated test matrix এবং artefact provenance
40. প্রকল্প-পরিচালনা, PR policy, issue discipline এবং documentation
41. ৩০/৬০/৯০ দিনের বাস্তব কর্মপরিকল্পনা
42. ছয় মাস ও বারো মাসের milestone plan
43. P0/P1/P2/P3 ticket backlog ও dependency graph
44. Acceptance criteria: কোন প্রমাণে কোন কাজ “সম্পন্ন” বলা যাবে
45. release readiness checklist ও stop-ship শর্ত
46. ঝুঁকি-নিবন্ধন এবং ব্যর্থতার প্রতিক্রিয়া
47. কোড-স্তরের উদাহরণ ও প্রস্তাবিত test commands
48. সর্বোচ্চ-ফলদায়ক কাজের ক্রম: এখন কী করা উচিত
49. সংযোজনী: evidence template, issue template ও definition of done
50. শেষ সিদ্ধান্ত: কোন অবস্থায় OnuronOS-কে কোন নামে পরিচয় দেওয়া যাবে

---

# ১. দলিলের লক্ষ্য, সীমা ও ব্যবহার-পদ্ধতি

এই রূপরেখার উদ্দেশ্য অনুরণ ওএসকে শুধু বড় README, আকর্ষণীয় UI বা অনেকগুলি ডেমন-নামের প্রকল্প হিসেবে না রেখে, ক্রমশ একটি পরীক্ষাযোগ্য, reproducible, নিরাপদ এবং বাস্তব হার্ডওয়্যারে পোর্ট করার উপযুক্ত অপারেটিং সিস্টেমে পরিণত করা। প্রকল্পের লক্ষ্য উচ্চাভিলাষী: Linux LTS kernel-এর উপর Rust-ভিত্তিক userspace, `nilinit` হিসেবে PID 1, একীভূত `NilHAL`, `nillang`/`nilc` toolchain, `.nilax` package ecosystem, `nilrt` application isolation, `nilui`/`nilui-gpu` UI pipeline, এবং `Alap` framework। একই সঙ্গে প্রকল্পটি Samsung Galaxy S25-এর মতো Android ডিভাইসে hosted application mode-ও পরীক্ষা করতে চাইছে। এই দুই লক্ষ্য পরস্পর সম্পর্কিত হলেও এক নয়; তাই উন্নতির রূপরেখায় শুরু থেকেই দুটি আলাদা track রাখা হয়েছে।

এই দলিল কোনো এক-ধাপের “সব ঠিক করে ফেলো” নির্দেশনা নয়। একটি OS পোর্টে build pipeline, image packaging, kernel compatibility, userspace, init/supervisor, storage, UI, input, power, modem, camera, audio, permission system, verified boot এবং emergency recovery—প্রতিটি অংশ একে অন্যের উপর নির্ভরশীল। নিচের পরিকল্পনায় তাই dependency-first ক্রম অনুসরণ করা হয়েছে: আগে build-এর সত্যতা, তারপর virtual-machine boot, পরে service readiness ও persistence, তার পরে runtime/UI integration, তারপর hosted Android bridge, এবং সবশেষে physical phone bring-up। এই ক্রম বদলে সরাসরি flash করতে গেলে debug cycle কঠিন হবে এবং ডিভাইস নষ্ট হওয়ার ঝুঁকি তৈরি হবে।

## ১.১ কী এই রূপরেখার মধ্যে আছে

- ১০ অক্টোবর ২০২৬-এ দৃশ্যমান public `main`-এর বাস্তব অবস্থা, নির্দিষ্ট commit ও CI ফলাফলের ভিত্তিতে অগ্রাধিকার নির্ধারণ।
- Rust, Python, shell build tools এবং Android Java/JNI-র মধ্যকার সীমানা পরিষ্কার করা।
- x86_64 এবং AArch64 QEMU-র পৃথক build, test, boot ও storage validation gate।
- Samsung S25-এ APK হিসেবে চলা OnuronOS hosted shell-কে native OS boot থেকে আলাদা রাখা।
- OnePlus 6T `fajita`-কে সম্ভাব্য প্রথম native reference device হিসেবে নেওয়ার আগে সঠিক প্রস্তুতি ও safety gate।
- feature status, maturity table এবং evidence folder-কে code-এর সঙ্গে সমন্বয় করা।
- প্রতিটি বড় কাজের জন্য acceptance test, failure signal, rollback এবং release-blocking condition নির্ধারণ।

## ১.২ কী এই দলিলের মধ্যে নেই

এই রূপরেখা কোনো নির্দিষ্ট ফোনে flash করার অনুমতি নয়; কোনো boot image, `vbmeta`, partition layout বা device tree-কে verified বলে ঘোষণা করে না; Samsung retail S25-এ bootloader unlock বা native boot সম্ভব—এমন দাবি করে না; OnePlus 6T-তে mainline Linux support থাকলেই OnuronOS চালু হবে—এমন ধরে নেয় না; এবং Android application-এর ভিতরে shell দেখা গেলে সেটিকে Android-কে প্রতিস্থাপনকারী standalone OS বলে না। বাস্তব হার্ডওয়্যার, modem, camera sensor, audio codec, GPU এবং suspend/resume আচরণ সম্পর্কে সিদ্ধান্ত নিতে reproducible hardware logs প্রয়োজন।

## ১.৩ তথ্যের তিনটি স্তর

এই দলিল পড়ার সময় প্রতিটি বক্তব্যকে তিনটি শ্রেণির একটিতে ভাবতে হবে।

**যাচাইকৃত বর্তমান তথ্য** হলো রিপোজিটরিতে দৃশ্যমান ফাইল/কোড বা নির্দিষ্ট CI run-এর ফলাফল। যেমন, সর্বশেষ workflow-তে `linux-tests` সফল, কিন্তু `aarch64-qemu` job ব্যর্থ হয়েছে। এটি workflow-র ফলাফল; ARM64 hardware boot-এর ফলাফল নয়।

**কোড থেকে নির্ণীত ঝুঁকি** হলো এমন অসঙ্গতি যা বর্তমান code path-এ দেখা যায়, কিন্তু বাস্তব ডিভাইসে ক্ষতিকর ফল ঘটেছে বলে আমরা দাবি করছি না। যেমন, flasher-এর generic partition assumptions এবং `fajita`-র profile-specific প্রয়োজন এক নয়। এটির অর্থ “ভুল partition-এ flash হয়েছে” নয়; অর্থ “সেই ঝুঁকি নিরসনের জন্য প্রয়োজনীয় যাচাই বাকি”।

**প্রস্তাবিত ভবিষ্যৎ কাজ** হলো এই রূপরেখার acceptance criteria। এগুলো বাস্তবে করা না পর্যন্ত `planned` থাকবে। এই পার্থক্য বজায় রাখা জরুরি, কারণ OS development-এ অতিরঞ্জিত status মানুষকে ভুল image flash করতে, fake sensor data-কে real ভাবতে, বা security boundary যাচাই ছাড়াই বিশ্বাস করতে প্রলুব্ধ করতে পারে।

## ১.৪ রূপরেখা ব্যবহার করার নিয়ম

প্রতিটি অধ্যায়কে আলাদা engineering epic হিসেবে ব্যবহার করা যায়। কাজ শুরুর আগে ছোট issue তৈরি করতে হবে; issue-তে বর্তমান আচরণ, পুনরুৎপাদনের ধাপ, expected behavior, সংশ্লিষ্ট ফাইল, test strategy এবং rollback লিখতে হবে। কাজ শেষে “কোড লিখেছি” বলা যথেষ্ট নয়। সংশ্লিষ্ট automated test, generated artefact, serial log, manifest, checksum বা hardware test record দিতে হবে। কোনো ধাপে বাধা এলে তার ব্যর্থতার কারণ লিখতে হবে; test skip করে green badge তৈরি করা যাবে না। এই নিয়মই রূপরেখাকে দীর্ঘ কিন্তু ব্যবহারযোগ্য রাখবে।

---

# ২. বর্তমান অবস্থার সংক্ষিপ্ত অডিট

## ২.১ সর্বশেষ repository state

সর্বশেষ দৃশ্যমান কমিট [`be64e988`](https://github.com/joysriramsarkar/onuronOS/commit/be64e988a9a2a9658b300496f3fb601dd4aeb798), যার message হলো “feat: add Android host application and shell implementation for OnuronOS mobile port”। এর আগের [`5108c44`](https://github.com/joysriramsarkar/onuronOS/commit/5108c44a4fcf6fe05838251c10e78f74f4606526)-এ ARM64 QEMU workflow, image-building hardening, `mkdisk.py`, flashing gates এবং ADR-0001 থেকে ADR-0009 যোগ হয়েছিল। নতুন `be64e98` commit-এ shell-এর Calculator, Notes, Music এবং Camera screen, terminal commands এবং application-state fields যোগ করা হয়েছে। এই উন্নতি UI prototype-কে বড় করেছে, কিন্তু নতুন স্ক্রিন থাকা বাস্তব calculator engine, persistent notes database, audio playback বা camera capture সম্পূর্ণ হওয়ার প্রমাণ নয়। `docs/maturity.toml`-এ একাধিক subsystem-কে এখনও stub বা simulated বলে চিহ্নিত করা আছে; সেই classification বজায় রাখা দরকার।

## ২.২ সর্বশেষ CI-এর সারাংশ

সর্বশেষ visible workflow রান [`38024019829`](https://github.com/joysriramsarkar/onuronOS/actions/runs/38024019829)। এতে Linux `linux-tests` job সফল হয়েছে: workspace tests, Python harness unit tests, Clippy, Linux musl build, initramfs packaging, reproducibility check, persistent-data harness এবং x86_64 QEMU boot smoke test—ধাপগুলো সব successful হিসেবে রিপোর্ট হয়েছে। একই workflow-র `aarch64-qemu` job `Build ARM64 musl binaries` ধাপে ব্যর্থ হয়েছে। এর ফলে ARM64 initramfs packaging, ext4 disk formatting এবং ARM64 QEMU boot smoke test skipped হয়েছে।

লগের compiler diagnostic `runtime/nilhal/src/lib.rs:200:32`-এ `E0308` mismatch দেখায়: একটি pointer type `*const u8`, অন্যটি `*const i8`। `CStr::from_ptr`-এর জন্য platform-specific C character type ব্যবহার করা দরকার; hard-coded signed/unsigned byte pointer দিয়ে C ABI ধরে নেওয়া ঠিক নয়। এই compile failure-ই এখন প্রথম blocking defect। এটিকে ঠিক না করে ARM64 boot-এর status “working” বলা যাবে না।

একই latest commit-এ Code Quality, Security Audit, Windows Simulator, Farm CI এবং SELinux Policy CI সফল হয়েছে। এগুলো ইতিবাচক, কিন্তু এই test suites ARM64 kernel/userspace boot বা physical phone behavior-কে প্রতিস্থাপন করে না। CI green-কে নির্দিষ্ট job-এর scope-এ ব্যাখ্যা করতে হবে।

## ২.৩ বর্তমান শক্তি

১. Project-এ একটি বিস্তৃত Rust workspace আছে এবং build/tooling-এর উল্লেখযোগ্য অংশ automated test দিয়ে পরীক্ষা করা হয়। ২. `mkinitramfs.py`-তে architecture-specific target triple ও pinned kernel checksum ব্যবহার করার চেষ্টা আছে। ৩. `nilinit`, package manager, sandbox, NilHAL, NilLang, UI renderer এবং নানা system daemon-এর জন্য পৃথক crate/path আছে। ৪. `docs/maturity.toml`-এ fake/demo data-র জন্য simulated marker-এর ধারণা রাখা হয়েছে। ৫. ADR-গুলো native OS আর Android-hosted runtime-এর পার্থক্য এবং flashing/verified-boot-এর সতর্কতা লিখিত করেছে। ৬. `build/flash-device.sh`-এ generic target-কে unsupported বলে block করার gate এবং userdata erase-কে opt-in করার পরিবর্তন এসেছে। ৭. x86_64 QEMU boot এবং persistence-এর জন্য repeatable CI path আছে। এগুলো প্রকল্পের ভিত্তি; এগুলোকে মুছে নতুন architecture বানানোর দরকার নেই।

## ২.৪ বর্তমান প্রধান ঘাটতি

- ARM64 target compile হয় না; তাই target-specific integration এখনও অসম্পূর্ণ।
- Build script, target naming, output directories, boot-image format এবং device profile-এর মধ্যে এখনও সমন্বয় দরকার।
- QEMU persistence-এর evidence ফাইল ও live CI result একই commit/run-এর সঙ্গে শক্তভাবে bind করা নেই; evidence provenance উন্নত করতে হবে।
- `nilinit` early mount-এ কিছু failure ignore করে success log করে; data mount না হলে tmpfs fallback user-কে data volatile হওয়ার সতর্কতা দেয়, কিন্তু release mode-এ storage failure policy কঠোর করতে হবে।
- `check_core_health` process-running যাচাই করে; এটি service protocol readiness-এর সমান নয়। `check_readiness`-ও বর্তমানে readiness path উপস্থিতি পরীক্ষা করতে পারে, কিন্তু actual request/response handshake নয়।
- `setup_cgroups()` directory create করলেই cgroup v2 resource control প্রমাণ হয় না। SELinux policy load ব্যর্থ হলেও current code warning দিয়ে এগিয়ে যেতে পারে।
- Android host bridge-এর কিছু path বাস্তব host telemetry যুক্ত করলেও camera/audio/network/telephony-র সব hardware-to-UI path end-to-end প্রমাণিত নয়।
- বর্তমান Android network HAL-এ hard-coded network data রয়েছে; camera capture failure-এ sample JPEG ফেরত দেওয়া হতে পারে; app-level demo screens-এ স্থির content রয়েছে।
- Alap framework-কে ADR-0009-এ দায়িত্ব দেওয়া হয়েছে, কিন্তু top-level `alap/` crate/path tree-তে নেই। তাই architecture doc এবং repository implementation-এর gap পূরণ করতে হবে।
- `fajita`-র জন্য hardware profile ও evidence folder আছে, কিন্তু hardware matrix-এ সব subsystem `PLANNED`; physical bring-up log বা validated full image নেই।
- `mkbootimg.py`-তে header packaging tool আছে, কিন্তু standard boot chain, bootloader-enforced AVB, actual DTB/DTBO, correct slots এবং recovery-tested flash sequence ছাড়া এটি device-ready boot image-এর নিশ্চয়তা নয়।

## ২.৫ বর্তমান status কীভাবে বলা উচিত

এই মুহূর্তে সবচেয়ে সৎ অবস্থান হলো: “x86_64 QEMU prototype-এর automated boot path আছে এবং latest CI run-এ সেটি সফল হয়েছে; ARM64 QEMU target-এর CI job যোগ হয়েছে কিন্তু AArch64 compile failure-এর কারণে boot validation হয়নি; Android-hosted runtime উন্নয়নাধীন; physical native phone port এখনও unvalidated।” এটি ভবিষ্যৎ লক্ষ্যকে ছোট করে না। বরং পরবর্তী engineering কাজের সীমা স্পষ্ট করে এবং প্রকৃত অগ্রগতি পরিমাপযোগ্য করে।

---

# ৩. পণ্যের লক্ষ্য ও আর্কিটেকচারের ভিত্তি

OnuronOS-কে দীর্ঘমেয়াদে তিনটি আলাদা target এবং তাদের মধ্যে ভাগ করা portable developer platform হিসেবে ভাবতে হবে।

## ৩.১ Track A — Virtual reference system

Track A হলো QEMU-তে চালানো Linux kernel + initramfs + `nilinit` + system services + persistent disk + serial console + optional virtual display/input। এই track-এর কাজ বাস্তব ফোনের বদলি নয়; architecture-independent logic, PID 1 behavior, storage semantics, system IPC, package install, app runtime এবং build reproducibility যাচাই করার নিরাপদ testbed। x86_64 track বর্তমানে বেশি পরিণত। AArch64 QEMU track হলো ARM64 userspace ও kernel path-এ architecture-specific bug ধরার gate। এই দুই target-এর জন্য একই interface contract থাকবে, কিন্তু build artifacts আলাদা থাকবে।

Track A-তে pass criteria: clean checkout-এ build সফল; checksum pinning ঠিক; architecture-সঠিক ELF; initramfs manifest তৈরি; VM serial log-এ fatal error নেই; required service readiness handshake সফল; persistent disk-এ test marker লেখা, VM reboot, marker পুনরায় পড়া যায়; corrupted kernel, wrong-architecture binary, missing core service এবং invalid image দিলে pipeline fail করে। Screen screenshot এই পরীক্ষার প্রয়োজনীয় অংশ নয়, যদিও renderer smoke test আলাদা যুক্ত করা যায়।

## ৩.২ Track B — Android-hosted runtime

Track B হলো Android APK, যার ভিতরে OnuronOS-এর hosted shell ও Rust library চলে। Android OS, Linux kernel, drivers, permission manager, surface lifecycle এবং telephony framework—সবই host Android-এর অংশ থাকে। `android-host/`-এর Java/JNI bridge host capabilities-কে Onuron userspace-এর HAL layer-এ প্রকাশ করতে পারে। এই track Samsung Galaxy S25-এ পরীক্ষা করার জন্য কার্যকর, কারণ এখানে Android থেকে বেরিয়ে নতুন OS boot করার দাবি নেই। এই track-এর success criteria: NDK build এবং Gradle APK build reproducible; APK-তে সঠিক ABI-র `.so`; JNI protocol version match; live battery/network events UI-তে প্রতিফলিত; permission denial সঠিকভাবে দেখানো; native library না থাকলে UI স্পষ্ট `hosted/degraded` mode-এ যায়; Camera2/audio path permission-সহ যাচাই; Activity pause/resume/rotation/destruction-এ thread ও surface leak হয় না।

## ৩.৩ Track C — Native phone port

Track C-তে ফোনের boot chain থেকে Linux kernel, device tree, firmware এবং native `nilinit` সরাসরি চলবে। Android APK এখানে host হবে না। ফোনের জন্য SoC-নির্দিষ্ট kernel config, DTB/DTBO, display panel driver, touch controller, UFS, PMIC, charger, Wi-Fi, Bluetooth, audio codec, modem, camera sensor, secure boot/AVB এবং partition layout লাগবে। শুধু ARM64 compilation সফল হলেই এই স্তরের কাজ শুরু থেকে শেষ পর্যন্ত সম্ভব হয়ে যায় না। Generic QEMU kernel physical phone-এর kernel হিসেবে বিবেচনা করা যাবে না।

ADR-0007 অনুযায়ী OnePlus 6T (`fajita`) candidate হিসেবে নির্বাচিত, Samsung S25 hosted runtime target হিসেবে রাখা হয়েছে। এই সিদ্ধান্তকে implement করতে হলে আগে QEMU ARM64 gate ও Android hosted gate পাস করতে হবে; তারপর hardware acquisition, bootloader status যাচাই, recovery package সংগ্রহ, stock image backup এবং exact partition map নির্ধারণ করতে হবে। Hardware access না থাকলে Track C-র ওই ধাপগুলো “planning” থাকবে।

## ৩.৪ Shared portable platform

NilLang application source, Alap component API, `nilui` declarative scene model এবং system capability API যতটা সম্ভব platform-neutral হতে পারে। কিন্তু “একই source code” মানে সব device-এ একই underlying implementation নয়। উদাহরণ: display capability-র public API একই থাকলেও QEMU-তে VirtIO/GPU, S25 hosted mode-এ Android Surface/Canvas, এবং native OnePlus-এ DRM/KMS backend হতে পারে। প্রতিটি backend-কে success, unsupported, permission denied, timeout, not ready এবং backend unavailable—এমন নির্দিষ্ট error semantics দিতে হবে। কোথাও hard-coded demo state ফিরিয়ে বাস্তব success দেখানো যাবে না।

## ৩.৫ Product promise-এর সীমা

README, UI, screenshots এবং release note-এ ফিচার status একই taxonomy দিয়ে দেখাতে হবে: `planned`, `prototype`, `simulated`, `tested-in-QEMU`, `tested-on-Android-host`, `tested-on-physical-device`, `release-candidate`, `production`। একটি feature এক track-এ prototype আর অন্য track-এ unimplemented হতে পারে। যেমন camera screen S25 APK-তে দেখা গেলেও native Linux `camerad` hardware capture-কে proven বলা যাবে না। এই পার্থক্য developer ও tester—দু’পক্ষের প্রত্যাশা নিয়ন্ত্রণ করবে।

---

# ৪. অগ্রাধিকার কাঠামো এবং কাজের নিয়ম

প্রতিটি কাজ priority, dependency এবং risk অনুযায়ী সাজানো হবে।

- **P0 / Stop-ship:** ভুল architecture, ভুল image, persistent data loss, boot failure, key/trust failure, privilege bypass বা হার্ডওয়্যার ক্ষতির ঝুঁকি। এ ধরনের issue open থাকলে physical flash বা release candidate তৈরি করা যাবে না।
- **P1 / Critical path:** QEMU ARM64 boot, true service readiness, storage persistence, hosted JNI pipeline, package launch end-to-end এবং real UI/render integration-এর মতো বিষয়—এগুলো ছাড়া OS end-to-end product হিসেবে গণ্য হতে পারে না।
- **P2 / Required before beta:** accessibility, diagnostics, robust Settings, network/audio/camera integration, performance budget, OTA rollback tests এবং documentation polish।
- **P3 / Later:** app store discovery, large widget catalogue, broad Android compatibility, mesh ecosystem expansion, advanced effects, nonessential animations ইত্যাদি। এগুলো P0/P1 অতিক্রম করে project focus দখল করতে পারবে না।

## ৪.১ Dependency-first সিদ্ধান্ত

কাজের প্রলোভন হয় UI, demo app, icon, wallpaper বা আরেকটি daemon যোগ করার দিকে; কারণ তা দৃশ্যমান। কিন্তু boot pipeline ভুল হলে সুন্দর UI ভিন্ন architecture-এ compile হয়ে যেতে পারে। তাই dependency order হবে: toolchain → cross-build → rootfs/initramfs → QEMU boot → persistent data → service readiness → runtime and package install → rendering/input → host backend → device-specific kernel/DTB → physical test। এই ক্রমে পরের স্তরে যাওয়া আগের স্তরের evidence-নির্ভর হবে।

## ৪.২ এক সময়ে একটি critical path

একটি sprint-এ একই সঙ্গে NilLang grammar, audio HAL, device-tree port, app store এবং launcher redesign-এ কাজ করা উচিত নয়। প্রথমে একটি critical epic বেছে নিয়ে ছোট PR-এ ভাগ করতে হবে। উদাহরণ: ARM64 compilation fix → ARM64 initramfs → ARM64 boot → storage persistence। এই ধারার মাঝখানে unrelated UI feature ঢোকালে code review, failure localization এবং regression attribution কঠিন হয়।

## ৪.৩ Feature freeze কোথায় দরকার

ARM64 build green না হওয়া পর্যন্ত `shell`-এ নতুন demo app যোগ করার বদলে boot/build defects-এ সময় দেওয়া উচিত। QEMU ARM64 boot এবং persistence pass হলে S25 hosted runtime-এ কাজের সময় নির্ধারণ করা যায়। Hosted runtime-এর end-to-end tests পাস হলে OnePlus-specific preparation শুরু করা যায়। Freeze মানে পুরো project বন্ধ নয়; শুধু critical path-কে ব্যাহত করতে পারে এমন নতুন scope সাময়িক বন্ধ রাখা।

## ৪.৪ কোনো failure-কে আড়াল করা যাবে না

Test skip, fallback, mock অথবা warning acceptable হতে পারে developer convenience-এর জন্য, কিন্তু তা ফলাফলে স্পষ্ট দেখাতে হবে। `allow_skip`-এর মাধ্যমে QEMU binary না থাকলে test skipped হওয়া developer-only; CI-required job-এ সেটিকে pass হিসেবে গণ্য করা যাবে না। Test mode বা fake backend ব্যবহার করলে artefact manifest-এ `non_release=true`, `backend=fake` বা `simulated=true` উল্লেখ থাকবে। Release candidate build-এ production codepath-এর critical initialization fail করলে `exit nonzero` বা recovery mode-এ যেতে হবে—চুপচাপ degrade করা যাবে না, যদি না নির্ধারিত degraded behavior-টি নিরাপদ ও স্পষ্টভাবে ঘোষিত হয়।

---

# ৫. প্রথম ৭২ ঘণ্টা: তাত্ক্ষণিক স্থিতিশীলতা

এই অধ্যায়ের লক্ষ্য নতুন feature নয়; পরের প্রতিটি কাজে বিশ্বাসযোগ্য foundation তৈরি করা। প্রথম ৭২ ঘণ্টার কাজকে ছোট, স্বাধীন ও যাচাইযোগ্য PR-এ ভাগ করতে হবে। প্রথম PR-এ NilHAL pointer-type compilation failure ঠিক হবে। দ্বিতীয় PR-এ ARM64 job আবার চালিয়ে failure message সংগ্রহ হবে। তৃতীয় PR-এ target naming ও builder semantics পরীক্ষা করা হবে। চতুর্থ PR-এ evidence files-কে current CI artefact থেকে পৃথক করা হবে। সবশেষে একটি small status note প্রকাশ করতে হবে—কী পাস, কী ফেল, এবং পরের gate কী।

## ৫.১ প্রথম দিনের checklist

প্রথমে locally `git status`, `git rev-parse HEAD`, `cargo test --workspace --all-targets` এবং ARM64 target build reproduce করতে হবে। সুনির্দিষ্ট error commit SHA-সহ issue-তে যুক্ত হবে। Rust cross-compilation কাজ চালানোর আগে CI-তে যে target triple ব্যবহৃত হচ্ছে সেটি local-এ একইভাবে স্থাপন করতে হবে। Type fix-এর সঙ্গে test যোগ করতে হবে, যাতে target-specific C pointer declaration compile হয়। শুধু একটি cast বসিয়ে compiler-কে চুপ করানো যাবে না; public C ABI struct-এ field type ও plugin ABI declaration দু’দিক একই contract পালন করছে কি না দেখতে হবে। তারপর `cargo test` ও host Clippy; এরপর `cargo build --target aarch64-unknown-linux-musl --workspace` চালাতে হবে।

একই দিনে `build/mkinitramfs.py --arch aarch64` চালিয়ে দেখার আগে workspace cross-build green হওয়া দরকার। Error হলে সেটি পরের ধাপে যাচাই করতে হবে। Download failure, checksum mismatch এবং target-binary missing—এই তিন ধরনের ব্যর্থতা স্পষ্ট আলাদা error message দেবে। `out/aarch64-qemu`-এর পূর্ববর্তী stale files যেন নতুন failed build-কে পুরোনো successful artifacts দিয়ে pass করিয়ে না দেয়, তা নিশ্চিত করা দরকার। Build শুরুতে artifact staging directory clean করা বা manifest-এ git SHA bind করা—দুইয়ের একটি কঠোর নীতি প্রয়োজন। পুরোনো initramfs ফাইল থেকে success বোঝা যাবে না।

## ৫.২ দ্বিতীয় দিনের checklist

AArch64 compilation পাস হলে ARM64 rootfs package এবং ELF audit চালাতে হবে। `readelf -h` দিয়ে `nilinit`, `nild`, `nilbus`, `nilshell`, `netd`, `audiod`, `powerd`, `nilkeyd`—সব selected binaries-এর machine field `AArch64` কিনা যাচাই করতে হবে। Static linking দাবি করলে `readelf -l` এবং `ldd`-এর উপযুক্ত equivalent দিয়ে dynamic interpreter/dependency আছে কি না পরীক্ষা করতে হবে। Initramfs extract করে `/init`, `/sbin/init`, `/usr/bin/nilinit` mode, ownership, config file এবং directory layout পরীক্ষা করতে হবে। তারপর `mkdisk.py --real --force` বা documented safe path-এ real ext4 image তৈরি করে filesystem signature ও label যাচাই করতে হবে।

QEMU boot test-এ শুধু “boot completed” string যথেষ্ট নয়; serial log-এ mount errors, service launch errors, permission errors এবং fallback markers-ও parse করতে হবে। `/data` mount সফল হয়েছিল কি না runtime থেকে marker লেখা, shutdown, restart এবং পুনরায় পড়া দিয়ে যাচাই করতে হবে। যদি smoke runner interactive process-কে terminate করে, আগে test marker sync/flush হয়েছে কি না নিশ্চিত করতে হবে। VM reboot-এর পর checksum comparison করতে হবে। Test harness-এ disk path explicit হবে; host machine-এর stale default disk accidental reuse হবে না।

## ৫.৩ তৃতীয় দিনের checklist

CI evidence-এ প্রতিটি JSON manifest-এ `source_revision`, `workflow_run_id`, `created_at_utc`, `target`, `kernel_sha256`, `initramfs_sha256`, `disk_sha256`, compiler version, build command, boot-log artifact link এবং test statuses থাকতে হবে। Generated evidence commit-এ manually “all_healthy=true” লেখা চলবে না; CI test ফলাফল থেকে তৈরি হবে। যদি evidence-কে repository-তে রাখা হয়, generation script ও provenance metadata রাখবে। Runtime log না থাকলে evidence file-এ status `not_run` থাকবে, `pass` নয়।

এরপর latest state-এর অগ্রগতি নথিভুক্ত করতে হবে: AArch64 compile fix, ARM64 packaging, boot smoke, storage persistence, service readiness—প্রতিটি আলাদা checkbox। Project board-এ issue dependency link থাকলে কেউ ভুল করে phone port শুরু করবে না। এই ৭২ ঘণ্টার শেষে সফলতার মাপকাঠি অনেক feature নয়; current main থেকে দুই architecture-এর build ও reproducible QEMU evidence কতটা শক্ত হয়েছে, সেটিই মাপকাঠি।

---

# ৬. P0-1: NilHAL-এর ARM64 compilation ত্রুটি

## ৬.১ ত্রুটির প্রকৃতি

সর্বশেষ CI log-এ `runtime/nilhal/src/lib.rs:200:32`-এর কাছে `CStr::from_ptr((*self.module).name)` compile error দিয়েছে। Compiler দেখিয়েছে expected pointer `*const u8`, পাওয়া গেছে `*const i8`। এই ধরনের সমস্যা সাধারণত C ABI type declaration-কে Rust-এর platform-dependent C types-এর বদলে hard-coded byte type দিয়ে লেখা, অথবা আলাদা ABI declaration থেকে signed/unsigned pointer mismatch হওয়ার কারণে হয়। AArch64 target-এ `c_char`-এর underlying representation host architecture-এর তুলনায় আলাদা হতে পারে। সঠিক সমাধান হলো C string pointer-এর জন্য `std::ffi::c_char` বা `std::os::raw::c_char` contract ব্যবহার করা এবং plugin module-এর struct declaration-কে একই type-এ সংজ্ঞায়িত করা।

এখানে সবচেয়ে বিপজ্জনক quick fix হলো `as *const u8` বা `as *const i8` নির্বিচারে বসিয়ে compile করিয়ে নেওয়া। Cast pointer-এর type মিলিয়ে দিতে পারে, কিন্তু ABI contract, NUL-termination, lifetime অথবা caller/callee-এর ownership সমস্যার সমাধান করে না। আগে `HalModule`/plugin descriptor struct কোথায় declared হয়েছে, field `name` কী type, external header or `repr(C)` mapping কোন type ব্যবহার করছে এবং target-specific cfg কোথাও C char আলাদাভাবে define করেছে কি না—এসব পরীক্ষা করতে হবে। তারপর একটি cross-target compile regression test যোগ করতে হবে।

## ৬.২ বাস্তবায়নের পদ্ধতি

প্রথমে C-ABI boundary-র সমস্ত type একটি header-contract module-এ কেন্দ্রীভূত করতে হবে। উদাহরণস্বরূপ, string field যদি C `char *` বোঝায়, Rust side-এ `*const c_char`; binary payload হলে `*const u8`; enum/flag হলে fixed-width integer বা `repr(C)` enum; nullable callback হলে `Option<unsafe extern "C" fn(...)>`—যথাযথ ঘোষণা প্রয়োজন। `CStr::from_ptr` কেবল NUL-terminated valid C string pointer পেলে নিরাপদ। Module descriptor-এর `name` pointer কোথা থেকে আসে এবং library unload হওয়ার আগে তা valid থাকে কি না—তা স্পষ্ট করতে হবে। `get_name()` call-এর সময় library-এর lifetime `HalDevice`-এর `_lib` field দিয়ে রক্ষা করা হচ্ছে বলে মনে হলেও plugin descriptor memory ownership আলাদা হলে সেই contract নথিতে থাকতে হবে।

Type ঠিক করার পরে compile matrix-এ অন্তত host default target, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` এবং Android `aarch64-linux-android` যুক্ত হবে যেখানে crate-এর conditional configuration অনুমতি দেয়। সব target-এ একই tests চালানো সম্ভব না হলে compile-check, unit-test এবং integration-test পৃথকভাবে দেখাতে হবে। Cross-compiled binary execute করা যায় না এমন CI runner-এ QEMU test বা compile-only gate দিয়ে স্পষ্ট পার্থক্য করতে হবে।

## ৬.৩ গ্রহণযোগ্যতার শর্ত

- `cargo check -p nilhal --target aarch64-unknown-linux-musl` সফল।
- workspace-wide ARM64 `cargo build --release --workspace --target aarch64-unknown-linux-musl` সফল।
- host x86_64 workspace tests ও Clippy regression ছাড়া সফল।
- Android-host native library target-এর জন্য plugin ABI break না হওয়া; JNI/native integration tests successful।
- type conversion/unsafe call-এ explicit safety comment ও ownership rule আছে।
- `nilhal` এর plugin load test valid name, null name, malformed module descriptor, missing symbol, wrong ABI version এবং library unload behaviour যাচাই করে।
- CI log-এ ARM64 job আর `E0308` error-এ থামে না; পরের step fail করলে সেই failure-কে আলাদা issue হিসেবে ধরা হয়।

## ৬.৪ আরও বড় শিক্ষা

একটি pointer error ছোট হলেও এটি দেখায় যে target portability শুধু Rust লেখা দিয়ে নিশ্চিত হয় না। `repr(C)`, `c_char`, endianness, alignment, calling convention, atomic widths, syscalls, target-specific `cfg` এবং external C library assumptions—সবই cross-platform contract-এর অংশ। তাই ভবিষ্যতে নতুন HAL module যোগ করার PR-এ host-only test নয়, target compile matrix-ও প্রয়োজন হবে। এই gate প্রকল্পের স্থায়ী নিয়ম হওয়া উচিত।

---

# ৭. P0-2: বিল্ড টার্গেট, নামকরণ ও আউটপুট ডিরেক্টরির একীকরণ

রিপোজিটরিতে canonical naming-এর জন্য ADR-0002 আছে: `qemu-x86_64`, `qemu-aarch64`, `android-host-arm64` এবং `oneplus-fajita`। কিন্তু legacy alias এবং আলাদা script-এর output mapping এখনও mismatch তৈরি করতে পারে। `mkinitramfs.py` বর্তমানে x86_64 artifacts-কে `out/x86_64-generic`, AArch64 QEMU artifacts-কে `out/aarch64-qemu`-তে রাখে। `build/build.sh`-এ `arm64-generic`, `aarch64-generic` ও `aarch64-qemu` target path-এর logic আছে; `flash-device.sh`-এ `oneplus-fajita`/`fajita` profile আছে। নামের পার্থক্য ছোট মনে হলেও ভুল directory থেকে stale image নেওয়ার বাস্তব ঝুঁকি আছে।

## ৭.১ একক target manifest

একটি canonical target registry তৈরি করা উচিত—উদাহরণস্বরূপ `targets/qemu-x86_64/target.toml`, `targets/qemu-aarch64/target.toml`, `targets/android-host-arm64/target.toml`, `targets/oneplus-fajita/target.toml`। প্রতিটি target manifest-এ CPU architecture, Rust target triple, kernel source/checksum ID, image format, expected output file, QEMU machine, bootargs, storage device, firmware/DTB inputs, supported build hosts এবং flashing allowed flag থাকবে। Build scripts target names hard-code করে ভিন্ন mapping তৈরি করবে না; একক parser/registry থেকে default নেওয়া হবে। Legacy aliases সাময়িকভাবে translate হবে এবং deprecation message দেখাবে।

প্রতিটি build artifact তার target-এর identity বহন করবে। `manifest.json`-এ architecture এবং output directory দুটোই থাকবে। Consumer tool boot.img বা system.img তৈরি/flash করার আগে manifest-এর `target` নিজের নির্বাচিত target-এর সঙ্গে মেলায় কি না পরীক্ষা করবে। কোনো manifest নেই বা unknown target হলে refuse করবে। এটি shell script-এ optional warning নয়; fail-closed rule হতে হবে। `out/<target>` পরিষ্কার করার সময় canonical target map ছাড়া arbitrary path delete করা যাবে না।

## ৭.২ Build orchestrator-এ source of truth

বর্তমানে `build/build.sh`, `build/build.py`, `mkinitramfs.py`, `mkdisk.py`, `qemu-aarch64.sh`, `qemu-smoke.py`, `mkbootimg.py` এবং flashersের মধ্যে কিছু responsibility overlap করে। উন্নতি হলো সবকিছু এক giant script-এ গুঁজে দেওয়া নয়; বরং responsibilities-এর boundary নির্ধারণ করা। `build.py` orchestration করবে; target registry পড়বে; cargo build target চালাবে; initramfs packer-কে explicit paths দেবে; disk builder explicit disk format করবে; manifest/signing tool artifact provenance লিখবে; QEMU runner test করবে; flash tool শুধু validated release bundle consume করবে। Flash script নিজে image manufacture বা unpinned dependency fetch করবে না।

`mkinitramfs.py`-এর মতো tool CLI-তে explicit `--arch`, `--target-triple`, `--kernel`, `--kernel-sha256`, `--rootfs`, `--output`, `--manifest`, `--non-release-test-mode` থাকতে পারে, তবে canonical registry থাকলে এগুলো registry থেকে derive হবে। যেটি explicit argument হিসেবে দেওয়া হবে, সেটি registry-র সঙ্গে mismatch হলে build fail করবে; চুপচাপ override করবে না। Target aliases কেবল input normalization layer-এ থাকবে, output path canonical naming পাবে।

## ৭.৩ Clean-build ও stale artefact test

CI-তে `rm -rf out/<target>`-এর পরে fresh build করতে হবে, কিন্তু script যেন project root-এর বাইরে path delete না করতে পারে তা unit test করতে হবে। এক run-এ x86_64 build তৈরি, তারপর AArch64 packaging-এ host binary reuse হওয়ার negative test প্রয়োজন। ভুল target manifest দিয়ে `mkbootimg` বা `flash-device` চালালে no-write/no-flash guarantee যাচাই করতে হবে। `out`-এর আগের artifact রেখে নতুন build run করাও test case: যদি নতুন image generate না হয়, `source_revision` mismatch detect করে release packaging বন্ধ হতে হবে।

## ৭.৪ গ্রহণযোগ্যতার শর্ত

- একই target string সব tools-এ একই architecture ও artifact directory নির্দেশ করে।
- পুরনো alias ব্যবহার করলে deprecation warning এবং canonical target দেখায়।
- ভুল architecture, missing manifest বা checksum mismatch-এ build/flash nonzero return করে।
- target build host fallback only explicit development fixture mode-এ; release mode-এ fallback নিষিদ্ধ।
- artifact manifest-এ source commit, toolchain version, target triple, kernel/initrd/disk hashes আছে।
- Windows এবং Linux-এ build entry point একই target identity এবং equivalent output layout তৈরি করে।

---

# ৮. kernel উৎস, checksum pinning এবং reproducible build

## ৮.১ QEMU kernel ও ফোনের kernel এক নয়

`build/mkinitramfs.py`-এর বর্তমান configuration Alpine Linux 3.19 netboot থেকে x86_64 এবং AArch64 kernel নেয় এবং SHA-256 pin করে। QEMU virtual machine-এর জন্য এটি একটি ব্যবহারযোগ্য baseline হতে পারে, যদি checksum, kernel configuration, bootargs এবং driver list সত্যিই target environment-এর সঙ্গে মেলে। কিন্তু OnePlus 6T physical phone-এর জন্য QEMU kernel যথেষ্ট নয়। সেখানে SoC-এর interrupt controller, timer, storage controller, device tree, display panel, touch controller, PMIC, charger, Wi-Fi, Bluetooth, modem interface, audio codec এবং camera sensor-এর উপযুক্ত driver/configuration/firmware প্রয়োজন হবে। Kernel generic ARM64 হওয়ার কারণে ফোনে boot করবে—এমন ধরে নেওয়া যাবে না।

ADR-0003-এ virtual target এবং physical target-এর kernel-source strategy আলাদা করা হয়েছে। এই সিদ্ধান্ত কার্যকর করতে `kernel source mode` manifest-এ থাকবে: `pinned-prebuilt-qemu`, `reproducible-source-build` বা `device-port-tree`. Kernel version, source URL, release hash, config hash, compiler/LLVM version, patch series, defconfig, firmware dependencies এবং license notices build record-এ যুক্ত করতে হবে। Prebuilt kernel-এর SHA-256 pinning integrity check; এটি একাই kernel trustworthy বা secure প্রমাণ করে না। Source provenance, release authority এবং reproducible build আলাদা বিষয়।

## ৮.২ checksum যাচাই: mismatch হলে fail-closed

বর্তমান builder-এ checksum mismatch হলে error throw করার পরিবর্তন এসেছে—এটি ধরে রাখতে হবে। Existing file-এর size “এক মিলিয়নের বেশি” হলেই trusted নয়; exact SHA-256 match করতে হবে। Downloaded kernel-এর checksum mismatch হলে file delete করে nonzero status দিতে হবে। Download exception হলে empty output বা stale file রেখে success return করা যাবে না। Kernel download একটি build prerequisite হলে network failure build fail করবে; cached, checksum-matched kernel ব্যবহার করার offline mode চাইলে তা `--offline` নামে explicit এবং manifest-এ উল্লেখিত হবে।

Checksum pinning-update policy-ও লিখতে হবে। নতুন kernel release নিলে maintainer প্রথমে উৎসের authenticity, version, checksum, boot behavior, vulnerabilities, configuration delta এবং license যাচাই করবেন। নতুন hash update একই PR-এ reason ও source linkসহ হবে; runtime failure থেকে বাঁচতে checksum check সরিয়ে দেওয়া যাবে না। একাধিক architecture-এর hash ভুল করে swap হয়েছে কি না test করতে হবে। Hash শুধু file-কে নির্দিষ্ট করে; একই file আসল official source থেকে এসেছে কি না তা external trusted source/HTTPS provenance policy দিয়ে মূল্যায়ন করতে হবে।

## ৮.৩ ARM64 kernel validation

AArch64 kernel download হওয়া মানেই QEMU `virt`-এ bootযোগ্য নয়। CI-তে kernel architecture/file type যাচাই, kernel command line, serial console (`ttyAMA0`), initramfs compatibility, VirtIO block support এবং required config options check করতে হবে। Kernel config পাওয়া গেলে artifact-এ সংরক্ষণ করবে। Boot hang হলে serial log-এর শেষ lines এবং QEMU exit code evidence হিসেবে সংরক্ষণ করতে হবে। `-no-reboot` ও bounded timeout test runner-এ hang শনাক্ত করতে পারে; কিন্তু timeout-কে “booted” গণ্য করবে না।

## ৮.৪ reproducible artifact

Reproducible build বলতে একই source revision এবং declared toolchain/configuration থেকে পুনরায় build করলে equivalent বা byte-identical artifact পাওয়া। Kernel download reproducibility আলাদা; initramfs reproducibility আলাদা; ext4 image reproducibility আলাদা; APK build reproducibility আলাদা। বর্তমানে real ext4 filesystem timestamp/hash seed-এর কারণে byte-identical নাও হতে পারে—তাই filesystem image content equivalence ও mount behavior-কে image byte hash থেকে আলাদা পরীক্ষা করতে হবে। Initramfs builder-এ deterministic CPIO metadata, sorted file order, normalized mtime, stable gzip metadata এবং stable permissions প্রয়োজন। Build manifest-এ reproducibility status আলাদা field হবে; শুধু একই build-এ দুইবার in-memory compress করে hash মিললেই সমস্ত pipeline reproducible প্রমাণ হয় না।

## ৮.৫ গ্রহণযোগ্যতার শর্ত

- checksum mismatch positive test নয়, negative test হিসেবে নিশ্চিতভাবে build fail করে।
- kernel URL, pinned checksum, architecture এবং source provenance manifest-এ থাকে।
- stale kernel file বা wrong architecture detected হয়।
- kernel update-এর জন্য review checklist থাকে।
- QEMU boot log, config, kernel hash ও initramfs hash CI artifact হিসেবে পাওয়া যায়।
- QEMU kernel এবং native phone kernel source/profile স্পষ্টভাবে পৃথক।

---

# ৯. initramfs ও root filesystem নির্মাণ

## ৯.১ initramfs হলো boot-এ চালু হওয়া প্রথম userspace

`initramfs`-এ `nilinit`-কে `/init` হিসেবে শুরু করা হয়। তাই root filesystem তৈরি করা শুধু Rust binaries copy করার কাজ নয়। এটি earliest boot-এর জন্য প্রয়োজনীয় directory, devices, mount points, service configuration, shared libraries (যদি থাকে), dynamic loader (যদি থাকে), symlinks/compatibility aliases এবং kernel command line ধরে environment তৈরি করে। Wrong architecture-এর `nilinit` থাকলে kernel সেটি execute করতে পারবে না; ভুল dynamic linker থাকলে “not found” দেখাতে পারে, যদিও ফাইল উপস্থিত। Binary mode, ownership, symlink policy এবং critical configs সব যাচাই করতে হবে।

## ৯.২ rootfs builder-এর contract

`prepare_rootfs()` architecture অনুযায়ী release directory থেকে binaries নেয় এবং manifest তৈরি করে। এই আচরণকে explicit allowlist-ভিত্তিক করতে হবে। কোন binary mandatory, কোনটি optional, কোনটি target-specific, কোনটি development-only—এগুলো একটি manifest-এ নির্ধারিত থাকবে। `nilinit`, service supervisor-এর required daemons, recovery utility এবং minimal diagnostics mandatory; optional app sample missing হলে boot বন্ধ করার দরকার নেই। কিন্তু `nilinit` অনুপস্থিত হলে package step অবশ্যই fail করবে। প্রতিটি mandatory executable copy হয়েছে কি না, source hash এবং target hash মেলে কি না, ELF architecture ও program interpreter ঠিক কি না, executable bit সঠিক কি না—সব যাচাই হবে।

বর্তমানে builder `/init` এবং `/sbin/init`-এ `nilinit` copy করে compatibility path তৈরি করে। যদি symlink ব্যবহার করা হয়, CPIO writer symlink semantics ঠিকভাবে preserve করছে কি না test করতে হবে; যদি copy করা হয়, duplicate file size বৃদ্ধি ও hash consistency বিবেচনা করতে হবে। `/etc/nilos/services.toml`-এর fallback `include_str!` compile-time embedded config এবং rootfs-এর config যেন contradictory না হয়। Build-time config version অথবা checksum boot log-এ দেখানো যেতে পারে।

## ৯.৩ file-system safety

Rootfs তৈরির সময় symlink traversal-এর মাধ্যমে project tree-এর বাইরের file copy হওয়া ঠেকাতে হবে। Absolute path, `..`, unexpected devices, sockets, FIFO, world-writable executable এবং setuid bits-এর নীতি স্থির করতে হবে। Initramfs সাধারণত ছোট; সেখানে app store, user data এবং build cache থাকবে না। Kernel critical boot path-এ missing `/dev/console` হলে serial console fallback থাকতে পারে, কিন্তু success log যেন console পাওয়া ও mount সফল হওয়ার অবস্থা আলাদা করে। `/proc`, `/sys`, `/dev`, `/run`, `/tmp` mount-এর failure criticality explicit হবে।

## ৯.৪ binary inventory

Manifest-এ শুধু installed daemon-এর নাম নয়, প্রতিটি binary-র SHA-256, source path, target triple, linked dependency summary, build profile, feature flags এবং version থাকবে। “installed_daemons” field-এর নাম যেন non-daemon utilities বা shell apps-কে বিভ্রান্তিকরভাবে daemon না বলে। Binary names এবং service config entries-এর consistency test থাকবে: config-এ service `exec` path আছে কিন্তু binary install হয়নি—এমন হলে packaging error বা explicit optional declaration প্রয়োজন।

## ৯.৫ rootfs integration test

CI test rootfs CPIO archive extract করে verify করবে: `/init` executable; `nilinit` architecture matching; `services.toml` parseable; service paths exist; required directories exist; symlinks allowed target-এর বাইরে যায় না; device nodes policy অনুযায়ী তৈরি; permissions hardened; manifest contents archive file list-এর সঙ্গে মেলে। CPIO archive truncation, corrupt gzip, duplicate paths, malicious symlinks, null bytes এবং path traversal-এর test থাকবে। Target-এ চালাতে না পারলেও static archive inspection করা যাবে; AArch64 QEMU boot আলাদা integration test হবে।

## ৯.৬ গ্রহণযোগ্যতার শর্ত

- নতুন clone থেকে clean rootfs ও initramfs তৈরি হয়।
- wrong target binary, missing required daemon এবং malformed config build fail করে।
- archive contents manifest এবং checksums-এর সঙ্গে মেলে।
- rootfs input symlink/path traversal দিয়ে host files অন্তর্ভুক্ত করা যায় না।
- logs-এ mount এবং console errors সত্যভিত্তিক; failed mount-এর পর success message দেওয়া হয় না।
- `--allow-host-binaries-for-tests` থাকলে output manifest স্পষ্ট `non_release` চিহ্নিত করে এবং release flasher তা reject করে।

---

# ১০. persistence: ext4 disk, mount failure এবং reboot test

## ১০.১ ফাইল তৈরি হয়েছে মানেই filesystem তৈরি হয়নি

`build/mkdisk.py`-এর উদ্দেশ্য QEMU-তে `/dev/vda` হিসেবে দেখানো data disk তৈরি করা। এতে real filesystem mode এবং pure-Python synthetic fallback আছে। Synthetic mode-এ minimal superblock লেখা হয়, কিন্তু সেটি mountable ext2/ext4 filesystem নয়—ফাইলের নিজস্ব documentation-এও এই সীমা উল্লেখ আছে। `nilinit/src/main.rs` data mount ব্যর্থ হলে tmpfs fallback দেয়, এবং log-এ volatile storage-এর warning দেয়। এই আচরণ development convenience হিসেবে গ্রহণযোগ্য হতে পারে; release acceptance-এ persistent storage expected হলে silent data loss বরদাস্ত করা যাবে না।

## ১০.২ disk builder-এর default আচরণ

CI-তে real ext4 filesystem তৈরি করতে `e2fsprogs` install করা আছে। কিন্তু `mkdisk.py` CLI-র default `auto` mode tool availability-এর উপর নির্ভর করে real বা synthetic image নির্বাচন করতে পারে। এই nondeterministic environment-dependent behavior বিভ্রান্তি তৈরি করে: Linux CI real filesystem তৈরি করলেও Windows development machine synthetic image তৈরি করতে পারে। Proposal হলো `--real` ও `--synthetic` স্পষ্ট মোড রাখা; QEMU release-test target-এর জন্য real filesystem বাধ্যতামূলক করা; developer simulation-এ synthetic mode অনুমোদন করা কিন্তু manifest-এ `storage_format=synthetic_nonmountable` লেখা। Critical test কখনো `auto` mode-এ “filesystem mountable” ধরে নেবে না।

## ১০.৩ format করার আগে existing disk যাচাই

`create_disk_image()`-এ output path-এর file requested size-এর সমান হলে `force` false থাকলে existing disk reuse করা হয়। Persistence-এর জন্য এটি সাধারণত ভালো—প্রতিবার disk reformat হলে user data হারাবে। কিন্তু existing disk corrupt/unknown filesystem হলে blind reuse-ও বিপজ্জনক। সঠিক আচরণ: existing image আছে → read-only inspection; expected ext4 label/UUID/superblock/features মেলে কি না; format action চাইলে explicit `--force` বা `--format-existing` confirmation; wrong-size/truncated image এলে fail বা quarantine। Normal build-এ user data নষ্ট করে fresh image বানানো চলবে না। Test fixture তৈরি করতে আলাদা temporary output path ব্যবহার করতে হবে।

## ১০.৪ nilinit-এর mount policy

`mount_data_partition()` এখন candidate device `/dev/vda`, `/dev/vda1`, `/dev/sda`, `/dev/sda1`, `/dev/hda` পরীক্ষা করে ext4 ও ext2 mount চেষ্টা করে এবং না পারলে tmpfs fallback। Virtual QEMU disk-এ partition table না থাকলে পুরো `/dev/vda`-তেই filesystem থাকতে পারে; phone storage-এ partition map আলাদা হবে। এই candidate scan development-এ সহনীয়, কিন্তু production profile-এ explicit device UUID/label/partition mapping চাই। Wrong filesystem বা corruption-এর কারণে mount ব্যর্থ হলে system log, recovery path এবং UI-তে “storage read-only/unavailable” status দরকার। Critical data filesystem না থাকলে user-এর OOBE configuration, PIN state, installed apps অথবা contacts অস্থায়ী tmpfs-এ লেখা হবে—এটি unacceptable বিভ্রান্তি।

## ১০.৫ persistence test কীভাবে হবে

একটি integration test শুরু হবে fresh real ext4 image থেকে। QEMU boot হওয়ার পর process `/data/config/ci-persistence-marker` ফাইলে random nonce এবং build revision লিখবে; file `fsync` এবং parent directory `fsync` করা হবে। তারপর orderly shutdown/reboot ঘটবে। পরের boot-এ marker খুলে nonce/hash যাচাই হবে। এরপর আবার নতুন content লিখে crash simulation এবং safe shutdown দুটো path পরীক্ষা করতে হবে। `debugfs` দিয়ে host-side image content read-back একটি useful test, কিন্তু সেটি kernel-এর mount path ও runtime write সত্যিই হয়েছে কি না প্রমাণ করে না। দুটো test আলাদা রাখতে হবে: offline image inspection এবং live runtime persistence.

`nilinit` log-এ mounted device, filesystem type, volume UUID/label, `/data` mount options এবং persistence verification state লিখবে—তবে sensitive user data বা encryption key log করবে না। Test run-এ `tmpfs fallback` হলে smoke runner অবশ্যই fail করবে যদি test requirement persistent disk হয়। General developer boot mode-এ fallback চলতে পারে; কিন্তু log ও boot manifest-এ `data_persistence=false` দেখাতে হবে।

## ১০.৬ ভবিষ্যতের encryption

Persistent storage mount সফল হওয়ার পর fscrypt/LUKS/encrypted userdata নিয়ে কাজ করা উচিত; তার আগে encryption flag UI-তে দেখিয়ে লাভ নেই। Encryption-এ key provisioning, hardware-backed key availability, PIN/key derivation, recovery process, lost-key behavior এবং factory reset semantics নির্ধারণ করতে হবে। PIN/hash storage, user app data ও system configuration পৃথক করতে হবে। Encrypt/decrypt না করা test image-কে “encrypted” বলে report করা যাবে না। Boot-time decryption failure হলে data wipe বা plaintext fallback না করে recovery mode-এ যেতে হবে।

## ১০.৭ গ্রহণযোগ্যতার শর্ত

- QEMU persistence test live write → reboot → read যাচাই করে।
- Required-persistence mode-এ tmpfs fallback failure হিসেবে গণ্য হয়।
- Fresh disk creation এবং existing disk reuse আলাদা code path ও test পায়।
- Invalid, truncated, wrong-size ও wrong-filesystem image fail-closed বা recovery state তৈরি করে।
- Logs data loss prevention ও fallback status স্পষ্ট করে, কিন্তু private data প্রকাশ করে না।
- Native phone partition mapping target manifest থেকে আসে; generic candidate scanning-কে production truth ধরা হয় না।

---

# ১১. QEMU x86_64-এর regression gate

x86_64 হলো বর্তমান সবচেয়ে ভালোভাবে পরীক্ষা করা virtual target। সর্বশেষ [Linux/QEMU workflow](https://github.com/joysriramsarkar/onuronOS/actions/runs/38024019829)-তে এই job সফল। এর অর্থ current toolchain ও test harness-এর অধীনে নির্দিষ্ট x86_64 build, packaging, persistence harness এবং serial boot smoke test পাস করেছে। এটিকে স্থিতিশীল ভিত্তি হিসেবে ব্যবহার করা উচিত, কিন্তু “OS complete” বা “production secure” এর সমার্থক করা যাবে না। Regression gate-এর উদ্দেশ্য হলো প্রতিটি পরের PR পুরোনো working path ভাঙছে কি না শনাক্ত করা।

## ১১.১ boot smoke এবং deep integration আলাদা

`build/qemu-smoke.py` বর্তমান log-এ “Onuron OS boot completed” এবং “core services verified healthy” text খুঁজে success নির্ধারণ করে। String-based smoke test দ্রুত ও সরল; কিন্তু একই string ভুল sequence-এ print হওয়া, আগের log-এর অংশ থাকা, service crash-এর পরে text উপস্থিত থাকা অথবা false positive সম্ভব। তাই runner-কে structured boot protocol-এ উন্নীত করা উচিত। উদাহরণস্বরূপ, `nilinit` `/run/onuron/boot-status.json` লিখবে যাতে `schema_version`, `boot_id`, `source_revision`, `required_services`, `service_states`, `data_mount`, `selinux_status`, `overall_status` থাকবে। QEMU guest-এর ভেতরে test agent বা serial protocol দিয়ে এই status বের করা হবে। যদি JSON file guest-এর বাইরে সরাসরি পাওয়া না যায়, serial console-এ versioned marker line দেওয়া যায়; parser সেটিকে structured object হিসেবে validate করবে। শুধু substring search থাকবে fallback diagnostic হিসেবে, acceptance oracle হিসেবে নয়।

## ১১.২ test suite-এর স্তর

১. Unit tests: parsers, path validation, mount planning, restart policy, protocol serialization, package signatures। ২. Component integration: `nilinit` + `services.toml`; `nilpkg` install/verify; `nilrt-launch` sandbox; `nilui` render model। ৩. Boot integration: kernel + initramfs + PID 1 + required daemons। ৪. Storage integration: live ext4 write/reboot/read। ৫. Failure injection: required service missing, bad config, wrong kernel hash, corrupt disk, SELinux policy missing, `/dev/console` unavailable। ৬. UI smoke: renderer start, scene displayed, input event dispatched। ৭. Soak test: reboot loops, service restarts, long-running apps, storage I/O and memory use। সব test একই সময় সব target-এ চালানো সম্ভব না হলেও matrix-এ কোন level কোন target-এ চলে সেটি স্পষ্ট করা উচিত।

## ১১.৩ x86_64 gate-এর acceptance

একটি clean Linux runner-এ workspace tests, Rust lint, Python tests, cross-target build, deterministic initramfs, disk image check, QEMU boot এবং persistence marker test সব পাস করতে হবে। QEMU test যদি dependency অনুপস্থিতির কারণে skipped হয় তবে required CI job red হবে। Test runner success-এর সঙ্গে log artifact upload করবে, যাতে failure reproducing করা যায়। Timeout, QEMU crash, kernel panic, PID1 error, missing required process, missing readiness handshake এবং non-persistent data—সব failure আলাদা exit code বা human-readable message পাবে।

## ১১.৪ regression artefacts

প্রতিটি successful CI run-এর manifest-এ kernel, initramfs, rootfs manifest এবং data disk-এর SHA-256 থাকবে। `build_manifest.json`-এ target ও git revision থাকতে হবে; `build_revision=unknown` হলে release candidate আটকে যাবে। QEMU memory, vCPU count, machine type, CPU model ও kernel command line record করতে হবে। Artifact retention policy-তে latest successful main run এবং release candidate দীর্ঘমেয়াদে রাখা উচিত; সাধারণ PR artefact অল্প সময় রাখা যেতে পারে। Test logs ব্যক্তিগত user data বহন করবে না।

---

# ১২. QEMU ARM64-এর পূর্ণ bring-up gate

## ১২.১ বর্তমান অবস্থা

AArch64-এর জন্য `build/qemu-aarch64.sh`, PowerShell counterpart, `mkinitramfs.py` architecture config এবং `.github/workflows/linux-qemu.yml`-তে আলাদা job আছে। কিন্তু সর্বশেষ run-এ ARM64 workspace cross-build compilation failure-এ থেমেছে। তাই packaging, real ext4 creation এবং boot smoke steps skipped হয়েছে। `docs/evidence/qemu-aarch64`-এ manifest, persistence এবং service-health নথি থাকলেও latest CI evidence-এর সঙ্গে সেগুলোর provenance bind না করলে এগুলো সর্বশেষ commit-এর সফল test result বলে গণ্য হবে না। এই সত্যটি status-এ প্রকাশ করতে হবে।

## ১২.২ প্রথম ধাপ: ARM64 workspace build

NilHAL compile error ঠিক করার পর পুরো `cargo build --release --workspace --target aarch64-unknown-linux-musl` আবার চালাতে হবে। পরবর্তী failure পেলে একইভাবে fix-test-repeat করতে হবে। Cross-linker `aarch64-linux-gnu-gcc` এবং musl target-এর linker assumptions যাচাই করতে হবে; glibc cross-linker থাকার অর্থ musl linking স্বয়ংক্রিয়ভাবে সঠিক নয়। `rustup target add` target standard libraries সরবরাহ করে, কিন্তু linker ও native dependency compatibility আলাদা বিষয়। Crate-specific build script, C dependency, assembly, feature detection এবং syscalls target-এ compile হয় কি না দেখতে হবে।

## ১২.৩ দ্বিতীয় ধাপ: architecture-pure rootfs

ARM64 binary build সফল হওয়ার পর `mkinitramfs.py --arch aarch64` চালাতে হবে। Builder ভুল করে `target/release` host binary fallback করতে পারবে না। Manifest প্রতিটি ELF-এর machine type verify করবে। Source fallback `script/native` হিসেবে classify করা হলে সেটিও inspect করতে হবে—ELF না হলে script/shebang/interpreter এবং shell availability check করা দরকার। Rust binary dynamically linked হলে interpreter path ARM64 rootfs-এ আছে কি না verify করতে হবে। Rootfs CPIO archive parse এবং extraction test-ও দরকার।

## ১২.৪ তৃতীয় ধাপ: kernel, data disk ও virtual hardware

Pinned AArch64 kernel checksum match না করলে package step fail করবে। QEMU command-এ `-M virt`, নির্ধারিত CPU profile, serial console, memory, SMP এবং virtio-blk drive যুক্ত হবে। `qemu-smoke.py --arch aarch64`-এর disk attachment path `out/aarch64-qemu/data.img`-এর সঙ্গে মিলতে হবে। `mkdisk.py`-কে `--real` mode-এ চালানো দরকার। Test log-এর মধ্যে `virtio_blk`, mount result, `/data` persistence এবং core service readiness পাওয়া চাই। QEMU-তে GUI smoke later milestone হতে পারে; প্রথম gate serial boot/persistence হবে।

## ১২.৫ চতুর্থ ধাপ: service readiness ও persistence

AArch64-এ যে সাতটি core service required বলে `nilinit` দেখায়—`nild`, `nilkeyd`, `nilbus`, `netd`, `audiod`, `powerd`, `nilshell`—সেগুলোর launch হওয়া, process alive থাকা এবং readiness protocol উত্তর দেওয়ার পার্থক্য তৈরি করতে হবে। `services.toml`-এ daemon command host binary path-এর সঙ্গে মেলে কি না verify করুন। Service startup order dependency থাকলে তা explicit করা দরকার। QEMU boot-এর পরে service crash হলে supervisor restart behaviour log-এ স্পষ্ট হবে। এরপর disk persistence integration test চালানো হবে; ext4 superblock parser কেবল format আছে বলে শনাক্ত করতে পারে, real write persistence নয়।

## ১২.৬ ARM64 CI reporting

Job `success` হতে হলে compile → package → filesystem format → QEMU boot → readiness → persistence সব mandatory step পাস করতে হবে। Failure হলে `if: always()`-এর artifact upload log/partial manifest সংরক্ষণ করবে, কিন্তু upload step successful হওয়ায় job result green হওয়া যাবে না। বর্তমানে artifact path not found হলে warning দিয়ে upload step সফল দেখাতে পারে; এটি গ্রহণযোগ্য, কারণ মূল build step fail হলে job fail থাকে। তবে report-এ “artifact missing because build failed” স্পষ্ট করা যায়।

## ১২.৭ ARM64 acceptance checklist

- [ ] workspace cross-compile সফল এবং log artifact সংরক্ষিত।
- [ ] সব required binaries `AArch64`।
- [ ] wrong-architecture regression test fail করে।
- [ ] pinned kernel hash exact match।
- [ ] initramfs packaging থেকে valid manifest/checksum তৈরি।
- [ ] QEMU serial boot reaches explicit ready state।
- [ ] required service readiness protocol সফল।
- [ ] real ext4 `/data` mount হয়।
- [ ] marker write → reboot → read পাস করে।
- [ ] boot fail injection expected status তৈরি করে।
- [ ] evidence artefact current git SHA ও workflow run ID-র সঙ্গে যুক্ত।

---

# ১৩. nilinit, supervision এবং সত্যিকারের service readiness

`nilinit` PID 1-এর দায়িত্ব সাধারণ application launcher-এর চেয়ে অনেক বেশি। এটি early filesystem mount, persistent storage attach, SELinux load, cgroup setup, boot recovery decision, service start, socket activation, supervision এবং shutdown orchestration করে। PID 1 crash বা ভুল mount policy পুরো system boot failure ঘটাতে পারে। তাই `nilinit`-কে শুধু “success log print করে এবং processes চালায়” এমন program হিসেবে না দেখে system state machine হিসেবে ডিজাইন করতে হবে।

## ১৩.১ boot state machine

প্রস্তাবিত state: `Start → ConsoleReady → EssentialMountsReady → DataStorageReady → SecurityPolicyReady → ConfigValidated → CoreServicesStarting → CoreServicesReady → UIReady → BootComplete`। প্রতিটি transition-এর precondition থাকবে। Transition ব্যর্থ হলে severity ও recovery policy নির্ধারিত থাকবে। উদাহরণ: serial console অনুপস্থিত হলেও noninteractive target boot করতে পারে, কিন্তু log unavailable status রাখবে; `/proc` বা `/dev` critical mount fail করলে unknown state-এ যাওয়া যাবে না; data partition required target-এ mount fail হলে `RecoveryRequired` বা `DegradedReadOnly` state; SELinux optional dev target-এ disabled থাকতে পারে, কিন্তু release policy target manifest-এ explicit করতে হবে।

## ১৩.২ process running বনাম ready

বর্তমানে `Supervisor::check_core_health()` `running` map-এ নাম আছে কি না দেখে। `spawn()` সফল হলে process map-এ যুক্ত হয়; process startup-এর কিছু পরই crash করতে পারে। Boot health check-এর আগে ৫০ মিলিসেকেন্ড sleep এমন race-কে কিছুটা ধরলেও service initialization, socket bind, database open, device probe কিংবা IPC handshake শেষ হয়েছে কি না নিশ্চিত করে না। `check_readiness()` method-এ socket/file path existence দেখা গেলেও এটি server-এর readiness response চেক করে না। নতুন design-এ service readiness contract থাকতে হবে।

একটি ব্যবস্থা হতে পারে `sd_notify`-সদৃশ Unix socket; অন্যটি নির্দিষ্ট `READY` frame সহ `nilprotocol` IPC; আরেকটি service-specific socket probe। যে পথই বেছে নেওয়া হোক, contract versioned, timeout-bounded এবং privilege-aware হবে। `nild` শুধু process চালু থাকলেই ready নয়: core IPC socket bind, configuration parse এবং request handler প্রস্তুত থাকতে হবে। `netd` hardware network না থাকলেও daemon-ready হতে পারে, কিন্তু backend status `unavailable` হিসেবে প্রকাশ করবে। UI ready এবং phone modem ready-কে পৃথক state হিসেবে expose করতে হবে।

## ১৩.৩ restart policy

Supervisor restart backoff exponential হওয়া এবং healthy uptime-এর পর reset করা ভালো foundation। কিন্তু restart policy-তে service class, maximum restart frequency, required/optional status, dependency failure এবং crash-loop circuit breaker থাকতে হবে। PID 1 যেন প্রতিটি failed service-এর জন্য sleep করে অন্য services-এর supervision আটকে না রাখে। বর্তমান `tick()` restart delay-তে thread sleep করলে lengthy backoff supervisor loop-কে সাময়িক থামাতে পারে; future design-এ per-service `next_restart_at`-এর মতো scheduled timestamp ব্যবহার করা উচিত, যাতে এক service-এর crash আরেক service-এর monitoring দেরি না করায়।

## ১৩.৪ shutdown/reboot path

`/run/onuron/power_action` file দেখা system command channel হিসেবে সহজ, কিন্তু path permissions, atomic write, stale request এবং untrusted app থেকে write protection দরকার। System daemon authenticated IPC-তে reboot/shutdown request পাঠাবে; `nilinit` privilege boundary ও caller credentials যাচাই করবে। Shutdown sequence: reject new launches, stop UI/input, stop noncritical services, flush data, stop networking/modem/audio as applicable, sync storage, unmount, তারপর reboot/poweroff syscall। Timeout overrun হলে emergency sync/reboot policy থাকবে এবং log-এ কোন service timeout হয়েছে তা লিখবে। App-level unprivileged code সরাসরি system action file তৈরি করতে পারবে না।

## ১৩.৫ socket activation

`SocketActivationManager` socket-এ pending connection দেখে service launch করে এবং listening FD-কে child-এর fd 3-তে duplicate করার চেষ্টা করে। এটি তখনই কাজ করবে যখন service inherited file descriptor থেকে accept করতে জানে, environment variable parse করে, fd lifetime বজায় রাখে এবং নিজেরা একই path-এ নতুন listener bind করে conflict তৈরি করে না। সব configured socket-activated service-এ integration test প্রয়োজন। উদাহরণ: no connection → service not running; first valid connection → service starts; service accepts inherited fd; malformed client rejected; multiple pending connections lost হয় না; service crash হলে listener supervisor-এ থাকে; restart হলে new process একই listener reuse করে।

## ১৩.৬ acceptance criteria

- Boot status state machine documented ও machine-readable।
- Mandatory mount/policy/config failure ভুল করে success state দেয় না।
- Core service readiness response bounded timeout-এ verify হয়।
- Crash-loop backoff পুরো supervisor-কে block করে না।
- shutdown requests authenticated এবং data sync/unmount sequence tests আছে।
- socket activation tests বাস্তব child process ও inherited descriptor দিয়ে চলে।
- Recovery mode/boot counter-এর persistence test আছে; boot সফল ঘোষণার আগে counter clear হয় না।

---

# ১৪. early boot, `/proc`, `/sys`, `/dev`, cgroups ও SELinux

## ১৪.১ early mount-এর সত্যতা

`mount_early_fs()`-এ `/proc`, `/sys`, `/dev`, `/run` এবং `/tmp` mount attempt-এর ফল `let _ = mount(...)` দিয়ে ignore করা হয়, এরপর “Early virtual filesystems mounted” success log আসে। এটি debug log-কে বাস্তবতার সঙ্গে অসামঞ্জস্যপূর্ণ করে। প্রতিটি mount-এর outcome capture করতে হবে। কোন mount critical, কোনটি optional এবং কোনটি fallback পেতে পারে—নির্ধারণ করা উচিত। `/proc` PID-1 child supervision/debugging-এর জন্য, `/dev` device access ও console-এর জন্য, `/run` socket/volatile state-এর জন্য গুরুত্বপূর্ণ। `devtmpfs` kernel config ও early device availability-এর উপর নির্ভর করতে পারে; ব্যর্থ হলে `/dev` directory তৈরি করলেই device nodes পাওয়া যায় না।

Mount table helper-এ একটি struct ব্যবহার করা যায়: `source`, `target`, `fstype`, `flags`, `required`, `fallback`, `timeout`, `status`. Error message-এ errno এবং affected target থাকবে। Test-এর জন্য mount syscall mock করা যায়; Linux integration VM-এ actual mount-এর ফল যাচাই হবে। “Mount attempted”, “mounted”, “already mounted”, “fallback active”—এগুলো আলাদা log level/status।

## ১৪.২ cgroups v2

বর্তমান `setup_cgroups()` `/sys/fs/cgroup/onuron.slice` ও compatibility directory create করে এবং control group initialized বলে লগ করে। কিন্তু cgroup v2 mount, controller availability, subtree control, process placement, memory/pids/cpu limits এবং delegation configure না হলে resource control active প্রমাণিত নয়। প্রথমে `/sys/fs/cgroup/cgroup.controllers`, `cgroup.subtree_control`, mountinfo এবং kernel config detect করতে হবে। তারপর system parent cgroup, service cgroup, app cgroup structure তৈরি ও controllers enable করা হবে। Parent/child process placement root privileges ও writable cgroup filesystem-এর শর্তে করা হবে। কোনো controller unavailable হলে explicit feature status হবে, fake success নয়।

`nilrt` launch হওয়া প্রতিটি app-এর জন্য memory limit, process limit, CPU weight এবং optional I/O budget policy দিতে হবে। Emergency system services app exhaustion থেকে সুরক্ষিত থাকবে। Kill-on-memory-pressure behavior, OOM log, app restart policy এবং data integrity আলাদা test করতে হবে। QEMU-তে cgroup controls disabled থাকলে test gracefully skip করতে পারে only if test suite declares them optional; release candidate’s mandatory isolation profile must fail if the required kernel controllers are missing.

## ১৪.৩ SELinux policy load

`load_selinux()` policy file `/etc/selinux/targeted/policy/policy.33` থেকে পড়ে `/sys/fs/selinux/load`-এ write করে এবং enforce file-এ `1` লিখে। Missing file, open failure, write failure বা enforcing bit failure হলে warning দিয়ে boot চালিয়ে যেতে পারে। Developer target-এ permissive mode ইচ্ছাকৃত হলে target profile-এ explicit থাকবে; release mode-এ policy enforced হওয়ার কথা থাকলে load/enforcing failure stop-ship হবে। Kernel SELinux enabled কিনা, selinuxfs mounted কিনা, policy version compatible কিনা, enforcing status query করে verify করতে হবে। “policy file লিখেছি” মানেই kernel policy active নয়; “enforce file-এ লিখেছি” মানেই read-back mode enforcing প্রমাণ করে না।

`secilc` না থাকলে build script syntax verified বলে warning দেয়; release policy compile step-এ `secilc` বাধ্যতামূলক করা উচিত। Security CI-তে policy compilation, `neverallow` checks, contexts coverage, allow rules review এবং sample AVC test থাকবে। Runtime-এ service domains ও file contexts সত্যিই apply হয় কি না Linux/QEMU test দিয়ে যাচাই করতে হবে। Policy loading orderও গুরুত্বপূর্ণ: mount securityfs/selinuxfs, load policy, set enforcing, label runtime sockets/files, then start confined services—প্রতিটি ধাপে dependency থাকতে হবে।

## ১৪.৪ privileged surface ও device access

`/dev`-এ সকল device node app sandbox-এ expose করলে namespace isolation যথেষ্ট নয়। Per-app device access `nilrt` permission broker, SELinux labels, cgroup device controller বা binder-like broker boundary দিয়ে সীমাবদ্ধ করতে হবে। Camera app-কে camera device node, audio app-কে ALSA access দিতে হবে system service-mediated request দিয়ে, direct unrestricted device access নয়। Hardware service daemon root/capability-limited user হিসেবে চলবে; privileged IPC request-এ peer credentials ও permission checks থাকবে।

## ১৪.৫ acceptance criteria

- Mount success শুধু successful syscall/read-back-এর পরে report করা হয়।
- Required mount failure নির্ধারিত recovery/degraded state তৈরি করে।
- cgroup v2 সত্যিকারের mount ও controller activation test আছে।
- Release profile SELinux policy load এবং enforcing status যাচাই করে; failure fail-closed।
- CI-তে `secilc` বাধ্যতামূলক release dependency।
- service/app domains, file/socket labels এবং privileged device access automated test-এ যাচাই হয়।

---

# ১৫. `nilrt`: app isolation, UID/GID, permissions ও seccomp

## ১৫.১ Sandbox-এর বর্তমান ভিত্তি

`runtime/nilrt/src/sandbox.rs`-এ Linux namespace, private mount propagation, `chroot`/`pivot_root`, fresh `/proc`, app-private `/tmp`, hidden বা read-only `/sys`, `PR_SET_NO_NEW_PRIVS` এবং seccomp-ভিত্তিক isolation-এর foundation আছে। Source comment নিজেই বলে এটি defense in depth, escape-proof guarantee নয়; `/dev` shared হওয়ার residual risk-ও স্বীকার করে। এই threat-model honesty বজায় রেখে capability-কে ধাপে ধাপে কঠোর করতে হবে। Unit test mount plans যাচাই করে, কিন্তু বাস্তব Linux namespace lifecycle, privilege drop, seccomp restrictions ও device visibility আলাদা integration suite দাবি করে।

## ১৫.২ per-app UID allocation

`nilrt-launch`-এ deterministic hash-ভিত্তিক UID mapping, persisted registry এবং `NIL_APP_UID` override-কে test/debug mode-এ সীমিত করার কাজ যোগ হয়েছে। কিন্তু registry file load → candidate allocate → JSON write/rename sequence concurrent process-এর মধ্যে lock না করলে race condition সম্ভব। Atomic rename একা concurrent update serialize করে না। দুই installer একই old registry পড়ে একই candidate UID বেছে নিয়ে আলাদা temp file rename করলে এক mapping overwrite হয়ে অন্য app-এর UID mapping হারাতে পারে। Solution হলো cross-process lock, registry transaction এবং atomic+durable replace; conflict detect করে retry করতে হবে। Registry malformed/corrupt হলে silently empty map ধরে নতুন UID allocation করা উচিত নয়—corrupt backup/quarantine ও diagnostic লাগবে।

Hash-based mapping deterministic হলেও security boundary হিসেবে app ID collision resolution ও registry integrity জরুরি। UID range host system-এর existing user IDs-এর সঙ্গে collide করছে কি না, `/etc/passwd`-এর identities বা reserved system users বাদ দেওয়া হয়েছে কি না তা যাচাই করতে হবে। User namespace ব্যবহার করলে host UID mapping, subordinate UID configuration এবং namespace support semantics আলাদা হবে। App ID validation UID registry-র আগে করা হবে। Production environment variables দিয়ে app root, data root, registry path বা UID override change করা privilege escalation-এর সুযোগ তৈরি করতে পারে; config values privileged launcher-owned file থেকে validated হবে।

## ১৫.৩ process isolation ও permission enforcement

`SandboxConfig`-এ `strict_permissions` dev-এ false, `hide_sysfs` default true এবং optional SELinux context আছে। Production launch path নিশ্চিত করতে হবে strict permissions true-by-default, unless app package permission declaration এবং system policy দ্বারা explicit grant করা হয়। Development fallback release binary-তে accidentalভাবে সক্রিয় থাকার ঝুঁকি সরাতে build profile gating এবং runtime hard guard উভয় ব্যবহার করা উচিত। `storage.write`, `network`, `camera`, `microphone`, `location`, `telephony`, `bluetooth`, `notifications`, `clipboard`—প্রতিটি permission-এর semantics documented, grant UI এবং system broker test থাকতে হবে। Permission name থাকা মানে বাস্তব hardware resource access অনুমোদন নয়; broker path-এ caller ID, app signature/trust, grant state, user choice, timeout ও revocation যাচাই হবে।

## ১৫.৪ seccomp allowlist

বর্তমান maturity file seccomp allowlist (~110 syscall) উল্লেখ করে। একটি single broad allowlist সব apps-এর জন্য অতিরিক্ত ক্ষমতা দেয়। First stable version-এ app class অনুযায়ী policy profiles থাকতে পারে: pure UI, network client, media playback, camera consumer, developer tooling। Dangerous syscall block-এর test positive এবং negative দুই ধরনের: normal file/network/clock/memory syscalls expectedভাবে কাজ করে; `ptrace`, `kexec`, `reboot`, mount operations, `bpf`/raw privileged APIs, unneeded kernel interfaces blocked হয়। Architecture-specific syscall numbers আলাদা হওয়ায় ARM64 profile আলাদা compile/test করতে হবে। Unsupported architecture-এ default allow না হয়ে deny/launch failure হবে।

## ১৫.৫ filesystem root ও writable data

প্রতিটি app-এর immutable code/rootfs এবং private user data পৃথক থাকবে। Code path read-only; app data path app UID ownership-এ read-write; shared downloads/media user consent ও scoped interface দিয়ে থাকবে। Symlink/hardlink, bind mounts, `/proc/<pid>/root`, current working directory escape, mount propagation এবং path traversal tests দরকার। `chroot()`-এর পরে `chdir("/")` fix আছে বলে documented; তবু `pivot_root`/namespace setup failure-এ fallback কীভাবে আচরণ করে তা test করতে হবে। Critical mount fail হলে unisolated process চালানো যাবে না।

## ১৫.৬ failure mode

Security namespace unavailable, `setgroups` fail, UID/GID switch fail, `no_new_privs` fail, seccomp apply fail, rootfs permission invalid, SELinux context apply fail—এগুলোকে warnings দিয়ে ignore করলে app intended sandbox ছাড়া চলতে পারে। Production policy-তে security-critical step fail করলে app launch abort হবে। Developer simulation mode আলাদা, visibly marked এবং release package-এ disabled হতে হবে।

## ১৫.৭ acceptance criteria

- UID registry concurrent allocations-এ corruption/duplicate mapping তৈরি করে না।
- Privileged environment overrides release mode-এ ignored only with explicit diagnostics, or rejected; they never silently widen access.
- App ID/traversal tests pass; app cannot escape its rootfs via symlink, `..`, cwd, procfs or mount propagation.
- Security-critical namespace/seccomp/credential failures app launch abort করে।
- Permission grants and revocations are applied by a privileged broker, not just UI state.
- Per-app resource limits, device access restrictions and SELinux domains have Linux integration tests.
- No test labels a degraded or fake sandbox as production secure.

---

# ১৬. `nilpkg` ও `.nilax`: package trust, update এবং rollback

## ১৬.১ `.nilax`-এর trust model

`pkg/nilpkg`-এ signed package, Ed25519 signatures, SHA-256 integrity, trusted publisher key directory, atomic install/upgrade/rollback journal, cross-process lock এবং key revocation-এর ভিত্তি আছে বলে `docs/completion-checklist.md` নথিভুক্ত করে। এই capabilities-কে end-to-end app launch-এর সঙ্গে integrate করতে হবে। একটি `.nilax` হলো trusted source থেকে আসা package metadata ও payload-এর container; `manifest.json`-এ app ID, version, executable/payload hash, size, permissions, signature এবং runtime requirements থাকবে। Archive extraction-এর আগে path traversal, absolute path, symlink policy, duplicate entries, oversized payload, compression bomb, unsupported architecture এবং invalid signature checks করতে হবে।

## ১৬.২ publisher trust এবং self-signed key

প্যাকেজ নিজস্ব public key embed করলে সেই key-এর signature verify করা যেতে পারে, কিন্তু trust প্রতিষ্ঠিত হয় না। `nilpkg`-কে independently trusted publisher key চাই; কেবল archive-এর ভিতরে থাকা public key দিয়ে package install অনুমোদন করা যাবে না। Trust store-এর key import process-এ fingerprint display, user/administrator confirmation এবং revocation state দরকার। App store catalogue package download source হতে পারে, কিন্তু store response নিজেই trust policy bypass করতে পারবে না। Key rotation, compromised publisher key, revocation, expiry policy, offline install এবং signed metadata timestamp semantics নির্ধারণ করতে হবে।

## ১৬.৩ update transaction

Upgrade sequence: validate package → validate trusted signature → compare version/channel → stage payload → verify contents → apply permissions/schema migration metadata → atomically swap install directory → fsync parent → write journal commit → keep rollback candidate → cleanup old data after transaction confirmed. Crash injection প্রতিটি point-এ run করতে হবে। Atomic rename, journal এবং lock থাকলেও same filesystem requirement, cross-device rename errors, directory fsync, disk-full, permissions, Windows rename semantics এবং concurrent update conflict test দরকার। Rollback-এর পর current version, old version, app data migration compatibility এবং signature trust re-check হতে হবে।

## ১৬.৪ executable model

একটি বড় architectural প্রশ্ন: `.nilax`-এর executable কি compiled native ELF হবে, নাকি `.nib` bytecode `NilVM`-এ চলবে, নাকি package type অনুযায়ী দুই রকম artifact থাকবে? ADR-0005 অনুযায়ী signed archive-এর মধ্যে NilLang bytecode থাকা উচিত বলে design model নির্দেশ করে। কিন্তু current integration test-এর নাম “end-to-end” হলেও code source থেকে bytecode বানায়, `.nilax` pack/install/verify করে এবং installed payload-কে NilVM-এ directly load করে; constructed `_spec: LaunchSpec` বাস্তবে `nilrt-launch` দিয়ে launch করে না। এটি useful vertical-slice test, কিন্তু real end-to-end sandbox integration নয়। Architecture contract একবার ঠিক করে manifest-এ `runtime_kind`, `bytecode_format_version`, `minimum_runtime_version`, `architecture` semantics স্পষ্ট করতে হবে।

## ১৬.৫ package store ও dependency solving

বর্তমানে app store catalogue simulated বলে maturity file-এ উল্লেখ আছে। Store backend বানানোর আগে package trust ও install lifecycle stable করা উচিত। Catalogue metadata signed হবে; package archive hash catalogue hash-এর সঙ্গে match করবে; TLS network security সহায়ক হলেও signed metadata-এর বিকল্প নয়। Dependency graph cycles, version constraints, unavailable dependencies, rollback compatibility এবং offline cache define করতে হবে। “Install” button success তখনই দেখাবে যখন verified package disk-এ atomically install হয়েছে এবং launch check pass করেছে; simulated catalogue থেকে click করলেই install successful দেখানো যাবে না।

## ১৬.৬ acceptance criteria

- Invalid signature, embedded self-signed-only key, revoked publisher, hash mismatch এবং wrong architecture reject।
- Archive path traversal, symlink escape, duplicate critical metadata ও oversized payload reject।
- Install/upgrade/rollback crash injection-এ package missing/corrupt থাকে না।
- Concurrent package operation lock contention-safe।
- Install করা bytecode `nilrt-launch`-এর মাধ্যমে actual sandbox lifecycle-এ চালানো হয়।
- Store catalogue simulated হলে UI visible marker এবং no false installation claim।

---

# ১৭. NilLang compiler, bytecode ও VM-এর স্থিতিশীল contract

NilLang হলো OnuronOS-এর নেটিভ application language-এর ভিত্তি। একটি portable language platform তৈরি করতে lexer/parser, AST, type checking, compiler, bytecode format, VM, standard library, error messages, debug metadata এবং package compatibility—এগুলোকে consistent হতে হবে। বর্তমানে `runtime/nillang`-এ parser/AST/compiler/bytecode/VM-এর code আছে; `NilVM::render_scene()` declarative UI tree-এর textual representation তৈরি করে। এটা language prototype ও UI model test-এর জন্য ঠিক আছে, কিন্তু screen-এ actual pixels দেখানো বা Button press event execute হওয়া প্রমাণ করে না।

## ১৭.১ language specification

Grammar ও semantics README-তে narrative আকারে নয়, normative specification-এ লিখতে হবে। Variable declaration, types, functions, state declarations, imports/modules, struct definitions, component syntax, control flow, error behavior, numeric model, string encoding, async/lifecycle semantics এবং source compatibility rules নির্ধারণ করতে হবে। যে syntax compiler গ্রহণ করে না, documentation-এ তা available বলে দাবি করা যাবে না। Versioned grammar examples প্রতিটি release-এর সঙ্গে থাকবে। Invalid program-এর জন্য deterministic diagnostics: file/line/column, offending token, expected syntax এবং stable error category প্রয়োজন।

## ১৭.২ type system

String-valued state map দিয়ে শুরু করা যায়, কিন্তু production app framework-এ bool, integer, float, string, list, map, nullable/reference types, enum, function callbacks এবং component props-এর typed contract দরকার। Numeric overflow, divide-by-zero, Unicode indexing, float precision ও error propagation define করতে হবে। UI event handler কী return করতে পারে—void, state update, async operation—এটি language/runtime ABI-র অংশ। Type checking compile-time-এ যত সম্ভব error ধরবে; runtime-এ invalid state access crash না করে structured error দেবে।

## ১৭.৩ bytecode format

`.nib`/serialized bytecode format-এর magic, version, endianness, section table, code length, constants, source map, imports, capability declarations এবং optional signature binding স্পষ্ট করতে হবে। Loader untrusted input হিসেবে bytecode ধরে bounds checking করবে। Truncated section, integer overflow, recursive nesting bomb, invalid opcode, missing constant, oversized string এবং mismatched compiler/runtime version-এর tests দরকার। Package signature archive-কে authenticate করলেও bytecode parser memory safety বজায় রাখতে হবে।

## ১৭.৪ deterministic VM execution

`NilVM` বর্তমানে program-এর primary app নির্বাচন করে state variables initialize করে এবং UI elements recursive render করে string output তৈরি করে। Multi-app packages, imports, mutable state, callbacks, event dispatch, resources, errors ও lifecycle এখনও systematicভাবে test করতে হবে। VM-এ CPU instruction budget, recursion depth, memory limit, string/array cap এবং cancellation policy লাগবে যাতে untrusted application infinite loop দিয়ে system hang করতে না পারে। VM exceptions app boundary-তে propagate হবে; PID1 বা system shell terminate করতে পারবে না। Deterministic test vector একই bytecode input-এ একই state/scene output দেয় কি না নিশ্চিত করবে।

## ১৭.৫ standard library ও system capabilities

NilLang application সরাসরি POSIX device APIs বা Android Java APIs ব্যবহার করবে না। Framework standard library typed capability API দেবে; যেমন storage read, camera request, audio stream, network client, notification create, display frame, vibration এবং telephony operation। Runtime caller identity ও permission broker-এর মাধ্যমে operation execute করবে। Capability unavailable হলে `BackendUnavailable`/`UnsupportedOperation` error হবে—fake success নয়। Hosted mode-এ Android permission denied এবং native mode-এ missing Linux driver একই high-level API-তে পৃথক structured status হিসেবে প্রতিফলিত হবে।

## ১৭.৬ developer experience

`nilc check`, `nilc build`, `nilc fmt`, `nilc test`, `nilc inspect`, `nilc run --backend=qemu|hosted`-এর মতো command design করা যায়, তবে প্রতিটি subcommand বাস্তবে কার্যকর হওয়ার পরে documented হবে। Editor integration-এর জন্য LSP, syntax highlighting, formatter এবং diagnostics JSON protocol পরে যোগ করা যায়। প্রথম priority হলো compiler/runtime semantics স্থিতিশীল করা। একটি “Hello app” build/install/launch test প্রতিটি commit-এ চলবে। তারপর button updates, text input, screen navigation, async network response এবং permission prompt-এর vertical slices যোগ করা উচিত।

## ১৭.৭ acceptance criteria

- Language spec, compiler acceptance tests এবং negative tests একই revision-এ versioned।
- Bytecode parser corrupt input-এ panic/out-of-bounds হয় না।
- VM instruction/resource limits enforced।
- State mutation ও UI callbacks actual executable, শুধু serialized text নয়।
- `.nilax` থেকে installed payload `nilrt-launch`-এ load হয়।
- Compiler and VM version mismatch clear diagnostic দেয়।

---

# ১৮. Alap framework-কে নকশা থেকে বাস্তব implementation-এ আনা

ADR-0009-এ Alap-কে high-level declarative mobile application framework হিসেবে সংজ্ঞায়িত করা হয়েছে: state/reactivity, app lifecycle, component graph, navigation/router এবং system-service contracts। একই নথিতে flow দেওয়া আছে `.nil → nilc → Alap validation → bytecode → signed .nilax → nilrt sandbox → NilVM + Alap runtime → NilUI compositor → NilHAL display`। এটি পরিষ্কার target architecture; কিন্তু বর্তমান repository tree-তে top-level `alap/` crate নেই। তাই Alap-এর দায়িত্ব, type definitions, implementation এবং tests আলাদা করে তৈরি করতে হবে। Documentation-এ “Alap framework ready” বলা চলবে না যতক্ষণ code ও acceptance evidence নেই।

## ১৮.১ Alap-এর API surface ছোট রাখুন

প্রথম release-এ কয়েকশ widget না করে minimal component set দিয়ে শুরু করা উচিত: `App`, `Column`, `Row`, `Text`, `Button`, `TextField`, `Image`, `Scroll`, `Spacer`, `Switch`, `Checkbox`, `Progress`, `Dialog`, `List`। প্রতিটির layout, semantic role, event model, state lifecycle, accessibility label ও error behavior নির্দিষ্ট করতে হবে। Component property naming, default values, layout constraints এবং invalid dimension rules stable হবে। নতুন component যোগ করার আগে existing components cross-backend equivalence tests pass করতে হবে।

## ১৮.২ reactive state model

`@State` declaration শুধু compile-time annotation হলে reactive behavior সম্পূর্ণ হয় না। কোন state পরিবর্তনে component tree invalidate হবে, update batching কিভাবে হবে, observers lifecycle-এর শেষে release হবে কি না, update loop/infinite invalidation কিভাবে ধরা হবে—এগুলো নির্ধারণ করতে হবে। UI thread-এর বাইরে state mutation কীভাবে marshal হবে, event loop priority কী এবং background event cancellation কেমন হবে তাও framework contract। State changes-এর deterministic test থাকতে হবে: initial render → button click → state update → next render expected value দেখায়।

## ১৮.৩ lifecycle

`onInit`, `onPause`, `onResume`, `onDispose` hook-এর semantics native ও hosted দুই mode-এ একই public contract পাবে। App background হলে timers, camera, microphone ও network requests বন্ধ বা pause হবে কি না permission/feature অনুযায়ী ঠিক করতে হবে। Surface destruction মানে UI renderer resources release; Activity pause মানে process অবশ্যই destroy হয়েছে এমন নয়। Process kill-এর পরে state restore, saved instance state, persistent state এবং ephemeral UI state আলাদা করা জরুরি। Lifecycle callbacks idempotent হওয়া দরকার; `onDispose` দ্বিগুণ ডাকলেও double-free ঘটবে না।

## ১৮.৪ navigation, input এবং error boundary

Router screen stack পরিচালনা করবে, hardware/software back button semantics এবং app-level navigation একীভূত করবে। Dialog ও permission prompt focus trap ও cancel behavior সহ কাজ করবে। Crashy child screen parent shell-কে crash করতে পারবে না। App error boundary fatal VM error-কে user-friendly screen/diagnostic বানাবে, sensitive input log করবে না। Navigation transition background/resume-এর সঙ্গে compatible হবে; graphics buffer lifecycle route change-এর সঙ্গে race করবে না।

## ১৮.৫ system-service wrappers

Alap-এর `Camera`, `Audio`, `Power`, `Network`, `Notifications`, `Telephony` API typed request/response ব্যবহার করবে এবং permission broker-এর মাধ্যমে caller identity পৌঁছাবে। API-এর `connectWifi()` শুধু local state true করে success return করবে না; actual association established না হলে operation status pending/failed হবে। `dial()` Android host-এ dialer খুলতে পারে, কিন্তু active call state host callback ছাড়া `Connected` হবে না। Camera preview request accepted হওয়া আর first valid frame পাওয়া আলাদা events হবে।

## ১৮.৬ implementation milestone

প্রথমে `alap-core` runtime crate, component model, state store, lifecycle contract ও unit tests তৈরি হবে। এরপর compiler parser/AST-তে Alap component graph validate হবে। তারপর `nilui`-এর scene graph-এ mapping হবে। এরপর button/textfield input event route, hosted backend, native renderer এবং package pipeline integrate হবে। `Alap` name ব্যবহার করে mock interface only থাকলে tier হবে experimental; একবার end-to-end sample app QEMU ও Android-hosted mode-এ tested হলে functional prototype বলা যাবে। Native physical device support আলাদা milestone।

## ১৮.৭ acceptance criteria

- `alap/` বা workspace-এ documented Alap crate বাস্তবে আছে এবং CI-তে build হয়।
- State change scene update করে; actual UI event handler execution tests আছে।
- Lifecycle pause/resume/dispose idempotent and leak-tested।
- Component graph semantic validation errors source location-সহ আসে।
- System APIs fake backend ও real backend আলাদা ফল দেখায়।
- একই sample app QEMU ও Android-hosted mode-এ একই component semantics বজায় রাখে।

---

# ১৯. NilUI এবং NilLang → UI → compositor pipeline

## ১৯.১ scene tree আর render output এক নয়

`runtime/nillang/src/vm.rs`-এর `render_scene()` বর্তমানে `UIElement`-কে `<Column>`, `<Text>`, `<Button>`-এর মতো textual representation-এ serialize করে। এটি compiler/VM test-এ দৃশ্যের কাঠামো আছে কি না যাচাই করতে সাহায্য করে। কিন্তু textual markup compositor-এ পাঠানো, layout গণনা, glyph rasterization, pixel buffer পূরণ, display presentation এবং input hit-testing—এগুলো ভিন্ন ধাপ। তাই “VM renders scene” কথাটি documentation-এ কোন স্তর বোঝায় তা লিখতে হবে। `scene serialization complete`, `layout complete`, `frame rendered`, `presented to host surface`, `user input delivered` আলাদা state হওয়া উচিত।

## ১৯.২ backend-neutral UI model

NilUI-তে widget declaration, layout constraints, visual style, semantic/accessibility properties এবং event bindings-এর platform-neutral representation থাকা উচিত। Renderer code-এ direct Android APIs বা `/dev/dri` path ঢুকে গেলে portability ভাঙবে; সেই কাজ NilHAL/display backend এবং platform adapter করবে। Scene node-এর stable ID, parent-child relationship, layout order, z-index, clipping, scroll state, focus ID এবং semantics tree থাকা দরকার। Render backend-এর output একই exact pixels হবে এমন নয়—font metrics ও device density-এর পার্থক্য আছে—তবে layout semantics, state change, focus order এবং button activation consistent হতে হবে।

## ১৯.৩ layout engine

প্রথম পর্যায়ে Column/Row-এর intrinsic measurement, min/max size, padding, margin, alignment, flex distribution, text wrapping, clipping, scroll viewport এবং screen density implement করতে হবে। Layout invalid input যেমন negative width, infinite dimension, nested scroll conflict অথবা zero-size view এলে deterministic behavior চাই। Layout test pixel screenshot-এর পাশাপাশি geometry snapshots-ও রাখতে পারে: node bounds, baseline, clipping region ও focusable rectangle। Responsive layout-এ logical dp/scale এবং physical pixels আলাদা রাখতে হবে। QEMU virtual display ও S25 host display-তে resolution/density runtime থেকে আসবে; 1080×2340 বা 120Hz constants demo fallback হিসেবে থাকলেও real capabilities বলে রিপোর্ট করা যাবে না।

## ১৯.৪ events এবং state

Button-এর declaration থাকা যথেষ্ট নয়। `onClick`, `onLongPress`, `onChange`, focus/blur, scroll, gesture এবং accessibility activation-এর event contract থাকতে হবে। Event loop UI thread-এ serialized হতে পারে; heavy network/file operation background executor-এ চলবে। State update render scheduling-এর সঙ্গে coherently bind হবে। App dispose হওয়ার পরে queued events drop হবে। Repeated touch, cancelled gestures, pointer ID reuse, multi-touch, keyboard input এবং stylus/pressure event tests দরকার।

## ১৯.৫ error boundary এবং performance

UI component exception, malformed scene এবং missing resource পুরো shell crash করবে না। Framework resource loading (fonts, images, icons), cache eviction, maximum tree depth এবং image dimension limits নির্ধারণ করবে। UI thread budget 60Hz-এ প্রায় 16.67ms, 120Hz-এ প্রায় 8.33ms; কিন্তু target device 120Hz সমর্থন করলেই সব frame সেই cadence-এ render হওয়া উচিত নয়। Render scheduler display refresh events ও workload-এর উপর ভিত্তি করে frame pacing করবে। Missed frames, dropped events, long tasks এবং layout thrash instrumentation-এ দেখা যাবে।

## ১৯.৬ acceptance criteria

- sample `.nil` app compile হয়ে scene model তৈরি করে।
- scene actual NilUI layout/raster pipeline-এ যায়।
- Button press scene callback execute করে visible state change ঘটায়।
- screenshot ও geometry snapshot test stability নিশ্চিত করে।
- no renderer/backend থাকলে explicit `BackendUnavailable` আসে; string scene-কে presented frame বলা হয় না।
- app crash boundary ও event cancellation test আছে।

---

# ২০. DRM/KMS renderer ও frame-presentation correctness

`runtime/nilui-gpu`-তে DRM device, dumb buffer, compositor, rasterizer, touch compositor ও `KmsPresenter`-এর মতো components আছে। `present.rs`-এ তিনটি buffer slot rotate করা হয় এবং pixels hardware buffer-এ copy করে `page_flip()` চালানো হয়। কিন্তু `page_flip()`-এর result বর্তমান code-এ ignore করা হয় (`let _ = ...`)। ফলে failed page flip-এও frame counter বেড়ে যেতে পারে এবং presenter success-like stats ফেরত দিতে পারে। প্রথমত real device-এ open/atomic modeset/plane/page-flip calls error-returning হতে হবে; second, present acknowledgement frame status-এর অংশ হতে হবে; third, virtual/fake backend-কে আলাদা report করতে হবে।

## ২০.১ DRM device open এবং virtual fallback

`DrmDevice::open_or_virtual()` নাম থেকেই fallback behavior আছে। Developer host-এ hardware না থাকলে virtual presenter rendering tests চালাতে সহায়ক। কিন্তু native release path-এ `/dev/dri/card0` open না হলে virtual display-তে successful boot বলে physical UI available দাবি করা যাবে না। Backend discovery-তে card node permissions, DRM master, connector connected state, mode list, plane formats, dumb buffer support এবং page-flip capability যাচাই করতে হবে। Permission denied ও device absent আলাদা error। Headless QEMU boot target renderer start না করেও OS core boot পাস করতে পারে; graphical smoke আলাদা job।

## ২০.২ pixel format ও framebuffer memory

PixelBuffer-এর RGBA/BGRA/ARGB representation, stride, bytes-per-pixel, gamma/color space, endian assumptions এবং DRM format matching নির্ধারিত থাকা উচিত। `copy_from_slice` length mismatch panic ঘটালে controlled error হবে। Buffer layout `width × height` মাত্র নয়; hardware line stride অনেক সময় padded হয়। Device resolution change বা rotation ঘটলে existing buffer dimensions stale হতে পারে; surface change frame scheduler-এর সঙ্গে synchronized হতে হবে। Triple buffering-এর slot rotation test slot number সঠিক দেখাতে পারে, কিন্তু front/back ownership বা vblank synchronization ঠিক কিনা প্রমাণ করে না। Real page flip complete event এবং buffer reuse timing যাচাই করতে হবে।

## ২০.৩ frame pacing ও metrics

বর্তমান `present_frame()` elapsed wall duration থেকে fps গণনা করে। One-time slow frame, scheduler noise এবং startup warm-up-এর জন্য average fps misleading হতে পারে। Rolling histogram, p50/p95/p99 frame time, dropped frames, GPU/CPU raster time, page-flip wait এবং queue depth record করতে হবে। `dropped_frames` estimate-কে precise dropped count হিসেবে না দেখিয়ে estimated metric label করতে হবে। Thermal throttling ও power modes performance metrics-এর সঙ্গে correlation করা যায়, কিন্তু user privacy বজায় রাখতে raw frame content logs-এ থাকবে না।

## ২০.৪ crash-safe renderer start

GPU/Vulkan initialization failure হলে QEMU/hosted developer mode software renderer fallback ব্যবহার করতে পারে, কিন্তু fallback visible status ও logs থাকা জরুরি। Native phone release candidate-এ unsupported GPU backend বা software fallback performance threshold-এর বাইরে হলে feature unsupported/experimental থাকবে। Buffer allocation, page flip, resolution switch, display unplug/reconnect, screen suspend/resume এবং compositor restart tests প্রয়োজন।

## ২০.৫ acceptance criteria

- DRM ioctl failures propagate as structured errors; ignored page-flip failure নেই।
- virtual renderer success physical backend success থেকে আলাদা।
- stride/pixel-format/dimension checks আছে।
- page-flip completion হওয়ার আগে buffer reuse হয় না।
- resolution/rotation/suspend/resume integration tests আছে।
- screenshot or captured frame checksum-এর সঙ্গে frame counter ও present status মেলে।

---

# ২১. Input, gesture, focus ও accessibility pipeline

UI pipeline-এর অপরিহার্য অংশ touch/input। Android-hosted mode-এ Java `MotionEvent` JNI bridge-এ পাঠানো হচ্ছে, native mode-এ Linux evdev/input subsystem থেকে event আসবে। দুই ক্ষেত্রের event model consistent হতে হবে: pointer down/move/up/cancel, keyboard, focus navigation, scroll, multi-touch, pressure এবং coordinate transformation। Raw screen coordinates আর logical dp coordinate আলাদা; resolution change, screen inset, status/navigation bars এবং rotation-এর পরে mapping update করতে হবে।

## ২১.১ Android touch bridge

`MainActivity.OnuronView` event action, pointer ID, X/Y ও pressure host event হিসেবে পাঠায়; তবে touch action শুধু action-down-এ app UI hit area-তে local handler চালায়। ফলে Rust bridge-এ events পাঠানো আর Rust-driven UI state machine input consume করা আলাদা path। ভবিষ্যতে একই tap Java Canvas-এ locally action করতে পারে কিন্তু NilUI scene graph-এ deliver নাও হতে পারে। প্রথমে একটি single authoritative event router দরকার। Hosted UI যদি এখন Java Canvas-এর own hit areas দিয়ে চলে, সেটিকে “hosted shell screen” বলা যাবে; NilUI event-driven app runtime আলাদা mode হবে। Native app framework চালু হলে duplicate event handling বন্ধ করতে হবে।

## ২১.২ Linux evdev integration

Native target-এ `/dev/input/event*` থেকে touch events পড়তে হবে এবং device calibration, multitouch slots, pressure range, coordinate transform ও SYN_REPORT semantics support করতে হবে। Device-specific touch controller mapping `fajita` profile-এ থাকবে। Permission policy-তে apps সরাসরি evdev read করবে না; input daemon raw events সংগ্রহ করে compositor/framework-কে send করবে। Gesture recognition (tap, double-tap, long-press, swipe, drag, pinch) raw events-এর উপর deterministic thresholds ব্যবহার করবে। High latency বা dropped events detect করতে timestamps preserve করা দরকার।

## ২১.৩ focus, keyboard এবং accessibility

TextField ও button-এর input accessible হতে screen reader semantics, focus order, keyboard/D-pad navigation, scalable font, high contrast, reduced animation এবং minimum target size দরকার। Touch-first phone হলেও keyboard, external device ও testing automation সমর্থন করা উচিত। Each component accessibility label, role, state এবং actions semantics tree-তে expose করবে। Bengali text shaping, ligature, bidi text (যদি থাকে), emoji fallback এবং font fallback UI tests-এ থাকবে। Accessibility API ছাড়া UI শুধু সুন্দর হতে পারে, কিন্তু ব্যবহারযোগ্য system UI নাও হতে পারে।

## ২১.৪ acceptance criteria

- Android-hosted touch input native event path-এ duplicate/missed activation তৈরি করে না।
- Linux evdev event sequence test recorded fixture দিয়ে pass।
- Screen-size/rotation changes coordinate mapping update করে।
- focus traversal ও text input automated test আছে।
- input permissions least privilege অনুসরণ করে এবং apps raw device access পায় না।

---

# ২২. Samsung S25 hosted runtime: APK/NDK/JNI contract

## ২২.১ Track B-এর বাস্তবতা

S25 track-এ OnuronOS Android application-এর ভিতরে চলে। Android process, UI toolkit/Canvas, permission system, device drivers এবং underlying OS Android-ই থাকে। এই architecture demo, developer preview ও hosted capability integration-এর জন্য কার্যকর। তবে এটি native OS replace, standalone boot বা Android compatibility container-এর প্রমাণ নয়। `docs/s25-hosted-runtime.md` এবং ADR-0001/0007-এ এই পার্থক্য স্পষ্ট রাখা হয়েছে; code/UI/release text-এও রাখতে হবে।

## ২২.২ build-ndk integration

`android-host/build-ndk.sh` এবং PowerShell counterpart `libandroid_host.so` cross-compile করে `app/src/main/jniLibs/arm64-v8a`-এ stage করে। Gradle `jniLibs` folder package করতে পারে; কিন্তু staging script Gradle build-এর সঙ্গে automatically wired না থাকলে developer পুরোনো বা missing `.so`-সহ APK বানাতে পারে। Build task graph-এ preBuild dependency হিসেবে NDK build চালানো যায়, তবে Android Gradle Plugin task configuration-এর সঙ্গে robust way-তে integrate করতে হবে। বিকল্প হলো CI আলাদাভাবে `build-ndk` → verify hash/ELF/JNI symbols → Gradle assemble করে। গুরুত্বপূর্ণ হলো stale shared library silently use না হওয়া।

## ২২.৩ JNI ABI versioning

`NativeBridge.getProtocolVersion()` এবং Rust side `nativeGetProtocolVersion()` protocol version 1 ফেরত দেয়। Versioning দিয়ে compatibility gate শুরু হয়েছে, কিন্তু contract-এর details প্রয়োজন: major/minor version, feature flags, command/event schema versions, maximum JSON frame size, encoding, threading context, ownership/lifetime এবং failure semantics। Java library load fail হলে `isNativeLoaded=false` হয় এবং hosted mode pure Java-তে fallback করতে পারে; fallback mode explicit UI badge ও diagnostics-এ দেখা চাই। JNI symbol missing হলে protocol version `1` ফেরত দিয়ে compatibility fake করা উচিত নয়—load failure/ABI mismatch আলাদা error হবে।

## ২২.৪ command/event protocol

Guest-to-host command queue-তে dial/sms/camera/torch/brightness/volume/vibration/network telemetry-এর মতো payload আসে। JSON command validate করতে bounded input, valid fields, permitted IDs, command authorization, timeout এবং request correlation ID থাকা দরকার। Host-to-guest event-এর version, timestamp/monotonic time, source, result status এবং permission outcome থাকবে। Synchronous UI call-এ hardware operation block করা যাবে না। Command dispatch thread operation শুরু করবে এবং future/event callback return করবে। Queue unbounded হলে malicious/faulty app memory exhaustion ঘটাতে পারে; max depth, backpressure, coalescing telemetry এবং command priorities দরকার।

## ২২.৫ Android build verification

CI-তে Android SDK/NDK, JDK এবং Gradle wrapper version pin করতে হবে; dependencies checksum/verification metadata, Gradle dependency verification এবং build cache policy বিবেচনা করতে হবে। APK assemble হওয়ার পরে `unzip -l` দিয়ে `lib/arm64-v8a/libandroid_host.so` আছে কি না, ELF AArch64 কিনা, JNI exported symbols JNI names-এর সঙ্গে মেলে কি না, `AndroidManifest.xml`-এ permission declared কি না এবং APK signing config test mode/official mode আলাদা কি না যাচাই হবে। তারপর emulator smoke test launch, native load, bridge start, event flow এবং Activity lifecycle test করবে। Emulator camera/network behavior physical S25-এর সম্পূর্ণ বিকল্প নয়; hardware test report আলাদা।

## ২২.৬ acceptance criteria

- Clean CI checkout থেকে Rust `.so` build ও APK packaging হয়।
- APK-তে correct ABI library থাকে এবং manifest revision-এর সঙ্গে library revision মেলে।
- JNI protocol mismatch load time-এ clear failure দেয়।
- Native fallback mode UI-তে `HOSTED / NATIVE BRIDGE OFFLINE` ধরনের accurate status দেয়।
- Bridge command queue bounded; command result/event correlation test আছে।
- CI emulator launch/surface/lifecycle smoke pass; S25 physical tests আলাদা evidence-এ record করা হয়।

---

# ২৩. Android host lifecycle, permissions এবং privacy

Android 12+ runtime permission semantics, foreground service rules, background activity restrictions, OS battery optimizations এবং target SDK policy সময়ের সঙ্গে বদলাতে পারে। তাই hosted bridge-কে manifest-এ permission declare করেই “permission granted” ধরে নেওয়া চলবে না। `CAMERA`, `RECORD_AUDIO`, `SEND_SMS`, `CALL_PHONE`, `READ_PHONE_STATE` এবং notification/foreground service behavior user-grant এবং OS version অনুযায়ী vary করতে পারে। Permission denial feature-level result হিসেবে প্রকাশ করতে হবে; crash বা fake success নয়। Permission request context-aware হবে: user camera action করলে Camera permission prompt; background boot-এ অপ্রয়োজনীয় permission request করা যাবে না।

## ২৩.১ foreground service ও lifecycle

`OnuronBridgeService` battery/network receiver নিবন্ধন করে, command dispatcher চালায় এবং host bridge server শুরু করতে পারে। Service-এর lifecycle contract define করতে হবে: Activity visible, Activity paused, app background, screen off, process death, reboot এবং permission revocation-এ কী চলবে? Ongoing microphone/camera/telephony work foreground service requirement মেনে notification ও user-visible status প্রয়োজন হতে পারে। `startForeground` ব্যবহারের ক্ষেত্রে appropriate service type/permission এবং notification channel policy যাচাই করতে হবে। Android-version-specific behavior CI matrix-এ test করা উচিত।

## ২৩.২ sensitive logging

Phone number, SMS message, Wi-Fi SSID, local IP, camera frame বা audio sample log-এ রাখা যাবে না। Debug mode-এও redaction default হবে। `redactNumber()` function থাকলে সব logging path সেই function ব্যবহার করছে কি না audit করতে হবে; এক method redacted হলেই privacy নিশ্চিত নয়। Diagnostics-এ command type, latency, success/error category, backend name ও event count যথেষ্ট হতে পারে। User consent ছাড়া usage analytics বা telemetry server-এ পাঠানো যাবে না। OnuronOS-এর zero-telemetry philosophy README-তে থাকলে network capture/permission review দিয়ে যাচাই করা উচিত।

## ২৩.৩ notification ও background task

Android host notification create করতে হলে notification permission এবং channel creation এর পাশাপাশি notification visibility, content privacy, action authenticity ও lifecycle বিবেচ্য। SMS contents lockscreen-এ leak হতে পারে; default lockscreen content redacted রাখুন। Background dispatcher thread process killed হলে pending commands কোথায় থাকে? In-memory queue হলে durability guarantee নেই; command accepted কিন্তু operation complete হয়নি—এটি result হিসেবে জানানোর protocol প্রয়োজন। Retry করা safe কি না command-specific (vibration retry harmless, SMS retry duplicate send করতে পারে) policy আলাদা হবে।

## ২৩.৪ host security boundary

APK hosted architecture-এ Onuron runtime app sandbox-এর ভিতরে থাকে; তা Android OS permissions bypass করে না। JNI native library memory-safe Rust হলেও unsafe FFI, raw pointers, frame-size parsing এবং JSON command dispatch attack surface তৈরি করে। JNI entry points-এর input length check, null pointer check, lifecycle race handling এবং lock poisoning recovery থাকবে। Host service `exported=false` রাখার মতো manifest setting ভালো, কিন্তু explicit Android intent routes/receiver exported states audit করতে হবে। Sensitive command (SMS/phone) action request-এ UI authorization, permission check এবং explicit user confirmation আছে কি না test করা দরকার।

## ২৩.৫ acceptance criteria

- Permission denied/revoked হলে typed error এবং UI status।
- Camera/microphone/telephony operation explicit user action ও correct runtime permission ছাড়া start হয় না।
- Background/foreground lifecycle transitions leak বা duplicate dispatcher তৈরি করে না।
- logs-এ raw SMS/phone numbers/audio/video content থাকে না।
- command retry semantics duplicate SMS/call action ঘটায় না।
- Android API level matrix-এ APK launch, permission flow ও foreground operation tests pass।

---

# ২৪. Android display: Rust frame buffer সত্যিই পর্দায় দেখানো

বর্তমান `android-host/src/display.rs`-এর `present_frame()` pixel buffer JNI bridge-এর global frame queue-তে প্রকাশ করে এবং frame counter বাড়ায়। `MainActivity.java`-তে host `OnuronView` একটি Java Canvas দিয়ে interface আঁকে; `onSizeChanged()`-এ `NativeBridge.onSurfaceChanged(null, w, h)` call করা হয়েছে, কিন্তু null Surface পাঠালে native ANativeWindow/Surface presentation path বাস্তবে কী করে তা verify না করা পর্যন্ত end-to-end display claim করা যাবে না। UI Canvas-এ Onuron-like screen দেখা এবং Rust renderer-এর pixel buffer Android Surface-এ উপস্থাপন—দুটি আলাদা।

## ২৪.১ একক rendering authority নির্ধারণ

প্রথমেই সিদ্ধান্ত নিতে হবে S25 prototype-এর current UI কোথায় আঁকা হবে। Option A: Java `Canvas` prototype, Rust HAL শুধু telemetry/command engine; Option B: Rust-এ scene rasterize করে pixel buffer JNI-এর মাধ্যমে host `Surface`/`Bitmap`-এ পাঠায়; Option C: native Android NDK `ANativeWindow_lock/post` দ্বারা frame display; Option D: OpenGL/Vulkan SurfaceView path। সব option একসঙ্গে অর্ধেক চালু রাখলে visual state দু’জায়গায় আলাদা হয়ে যাবে। বর্তমান prototype-এ Java Canvas UI থাকলে সেটিকে “hosted UI prototype” বলা উচিত; Rust pixel pipeline separate feature flag-এ পরীক্ষিত হবে।

## ২৪.২ surface lifecycle

Android `Surface` creation, changed, destroyed events-এর পরে native resource lifetime ঠিক করতে হবে। `nativeSurfaceChanged(null, ...)` যদি API contract-এ valid না হয়, actual Surface object pass করতে হবে অথবা size-only function আলাদা করতে হবে। `Surface` destroyed হলে ANativeWindow reference release, render thread cancel এবং frame queue clear/drop করতে হবে। Activity paused হলে frame scheduling suspend এবং resume হলে surface reacquire করা দরকার। Multi-threaded renderer stale surface pointer ব্যবহার করলে crash/use-after-free risk থাকে; Rust unsafe FFI boundary-তে ownership model explicit করতে হবে।

## ২৪.৩ image data path

`nativeGetLatestFrame(int[] outPixels, int maxLen)`-এর contract dimension, stride, premultiplied alpha, color order এবং buffer size নির্ধারণ করবে। 1080×2340 frame-এ প্রায় 2.5 million pixels; 32-bit প্রতি pixel হলে এক frame প্রায় 10 MB। 60 fps-এ unoptimized copy path bandwidth ও GC pressure তৈরি করতে পারে। তাই polling every frame-এর বদলে dirty-frame signal, shared/direct buffer, Surface lock/post অথবা GPU texture sharing বিবেচনা করতে হবে। প্রথমে correctness tests, পরে optimization। Pixel data JNI থেকে Java int array-তে copy হলে `maxLen` validation ও length mismatch error থাকা দরকার।

## ২৪.৪ UI screenshot regression

একটি deterministic scene (solid colors, checkerboard, text glyphs, touch button) renderer-এ feed করে host frame bytes capture করতে হবে। Test verify করবে dimensions, orientation, color channels এবং frame sequence number. Java Canvas output এবং Rust frame output দুটো তুলনা করতে হলে expected divergence-এর list তৈরি করতে হবে; “কিছু দেখা যাচ্ছে” test নয়। Device density/refresh rate dynamic নিতে হবে; hard-coded 120Hz S25 spec fallback only when actual telemetry unavailable. Hardware display refresh support reported API value থেকে আসতে হবে।

## ২৪.৫ acceptance criteria

- Rust-generated test frame S25 emulator এবং physical target-এ দৃশ্যমান।
- Actual Surface reference pass হয়; null surface misuse হয় না।
- Surface destroy/recreate/pause/resume tests crash/leak-free।
- Pixel format, stride, dimensions এবং frame count consistent।
- UI rendering authority documentation-এ স্পষ্ট।
- Native presentation unavailable হলে Java Canvas prototype-কে native display backend বলে দাবি করা হয় না।

---

# ২৫. Camera pipeline: simulated frame বনাম real capture

`android-host/src/camera.rs`-এ `AndroidHostCamera::capture_frame()` active camera না থাকলে error দেয়, কিন্তু camera open থাকলে host frame queue-তে valid JPEG frame পাওয়া না গেলে test-pattern JPEG ফেরত দিতে পারে। এই behavior unit tests-এর জন্য উপকারী: consumer image parser valid JPEG handle করতে পারে কি না পরীক্ষা করা যায়। কিন্তু একই fallback production UI-তে real photo বলে দেখালে feature status ভুল হবে। ADR-0004 সঠিকভাবে fake/simulated backend-কে explicit distinguish করার নীতি দেয়; camera path-এ সেটি বাধ্যতামূলক করা দরকার।

## ২৫.১ Camera2 pipeline-এর ধাপ

একটি বাস্তব capture-এর lifecycle: runtime permission → camera ID enumeration → selected lens ID validation → camera device open → capture session configuration → `ImageReader`/surface creation → preview stream → capture request → image frame delivery → JPEG/YUV format validation → timestamp/rotation metadata → resource release। `CameraManager.getCameraIdList()` শুধু ID দেয়; sensor metadata, supported output sizes, AF/AE modes, camera permission এবং hardware availability আলাদা করে যাচাই করতে হবে। Host command `CapturePhoto` queue-তে গেলেই capture complete নয়; asynchronous success/error callback এবং request ID দরকার।

## ২৫.২ fake frame handling

Fake backend-এর frame-এ metadata `is_simulated=true`, `source=test_pattern`, deterministic pattern hash এবং no-camera-device reason থাকবে। App screen-এ demo viewfinder থাকলে দৃশ্যমান SIM badge থাকবে। User “capture” চাপলে “Demo frame generated” অথবা “Camera backend unavailable” দেখাতে হবে, “Photo saved” নয়। Unit tests fake backend ধরে চলতে পারে, তবে integration tests `CAMERA` permission denied, camera unavailable, capture timeout, interrupted Activity, empty/invalid JPEG এবং queue overflow যাচাই করবে।

## ২৫.৩ actual frame integrity

JPEG validate করতে কেবল SOI/EOI marker যথেষ্ট নয়; decoder পর্যন্ত parse test দরকার। Camera2 ImageReader থেকে ByteBuffer copy করার সময় buffer position/limit, plane stride, row padding এবং image close করা হয়েছে কি না নিশ্চিত করতে হবে। High-resolution frames memory-heavy; bounded queue এবং latest-frame preference/consumer backpressure দরকার। Real capture time stamp monotonic clock-এর সঙ্গে bind হবে। Saving path user-consented storage API দিয়ে; file is written atomically; app permissions and media visibility semantics clear থাকবে।

## ২৫.৪ native camerad-এর সীমা

`services/camerad`-এ Linux V4L2 sensor discovery আছে বলে maturity file বলে; sensor পাওয়া মানে photo pipeline working নয়। Native device-এ media controller graph, ISP, libcamera/vendor tuning, supported pixel formats, camera calibration, autofocus, flash, power sequencing এবং sensor firmware লাগতে পারে। OnePlus 6T hardware matrix-এ camera এখনও planned; তাই Android Camera2 success-কে Linux V4L2 native camera success হিসেবে re-use করা যাবে না। Shared high-level camera API থাকতে পারে, কিন্তু backend maturity আলাদা।

## ২৫.৫ acceptance criteria

- Real capture path-এ `is_simulated=false` কেবল validated host frame completion-এর পর।
- Test pattern explicitly marked and never saved/represented as live camera capture. 
- Permission, no-camera, busy camera, timeout, malformed frame ও lifecycle interruption tests আছে।
- Frame queue bounded and no leak; all Android `Image` objects close করা হয়।
- physical S25 camera test evidence-এ model, Android build, test ID ও result থাকবে।
- Native `camerad` maturity hardware test না হওয়া পর্যন্ত planned/prototype থাকে।

---

# ২৬. Audio pipeline: PCM, AudioTrack/AudioRecord ও focus

`android-host/src/audio.rs`-এ PCM samples-এর queue/buffer management আছে এবং `services/audiod`-এ routing/focus policy ও ALSA PCM parser যুক্ত হয়েছে। তবে architecture design আর real audible output-এর মধ্যে বহু step রয়েছে: Rust queue → JNI command/event → Java `AudioTrack` playback loop → audio focus/lifecycle → hardware output; record path-এ `AudioRecord`/AAudio input → sample format conversion → Rust buffer → consumers। এই path end-to-end test না হলে Audio HAL-কে “real audio” বলা যাবে না।

## ২৬.১ PCM contract

Sample type (`i16` বা float), sample rate, channel count, interleaving, endianness, chunk duration এবং queue ownership formalize করতে হবে। 48kHz mono 16-bit audio-তে এক second-এ 96KB PCM, দুই second-এর 96,000 samples queue-র size calculation sample format অনুযায়ী ঠিক করতে হবে। Stero হলে memory double; 24-bit বা float path আলাদা। Buffer overflow হলে drop policy (oldest/newest), underrun হলে silence insertion, clock drift এবং timestamp handling define করতে হবে। Playback ও record stream queue separate রাখতে হবে।

## ২৬.২ host playback

Android service `AudioTrack` তৈরি করে selected audio format-এর সঙ্গে queue connect করবে। Audio focus request ও ducking lifecycle host Android rules অনুযায়ী হবে। Activity background হলে playback চলবে কি না app policy ও service type অনুযায়ী সিদ্ধান্ত নিতে হবে। AudioTrack initialization failure, device route change, headset disconnect, Bluetooth route, permission denial ও audio focus loss typed error/event হবে। Queue consume হচ্ছে কি না instrumentation-এ দেখা যাবে; Rust buffer growing indefinitely হবে না।

## ২৬.৩ microphone recording

Recording-এর আগে runtime `RECORD_AUDIO` permission দরকার। AudioRecord initialization, buffer size, device privacy indicator, app lifecycle, stop/release behavior এবং data retention policy নির্ধারণ করতে হবে। App UI-তে microphone active indicator দেখানো যায়; screen recording বা diagnostics-এ raw audio থাকবে না। Capture thread bounded real-time work করবে; blocking logs ও heap allocations এড়িয়ে চলবে। Native target-এ ALSA PCM path আলাদা; Linux audio card/codec present না থাকলে `BackendUnavailable` ফেরত দেবে।

## ২৬.৪ system audio policy

`audiod` stream focus priorities (EmergencyCall > VoiceCall > Alarm > Notification > Media) policy হিসেবে reasonable, কিন্তু real routing semantics implement/test দরকার। Call audio, ringtone, notification volume, media volume, accessibility audio এবং Bluetooth SCO/LE Audio behavior platform-specific. `audiod` alsa parser `/proc/asound/pcm`/sysfs থেকে devices discover করলেও `hw:CARD,DEV` open, parameters negotiate, write/read frames এবং xrun recovery না করলে playback path complete নয়। Software mixer test এবং actual hardware test আলাদা report করতে হবে।

## ২৬.৫ acceptance criteria

- Deterministic generated tone AudioTrack-এ বাজে এবং capture loopback বা tone detector test করে।
- Record path sample count, format, permission status ও timestamp verifies।
- Queue bounded, underrun/overrun metrics visible।
- Audio focus lost, route change, pause/resume ও shutdown tests pass।
- No microphone access without permission; no sensitive PCM in logs. 
- `audiod` status actual device path versus simulated backend স্পষ্ট করে।

---

# ২৭. Telephony, SMS, SIM, AT command এবং modem semantics

Telephony mobile OS-এ অত্যন্ত সংবেদনশীল subsystem, কারণ UI-তে dialer দেখানো বা AT command parser থাকা বাস্তব cellular voice/SMS service হওয়ার সমান নয়। `services/telephonyd`-এ AT command processor ও telephony model আছে; Android-host service-এ `ACTION_DIAL`, `SmsManager` এবং permission declarations-এর path আছে। কিন্তু এই তিনটি স্তর—simulation model, host Android action bridge, native modem control—আলাদা করে রাখতে হবে।

## ২৭.১ call lifecycle

Call states অন্তত `Idle`, `DialRequested`, `DialerPresented`, `Dialing`, `Ringing`, `Active`, `Held`, `Disconnected`, `Failed`, `Unknown` থাকতে পারে। Hosted Android-এ `Intent.ACTION_DIAL` সাধারণত user-confirmed dialer UI খুলে; এটি নিজে active call connect করে না। তাই Rust `AndroidTelephony::dial()`-এ সাথে সাথে `CallState::Active` করে call ID ফেরত দেওয়া হলে UI false positive তৈরি করতে পারে। Host থেকে actual telephony state/intent callback পাওয়া না গেলে result `DialerPresented` বা `NeedsHostConfirmation` হওয়া উচিত। Hangup/answer operation-ও permission, Android API restriction এবং default dialer role-এর উপর নির্ভর করতে পারে।

## ২৭.২ SMS semantics

`SmsManager` call throw না করলেই SMS recipient-এর কাছে পৌঁছেছে—এমন নয়। Sent/delivery result `PendingIntent` বা platform callback থেকে আসে; message id, request id, timestamp এবং state (`Queued`, `Sent`, `Delivered`, `Failed`, `Unknown`) রাখা উচিত। Command timeout হলে auto retry duplicate SMS পাঠাতে পারে; তাই idempotency key/operation ID এবং no blind retry policy দরকার। SMS permission runtime grant ও API restrictions test করতে হবে। User confirmation, destination display এবং send action auditability গুরুত্বপূর্ণ।

## ২৭.৩ AT parser এবং real modem

`process_at_command()`-এ `AT`, `AT+CPIN?`, `AT+CREG?`, `AT+CSQ`, `AT+COPS?`, `ATD`, `ATH`, `AT+CMGS`-এর মতো command response তৈরি হয়। যদি `AT+CREG?` স্থিরভাবে registered value ফেরত দেয় বা SIM/carrier/signal model hard-coded হয়, তা simulation. Real modem integration-এ serial port/device permissions, modem discovery, transport (AT serial, QMI, MBIM), unsolicited result codes, timeout/abort, command serialization, SIM PIN/error state এবং modem reset handling লাগবে। AT command standard-এর supported subset document করতে হবে; parser input injection, unsolicited line mixing এবং large response-র bounded handling দরকার।

## ২৭.৪ India cellular requirements

Physical India device-এ emergency calling, IMS/VoLTE, operator-specific modem firmware, carrier policy, SIM support, regulatory requirements এবং OEM modem stack complexity রয়েছে। A generic AT command can show signal or SIM state but it doesn't implement telephony. প্রথম native release-এ real call/SMS না চললে UI-কে dialer demo হিসেবে labelled রাখতে হবে; emergency call capability কখনও simulated হিসেবে advertised করা যাবে না। Real emergency call tests শুধুমাত্র legal/safe lab procedure ও controlled device testing-এর মধ্যে সীমাবদ্ধ থাকবে।

## ২৭.৫ acceptance criteria

- Call state host/modem callback থেকে update হয়; dialer launched-কে active call বলা হয় না।
- SMS success state Android/modem callback থেকে আসে; request queued ও delivered আলাদা।
- AT parser tests unsupported/malformed/oversized input reject করে।
- Carrier/SIM/signal data live source থেকে না এলে `simulated=true`/`unknown` status।
- modem permissions ও device nodes restricted; raw AT interface ordinary apps-এ exposed নয়।
- physical target matrix-এর voice/SMS tests evidence ছাড়া “functional” tier পায় না।

---

# ২৮. Network, Wi-Fi, DNS, Bluetooth ও system telemetry

`android-host/src/network.rs` এখনও hard-coded Wi-Fi/cellular state, sample SSID এবং sample access point data ব্যবহার করছে বলে বর্তমান অডিটে দেখা গেছে। অন্যদিকে `OnuronBridgeService.java`-তে `ConnectivityManager` callbacks থেকে live connectivity telemetry পাঠানোর code আছে। এই দুই স্তর sync না হলে Android side live event পাঠালেও Rust NilHAL UI-তে fake cached state দেখা যেতে পারে। ভবিষ্যতের architecture-এ `ConnectivityManager`/`WifiManager` হলো hosted backend-এর source of truth; Rust side event contract-এ actual status গ্রহণ করবে।

## ২৮.১ telemetry model

Network state-এ `connected`, `validated/internet_available`, `transport`, `interface`, `local_addresses`, `dns_servers`, `wifi_ssid` (permission/OS-available হলে), `metered`, `roaming`, `vpn_active`, `updated_at`, `source`, `is_simulated` থাকতে পারে। `connected=true` আর internet validated একই নয়। Link connected হলেও captive portal বা DNS failure থাকতে পারে। IP address, SSID এবং carrier sensitive local data; logs-এ redact/omit হবে। Host API restricted হলে field `unavailable` হবে, fake string নয়।

## ২৮.২ Wi-Fi scan/connect

Wi-Fi scan Android OS restrictions, location permission, nearby devices permission এবং throttling policy দ্বারা সীমাবদ্ধ হতে পারে। `scan_wifi()` এক sample AP ফেরত দিলে তা discovery নয়। Live scan-এর result timestamp, BSSID privacy handling, signal units ও security type সহ আসবে। Connect operation Android version-এর Wi-Fi provisioning API ও user confirmation require করতে পারে; direct silent credential injection general app-এ অনুমোদিত নাও হতে পারে। Framework API async state: request → pending → connected/failed/permission denied/timeout হবে। Password logs বা IPC debug output-এ কখনও থাকবে না।

## ২৮.৩ Native Linux networking

`netd`-এর Linux backend sysfs `/sys/class/net` থেকে interface `operstate` দেখতে পারে; এটি basic carrier/link status, IP, route, DNS বা internet reachability-এর সম্পূর্ণ model নয়। Actual network config interface-এ netlink, DHCP client, route management, resolv.conf/system resolver, Wi-Fi supplicant control এবং VPN integration প্রয়োজন। প্রথম milestone শুধু read-only telemetry হতে পারে; পরে configuration control। Link down state এবং missing sysfs আলাদা। `operstate=unknown` সব hardware-এ “up” বোঝায় না, তাই driver capability/source বিবেচনা করতে হবে।

## ২৮.৪ Bluetooth

`btd`-এ sysfs adapter discovery, pairing state ও scan discovery scaffold আছে। Real pairing requires BlueZ-compatible D-Bus বা dedicated HCI management, pairing confirmation, keys, trust policy, device address privacy এবং disconnect/reconnect handling. S25 hosted runtime-এ Android Bluetooth APIs use করা হবে; native Linux `btd` আলাদা backend। Pairing state local cache থেকে দেখানো যাবে না যদি stack state missing. Bluetooth audio (A2DP/HFP), BLE GATT এবং file transfer different capabilities; একটিকে implement করলেই সব Bluetooth “working” নয়।

## ২৮.৫ acceptance criteria

- Hard-coded SSID, IP, carrier, AP scan এবং fixed connectivity success production backend-এ সরানো বা simulated mode-এ সীমাবদ্ধ।
- Connectivity update Java host থেকে Rust client/UI পর্যন্ত test করা হয়।
- Wifi connect-এর success actual OS callback/connection state-এর আগে দেখানো হয় না।
- Native Linux network tests link, address, route, DNS এবং internet validation আলাদা করে।
- Bluetooth discovery/pairing/connection result বাস্তব backend থেকে এসেছে তা evidence-এ চিহ্নিত।
- SSID/IP/cellular information debug logs-এ leak হয় না।

---

# ২৯. Native phone port: OnePlus 6T `fajita`-র প্রস্তুতি

## ২৯.১ candidate বেছে নেওয়া আর port করা আলাদা

ADR-0007 OnePlus 6T (`fajita`)-কে initial native reference phone হিসেবে নির্বাচন করেছে এবং Samsung Galaxy S25-কে Track B hosted runtime target হিসেবে রেখেছে। একটি নির্দিষ্ট reference phone-এ ফোকাস করা ভালো, কারণ display, PMIC, modem, partitions এবং drivers প্রতিটি model-এ ভিন্ন। কিন্তু ADR বা `docs/evidence/oneplus-fajita/`-এ device identity, hardware matrix এবং recovery notes থাকলেই physical port সম্পন্ন হয় না। Current hardware matrix-এ boot/CPU, UFS, display, touch, battery, Wi-Fi/BT, audio, modem এবং camera এখনও `PLANNED`। তাই এটিকে “profile selected, bring-up not validated” হিসেবে পরিচয় দিতে হবে।

## ২৯.২ acquisition ও lab readiness

Hardware-এর আগে recovery environment প্রস্তুত করা উচিত। exact model/codename, region/variant, storage capacity, bootloader state, firmware build, Android version এবং partition map evidence হিসেবে record হবে। Stock recovery/firmware package অফিসিয়াল বা trusted source থেকে সংগ্রহ করতে হবে; hash store করতে হবে; bootloader unlock-এর data-wipe implications নথিভুক্ত করতে হবে। USB cable, known-good host machine, platform-tools version, power supply, backup storage এবং hardware serial log capture path পরীক্ষা করতে হবে। EDL/Qualcomm recovery tool “গ্যারান্টিযুক্ত” বলা যাবে না যতক্ষণ exact device variant-এ recovery procedure বাস্তবে যাচাই করা না হয়। Documentation-এর emergency recovery steps প্রকৃত hardware, cable/mode, signed package access এবং OEM limitations অনুযায়ী confirm করতে হবে।

## ২৯.৩ bring-up order

প্রথম goal UI নয়, kernel boot log। ধাপ হবে: unlocked bootloader verification → correct boot image format and DTB/DTBO → kernel entry → serial/early console → initramfs `/init` execute → `nilinit` PID 1 → essential mounts → storage read-only probe → serial diagnostic shell → persistent storage test → display output → touch input → battery/power → USB/network/Wi-Fi → Bluetooth → audio → modem/telephony → camera → suspend/resume/thermal testing। প্রতিটি ধাপের exit criteria থাকতে হবে এবং আগের ধাপ stable না হলে পরের subsystem-এর success claim করা যাবে না।

## ২৯.৪ community base ও kernel strategy

`fajita`-র জন্য community-maintained Linux/mobile ports থেকে kernel config, DTS/DTBO, driver status, firmware requirements ও known issues research করতে হবে। কিন্তু অন্য OS-এর kernel/config সরাসরি OnuronOS-এ কাজ করবে ধরে নেওয়া যাবে না। Onuron userland আলাদা হলেও kernel driver/DT integration-এর requirement একই device থেকে আসে; তবু boot image layout, initramfs, kernel command line ও userspace expectations matching করতে হবে। Kernel source branch, exact commit, patch series এবং license notices recorded হবে। Vendor firmware redistribute করা যাবে কি না license review হবে।

## ২৯.৫ device profile schema

`targets/oneplus-fajita/target.toml` বা সমতুল্য profile-এ অন্তত:

- `codename`, `marketing_name`, supported variants এবং known unsupported variants;
- SoC/board ID, architecture, kernel tree/revision, config hash, DTB/DTBO paths;
- boot image header version, page size, offsets, cmdline policy, ramdisk compression;
- boot/recovery/system/vendor/dtbo/vbmeta/super/metadata/userdata partition map ও slot support;
- required firmware blobs, licenses, source URL ও checksum;
- display, touch, storage, audio, Wi-Fi, Bluetooth, battery, modem, camera driver status;
- bootloader unlock procedure, stock backup path, restore procedure এবং recovery evidence;
- `flash_allowed=false` default until all safety conditions pass;
- supported install method, expected fastboot/fastbootd mode এবং rollback protocol.

কোনও unknown field default guess দিয়ে পূরণ হবে না। Device identifier mismatch, variant ambiguity বা incomplete profile থাকলে build/flash tool refusal দেবে।

## ২৯.৬ acceptance criteria

- Exact physical device identity ও firmware state documented এবং review করা।
- Stock backup এবং recovery procedure একটি আলাদা machine/backup storage-এ verified।
- Board-specific kernel, DTB/DTBO ও boot image format verified।
- Serial boot log `nilinit` পর্যন্ত পৌঁছায়; then storage/display/touch in separate milestones।
- প্রতিটি subsystem hardware evidence-সহ maturity update পায়।
- Generic QEMU kernel/image `fajita` profile দিয়ে flash করা যায় না।

---

# ৩০. Device profile, DTB, kernel config, partition map ও recovery

## ৩০.১ profile-এ magic constants ছড়িয়ে রাখা যাবে না

`build/mkbootimg.py`-তে `fajita`, `enchilada` এবং generic ARM64 profile-এর কিছু defaults আছে; flasher ও builder-এর মধ্যে এই তথ্য duplicative হলে drift হবে। একক profile registry-ই source of truth হতে হবে। `mkbootimg.py --profile fajita`-র default page size, base address, board নাম বা command line-এর logic tests-এ নির্দিষ্ট করা দরকার। Default base বা page size parameter explicitly set করা হয়েছে কি না argparse থেকে নির্ভরযোগ্যভাবে বোঝা কঠিন হতে পারে; profile selected হলে conflicting explicit args reject করার pattern বেশি নিরাপদ। Silent override নয়।

## ৩০.২ boot image version এবং boot chain

Android boot image header v0–v4 pack/unpack support থাকলেই সব ফোনে compatible হওয়া যায় না। Boot header version, vendor boot image, recovery partition, DTB placement, ramdisk fragments, AVB metadata, bootconfig এবং OEM bootloader expectations device generation অনুযায়ী আলাদা। `mkbootimg.py` generated file parse back করতে পারে কি না test useful; তবে standard Android tooling (`unpack_bootimg`, `avbtool`, device's bootloader behavior) দিয়ে independent compatibility verification দরকার। Tool-এর own create+parse test internal consistency যাচাই করে, actual bootloader acceptance নয়।

## ৩০.৩ partition map

OnePlus 6T A/B layout সম্পর্কে docs-এ `boot_a`, `boot_b`, `system_a`, `system_b`, `vendor_a`, `vendor_b`, `userdata` ইত্যাদি লেখা আছে। Actual device variants, dynamic partitions, `super`, `vbmeta`, `dtbo`, `vendor_boot`-এর উপস্থিতি এবং fastboot mode-এর partition access exact unit থেকে interrogate করতে হবে। Flasher hard-coded `fastboot flash boot` ও `fastboot flash system` করে থাকলে slot selection ও dynamic partition semantics match নাও করতে পারে। Target manifest-এর partition map-এ aliases, slot suffix policy, allowed writable partitions, required mode এবং size/format constraints থাকবে। `fastboot getvar all` output evidence হিসেবে রাখা যেতে পারে কিন্তু serials/unique IDs sanitize করতে হবে।

## ৩০.৪ recovery test

Recovery documentation-এ “EDL restores everything” বা “bootloader auto-rolls back after 3 failed boots” এর মতো নিশ্চিত দাবি শুধু source document থেকে নয়, exact device behavior দিয়ে verify করতে হবে। Bootloader version এবং slot metadata অনুযায়ী rollback আচরণ ভিন্ন হতে পারে। Test unit-এ stock boot image backup, test boot without flash if supported, boot slot fallback test, recovery partition entry, fastboot access recovery, exact firmware restore এবং user data backup/restore workflow যাচাই করতে হবে। Recovery process user data wipe করে কি না আগে থেকে স্পষ্ট হবে। Hardware device available না থাকলে recovery section status `procedure documented, not physically tested` থাকবে।

## ৩০.৫ firmware redistribution ও security

Proprietary firmware blobs system bundle-এ আনার আগে license/redistribution rights পরীক্ষা করতে হবে। CI artifact-এ private signing keys, bootloader unlock tokens, user-identifying logs বা private firmware mirror credentials থাকবে না। Firmware hash pin করা যাবে; but no secret credentials in repository. `vendor/firmware` packaging rules ও checksums আলাদা হবে। Kernel config এবং device tree source version control-এ রাখা যায় যদি license অনুমতি দেয়; generated DTB build output manifest-এ থাকবে।

## ৩০.৬ acceptance criteria

- All device-dependent constants live in versioned device profile, not scattered scripts.
- `mkbootimg --profile fajita` output matches profile and explicit conflicts fail.
- Exact partition map comes from verified hardware evidence.
- Fastboot command sequence validates target, slot, mode, partitions and image manifest before any write.
- Recovery procedure has tested steps or explicitly says untested.
- Firmware policy/legal provenance reviewed; secrets do not enter repository/artifacts.

---

# ৩১. Flashing safety, verified boot, AVB ও root of trust

## ৩১.১ flasher change-এর ইতিবাচক দিক ও অবশিষ্ট সমস্যা

`build/flash-device.sh`-এ connected fastboot device check, product identity lookup, generic ARM64 target block এবং userdata wipe opt-in এসেছে। এটি আগের চেয়ে নিরাপদ। কিন্তু script এখনও image validation, target profile correctness, partition layout, vbmeta semantics এবং actual AVB trust chain সম্পর্কে সম্পূর্ণ guarantee দেয় না। `--force-unsupported` gate bypass করতে পারে; developer-lab use-এ explicit unsafe override থাকতে পারে, কিন্তু release workflow-তে flag থাকা মানেই userকে risk বুঝিয়ে দেওয়া যথেষ্ট নয়। S25-এর জন্য কোনো native flash attempt এই project path-এ করা উচিত নয়।

## ৩১.২ validation বনাম signing বনাম verified boot

তিনটি ধারণা আলাদা:

**Integrity validation:** file checksum expected hash-এর সঙ্গে মেলে। এটি corruption শনাক্ত করে।  
**Authenticity signature:** trusted private key দিয়ে metadata/artifact sign, verifier trusted public key দিয়ে verify করে। Self-signed embedded public key নিজে root of trust নয়।  
**Bootloader-enforced verified boot:** bootloader/firmware device-এর pinned trust key এবং AVB/Dm-verity metadata verify করে kernel/system load করতে অনুমতি দেয়। Host-side custom script image verify করে থাকলেই bootloader সেই check enforce করছে—এমন নয়।

ADR-0006 `mkvbmeta.py`-র output-কে integrity descriptor prototype হিসেবে বর্ণনা করে; এই সততা বজায় রাখতে হবে। Production AVB claim করার আগে standard AVB 2.0 tooling, device-supported bootloader trust enrollment, vbmeta signature verification, rollback indexes এবং dm-verity root hash enforcement বাস্তবে connected হতে হবে।

## ৩১.৩ current flasher-এর stop-ship review

`flash-device.sh`-এ boot/system flash-এর আগে vbmeta presence বাধ্যতামূলক নয়; আগের revision-এও custom verify path ছিল কিন্তু latest diff-এ verification সরিয়ে দেওয়া হয়েছে। PowerShell script-এ `fastboot flash vbmeta --disable-verity --disable-verification ...` pattern রয়েছে, যা verified boot বন্ধ করে দিতে পারে। এই দুই script behaviour align করতে হবে এবং `--disable-verity` release flow থেকে সরাতে হবে। কোনও vbmeta artifact না থাকলে “continue without verification” নয়, flash reject হবে—unless profile specifically says test-only non-verified dev boot and script is named/labelled unsafe and cannot run as release. Script-এ success message “Flashed Successfully” শুধু `fastboot` return code success-এ দেখাবে; reboot command failure হলে release success final করা যাবে না।

## ৩১.৪ signing keys

OS signing root key, OTA metadata key, app publisher keys এবং development test keys আলাদা। OS private key repository, `.env`, build cache, public artefact বা generated `.gradle` directory-তে থাকবে না। Signing service HSM/secure storage বা protected CI secrets-এর মতো mechanism ব্যবহার করবে; secret read permission সীমিত থাকবে। Local developer build test-only key দিয়ে signed হলে `test_only=true` manifest এবং non-release label থাকবে। Key rotation, revocation, public key distribution, trust bootstrap ও emergency compromise handling policy থাকা দরকার।

## ৩১.৫ flashing preflight

Flasher-এর no-write dry-run default হওয়া উচিত। Preflight output: connected device product/codename, slot, bootloader mode, selected target profile, manifest target/revision, boot.img hash, system image hash, vbmeta hash/signature status, partition map, image sizes, planned commands এবং expected data-loss effects। User confirmation-এর আগে এই summary দেখাতে হবে। Device identity inconclusive, multiple device connected, wrong slot, wrong mode, missing backup, incompatible image header, invalid signature, missing target-specific DTB বা unverified recovery—এর কোনটি থাকলে flash block হবে। `--force-unsupported` release builds-এ not permitted হবে।

## ৩১.৬ AVB acceptance criteria

- Standard AVB structure validated using independent official tooling. 
- Device bootloader actually verifies expected trusted public key. 
- dm-verity/hash tree enforcement is enabled and runtime corruption test causes expected integrity failure.
- rollback protection and slot metadata connected to boot-control path.
- key generation, storage, rotation and revocation documented/tested.
- unsafe flags cannot be accidentally enabled in release bundle.
- flash dry-run, wrong-device, wrong-slot, corrupted image and missing key tests make zero writes.
- OnePlus physical flash not enabled until recovery process is independently tested.

---

# ৩২. A/B OTA, update transaction এবং rollback

`nilupd`-এর update logic-এ signed update manifests, A/B staged update ও rollback-এর কিছু foundation থাকতে পারে; কিন্তু actual boot-control/bootloader slot selection connect না হলে A/B state শুধু software bookkeeping। OTA pipeline-এ download হওয়া file, verified payload, inactive slot write, boot metadata switch, boot success confirmation এবং rollback count—সব platform বাস্তবতায় যুক্ত করতে হবে। প্রথমে QEMU-তে simulated boot-control backend implement/test করা যায়; production device backend ছাড়া native OTA `production` নয়।

## ৩২.১ update manifest

Signed manifest-এ target codename, current compatible version range, new build version, partition/component hashes, size, security patch level, minimum bootloader/firmware requirements, rollback index, required free space, signature key ID এবং install policy থাকবে। Package downloaded bytes SHA match, metadata signature verify, target profile match এবং version downgrade policy যাচাইয়ের পরে তবেই staging হবে। HTTPS transport দরকার হতে পারে, কিন্তু authenticity signed metadata-র উপর নির্ভর করবে। Manifest parser malicious sizes, integer overflow এবং unexpected path reject করবে।

## ৩২.২ A/B transaction

Typical flow: current slot confirmed good → write inactive slot → read-back/hash verify → update boot control to try inactive slot → reboot → first-boot health checks → mark slot successful → keep previous slot fallback until policy criteria pass. Boot failure count update metadata durable storage-এ রাখতে হবে। `nilinit` healthy boot decision আর bootloader slot success signal একই concept নয়; integration point explicitly specify করতে হবে। QEMU emulator-এ fake slot backend state machine test করা যাবে; physical target backend boot-control API/partition fields না থাকলে release criteria পাস নয়।

## ৩২.৩ power loss ও failure injection

Update test critical write-এর মাঝখানে process kill, disk-full, hash mismatch, interrupted network, signature invalid, corrupted inactive slot, first boot service failure, multiple consecutive reboots এবং version downgrade চালাবে। Previous known-good slot untouched থাকতে হবে। User data migration backward-compatible না হলে migration transaction versioned ও rollback-safe হতে হবে। System update যেন user data partition format না করে। Upgrade UI-তে estimate/percentage optional, কিন্তু “updated successfully” only after boot-control confirms successful boot. Download complete alone is not install complete।

## ৩২.৪ acceptance criteria

- Manifest trusted signature and target identity verified.
- Inactive image hash read-back matches before switching slot.
- Boot control writes are durable and recovery tested.
- Failed candidate boot reverts automatically within bounded attempts.
- User data remains preserved in normal update/rollback.
- QEMU fake backend labeled simulated; physical backend hardware evidence ছাড়া production নয়।

---

# ৩৩. Shell, launcher, Settings এবং demo data-এর সততা

সর্বশেষ commit-এ shell-এর Calculator, Notes, Music ও Camera screen এবং terminal command paths যোগ হয়েছে। এগুলো early UX design validate করতে সাহায্য করে। কিন্তু `shell/src/main.rs`-এ কিছু content স্থির বা seeded demo, Calculator expression parser অতি সীমিত, `python -c` path বাস্তব Python interpreter নয় বরং কিছু input-এ pseudo-output দিতে পারে, Notes screen fixed examples দেখায়, Music playlist static strings, Camera viewfinder sensor specs-কে literal হিসেবে দেখায়। এগুলোকে real app feature হিসেবে দেখানোর আগে functionality এবং persistence integrate করতে হবে। `docs/maturity.toml`-এর `simulated` marker UI-তেও স্থায়ীভাবে বজায় রাখতে হবে।

## ৩৩.১ terminal safe semantics

Terminal-এ `ls`, `cat`, `cd`, `calc`, `notes`, `music`, `camera` ইত্যাদি commands আছে। `cd` current directory string update করলেও path exists/directory কিনা validation না করলে prompt এমন directory দেখাতে পারে যেখানে user বাস্তবে ঢোকেনি। `cat`/`ls` path resolution sandbox root-এ confined হওয়া দরকার; relative `..`, absolute path এবং symlink escape test করতে হবে। `python -c` actual interpreter না হলে তার output “executed successfully” দেখানো বিশেষভাবে বিভ্রান্তিকর। দুটি সঠিক বিকল্প: embedded interpreter integration, অথবা feature unimplemented বলে স্পষ্ট error। Fake evaluation-কে Python execution বলা যাবে না। Terminal diagnostics-এ `services`, `ps`, `net`, `modem`, `audio`-র output system APIs থেকে আসবে; না হলে simulated badge সহ test data দেখাবে।

## ৩৩.২ Calculator

বর্তমান `eval_simple_math()` function simple string parsing করে plus/minus/multiply/divide হিসাব করে। Operator precedence, unary minus, nested parentheses, multiple operators এবং scientific notation সঠিকভাবে parse না করলে ভুল result সম্ভব। `rfind`-ভিত্তিক split expression semantics নির্ধারণ করে না; bytes indexing-ও Unicode expression-এ unsafe হতে পারে যদি char-boundary না রাখা হয়। প্রথমে supported grammar খুব ছোট হলে UI-তে তা বলা উচিত; production calculator চাইলে tokenizer + precedence parser বা vetted expression parser ব্যবহার করতে হবে, `eval` দিয়ে arbitrary Rust/Python code execute করা নয়। Division by zero, NaN/Infinity, overflow, rounding, locale decimal separator এবং Bengali digits input/output behavior test হবে।

## ৩৩.৩ Notes ও Music

Notes app-এর fixed strings পড়া যায় কিন্তু create/edit/save/search করলে persistent data path চাই। Notes database atomic write, schema version, backup/export এবং corruption recovery policy থাকা উচিত। Music player-এর playlist static metadata; actual playback না থাকলে Play/Pause action-কে playback status বলা যাবে না। Real media player-এ file picker, codec support, audio focus, position/seek, background playback, media control integration এবং error handling দরকার। প্রথম milestone offline local audio file decode/play; online streaming পরে authentication/licensing/network behavior অনুযায়ী।

## ৩৩.৪ Settings

Settings screen-এ Wi-Fi, cellular, display brightness, security, battery/storage lines থাকতে পারে। Control actual state mutate করে এবং host/native backend operation result পাওয়া না গেলে UI optimistic success দেখাবে না। Slider brightness adjust করলে permission/Android setting restriction state handle হবে। SELinux status kernel থেকে query করতে হবে; configuration file existence দিয়ে Enforcing দেখানো যাবে না। Storage encryption state filesystem/key state থেকে আসবে; static “encrypted” indicator নিষিদ্ধ। Settings fields source-of-truth, capability availability, last updated timestamp এবং unsupported reason ধরে UI state তৈরি করবে।

## ৩৩.৫ maturity taxonomy

`docs/maturity.toml`-এর tiers (`production`, `functional-prototype`, `experimental`, `stub`, `not-implemented`) ভালো ভিত্তি। কিন্তু `production`-এর অর্থ “real environment-এ hardened ও verified”—তাই physical target ready না হলে no rows in production থাকতে পারে। Each UI screen-এর `simulated` field এবং `ui_screens` mapping bidirectional consistency test করা উচিত। New screen যোগ করার PR-এ maturity record update বাধ্যতামূলক। UI screenshot-এর নিচে visible label/metadata থাকবে, বিশেষ করে fake battery, signal, weather, call history, app store catalogue, SoftBus peers, camera sensors এবং audio tracks-এর ক্ষেত্রে।

## ৩৩.৬ acceptance criteria

- No demo screen claims real sensor, connection, call or playback success without backend result.
- terminal path handling cannot escape permitted root; `cd` verifies paths.
- `python` feature either actual runtime integration or explicit unsupported output.
- Calculator accepted grammar documented and negative cases tested.
- Notes are persisted through actual storage layer before “saved” shown.
- `docs/maturity.toml` and shell UI simulation registry remain consistent in CI.

---

# ৩৪. System services, IPC, observability এবং debugging

## ৩৪.১ service inventory এবং ownership

`etc/nilos/services.toml`-এ required/optional service name, executable, restart policy, socket activation config এবং dependency metadata থাকতে হবে। প্রতিটি service-এর single responsibility এবং data ownership define করা দরকার: `nild` core service bus/daemon manager কী করে; `nilkeyd` key broker; `nilbus` internal IPC; `netd`, `audiod`, `powerd`, `telephonyd`, `camerad` hardware capability services; `nilshell` presentation shell। Service names duplicate হলে config reject হবে। `exec` parse ভুল হলে service silently drop নয়। Config schema versioned হবে এবং startup-এর আগে validate হবে।

## ৩৪.২ `nilprotocol` framing

Maturity file বলে canonical framed IPC-এর `ONUR` magic, versioned header, length-prefixed frame এবং 1 MiB payload cap আছে। এই wire protocol-এর field widths, byte order, checksum/integrity expectations, request ID, message type, version negotiation, maximum payload, streaming semantics, partial read/write এবং cancellation rules formal spec-এ রাখতে হবে। IPC socket-এর peer credentials verify করে service authority নির্ধারণ করতে হবে। Frame length attacker-controlled; allocation আগে cap check করতে হবে। Unknown message type, unsupported version, corrupted header, over-limit length এবং partial frame test আবশ্যক।

## ৩৪.৩ status/metrics endpoint

প্রতিটি daemon `/run/onuron/`-এ private socket বা system bus-এর মাধ্যমে health/status expose করবে। Health data-তে lifecycle, version, backend name, simulated status, device present, last successful operation, last error category, request count, queue depth, latency histogram এবং uptime থাকতে পারে। User data, secret tokens, SMS content, keys বা raw camera samples থাকবে না। System status bar-এর battery/network icons এই endpoint থেকে live data নেবে; fallback data হলে `SIM` marker থাকবে। Monitoring data-এর জন্য telemetry server প্রয়োজন নেই; local diagnostics যথেষ্ট।

## ৩৪.৪ logs

Kernel messages (`/dev/kmsg`), serial console, journal/logd files এবং app diagnostic log-এর policy আলাদা করতে হবে। Early boot-এ console unavailable হলেও ring buffer/log persistence plan দরকার। Logs structured fields (timestamp monotonic/UTC, boot ID, subsystem, severity, event code) ব্যবহার করবে। User-readable log message বাংলা/ইংরেজি হতে পারে, কিন্তু machine diagnostics stable event ID-তে নির্ভর করবে। Rate limiting repeated error storm এবং secret redaction দরকার। Rotating logs disk-full হলে boot বা system services break করবে না।

## ৩৪.৫ crash capture

`crashd`/`logd` নাম থাকলেই crash reporting complete নয়। Process exit code/signal, service restart count, panic summary, core dump policy, stack trace symbolization, build ID এবং artifact provenance record করতে হবে। Core dumps sensitive memory contain করতে পারে; permission/retention/encryption policy দরকার। Hosted Android mode-এ logcat diagnostics, native crash tombstones এবং Rust panic hooks integrate হতে পারে; user consent ছাড়া external upload হবে না। Crash loops UI-তে safe mode/recovery prompt দিতে পারে, কিন্তু repeated restart device usability নষ্ট করবে না।

## ৩৪.৬ acceptance criteria

- Service registry schema validation ও missing binary detection।
- IPC fuzz/frame bounds/peer-credentials tests pass।
- Each core service exposes readiness and truthful backend capability status.
- Logs include boot/revision context but exclude private content.
- Crash/restart events machine-readable and retention-managed.
- Shell status/terminal draws data from services, not fabricated outputs without visible marker.

---

# ৩৫. Security architecture: threat model, defense in depth ও release gate

## ৩৫.১ threat model লিখিত ও living document হতে হবে

Security feature list থাকলেই security architecture প্রমাণিত হয় না। Threat model-এ attacker capabilities, attack surfaces, assets এবং trust boundaries স্পষ্টভাবে থাকবে। উদাহরণ: untrusted `.nilax` package, compromised app publisher, network attacker, local multi-user app, malicious IPC client, corrupted update image, broken firmware/bootloader, physical device access, stale Android permission, malformed camera/audio frame, service crash এবং supply-chain compromise। কোন attack project-এর scope-এর মধ্যে, কোনটি hardware root-of-trust বা kernel hardening ছাড়া mitigate করা যাবে না—তা honestভাবে লিখতে হবে।

## ৩৫.২ trust boundaries

মূল boundary হবে: bootloader/kernel ↔ initramfs/PID1; PID1 ↔ service daemons; privileged service ↔ unprivileged apps; package manager ↔ publisher trust store; guest NilLang runtime ↔ host Java/JNI bridge; native QEMU/phone backend ↔ hardware devices; update installer ↔ current system slot। প্রতিটি crossing-এর validation, authentication, authorization, resource limits, error policy এবং audit event থাকতে হবে। Android JNI bridge-কে app-internal হলেও unsafe boundary হিসেবে ধরতে হবে; native memory parsing এবং OS operations সেখানে ঘটে।

## ৩৫.৩ “fail-closed” কোথায়

Fail-closed mandatory: wrong-architecture binary, checksum/signature mismatch, untrusted package, path traversal, privilege drop failure, seccomp setup failure, unauthorized privileged command, required device/profile mismatch, release key unavailable, AVB invalid, mandatory storage failure (release persistence mode), incomplete boot metadata এবং flashing manifest mismatch। Optional feature unavailable হতে পারে: physical camera absent on QEMU, Bluetooth adapter absent, Wi-Fi disconnected, no external display, hosted Android permission denied। Optional feature-এ structured `Unsupported/Unavailable/Denied` state ফিরবে; security boundary fail হলে unisolated fallback নয়।

## ৩৫.৪ security CI

`cargo audit` dependency scan ছাড়াও source secret scan, tracked APK/cache artifact scan, SBOM generation, license inventory, dependency verification, unsafe-code review gate, fuzzing, SAST, shellcheck, Python lint, Android manifest review, signing policy test এবং release manifest verification প্রয়োজন। Security audit job success currently helpful; কিন্তু তার scope নথিবদ্ধ করতে হবে। Regex-based SELinux `neverallow` scan alone semantic policy compile/runtime enforcement-এর বিকল্প নয়। CI green-কে “zero vulnerabilities” হিসেবে বলা যাবে না; tool scope এবং scan date report করতে হবে।

## ৩৫.৫ vulnerability response

Security issue report করার private contact/path, severity classification, reproduction and patch policy, release branch backport, dependency update rules এবং disclosure timeline define করতে হবে। Public issue-তে exploitable vulnerability disclosure আগে patch coordination দরকার হতে পারে। Security key compromise হলে app publisher revocation, root key rotation ও affected OTA trust invalidation policy থাকা উচিত। Dependabot/RustSec style advisories timely review হবে; version bump-এর পরে full target matrix পুনরায় চালানো দরকার।

## ৩৫.৬ acceptance criteria

- Threat model maps components, assets, attacker capabilities and mitigations.
- Every privilege boundary has tests for unauthorized access and failure behavior.
- Release CI includes dependency/SBOM/secret/license scans and signed provenance.
- Security-critical failure never silently degrades into unconfined execution.
- SELinux enforcing, seccomp active, UID and cgroup capabilities verified runtime-evidence-সহ।
- Security statuses in UI originate from live kernel/runtime query, not static string.

---

# ৩৬. Performance, power, resource budget এবং reliability

## ৩৬.১ performance budget

Mobile OS-এ boot time, idle RAM, app launch latency, UI frame time, touch-to-paint latency, storage latency, radio wakeups, audio latency এবং battery drain গুরুত্বপূর্ণ। কিন্তু এগুলো measure না করে “lightweight”, “fast” বা “120Hz optimized” দাবি করা উচিত নয়। CI/QEMU-এর CPU model physical Snapdragon performance-এর প্রতিনিধি নয়। তাই virtual target-এ correctness/relative regression এবং physical phone-এ absolute performance আলাদা metrics।

একটি baseline metrics set: clean boot until `CoreServicesReady`; boot until `UIReady`; cold/warm application launch; memory RSS per daemon; steady-state idle CPU; frame-time p50/p95/p99; touch-to-present latency; disk write/read latency; audio underrun count; service restart frequency; thermal condition; suspend-to-resume delay; battery current/temperature over a controlled test duration। Each metric test conditions, device profile, firmware, ambient temperature, power mode এবং sample size记录 করবে।

## ৩৬.২ memory budgeting

`nild`, `nilbus`, `nilkeyd`, `netd`, `audiod`, `powerd`, `nilshell`, VM এবং compositor-কে resource budget দিতে হবে। Unbounded queues, frame buffers, image decode buffers, camera frames, audio PCM queues এবং untrusted package archive parsing heap exhaustion ঘটাতে পারে। Queue depth caps, chunk streaming, buffer pooling এবং backpressure প্রযোজ্য। Memory pressure-এ app kill policy priorities: essential system services survive; user app suspended/killed with notification; storage transaction complete/recoverable। Crash dump ও log rotation-ও RAM/disk budget পাবে।

## ৩৬.৩ power management

`powerd`-এর `BatteryInfo::default()` simulated data ফেরত দিতে পারে; live backend unavailable হলে 85%/29.5C/Good status user-facing UI-তে বাস্তব বলে দেখানো যাবে না। Linux sysfs battery path, Android `BatteryManager`, charging state, temperature, current/voltage properties platform-specific। Wakelock ownership lifecycle-managed হবে; leaked wakelock indefinitely screen/device sleep আটকে রাখতে পারে। App-এর wakelock API permission-broker controlled হবে, maximum duration/diagnostics থাকবে। Idle timeout, background limits, display brightness, suspend blockers এবং thermal throttling system tests দরকার।

## ৩৬.৪ reliability testing

Boot loop test, randomized service kills, repeated mount/unmount, app install/uninstall cycles, package corruption, permission revocation, network disconnect, camera unplug/busy, audio route changes, screen pause/resume, rapid rotation এবং reboot under write load চালাতে হবে। Test pass criteria শুধু “process still running” নয়; data integrity, state recovery, no stale indicators, no resource leak এবং bounded recovery time। Soak test QEMU-তে core daemons ও UI renderer hours-long চালাতে পারে; physical S25/OnePlus lab-এ thermal/power/real peripherals আলাদা।

## ৩৬.৫ acceptance criteria

- Baseline performance metrics documented and reproducible.
- All untrusted input queues/buffers have bounds.
- Wakelocks/worker threads release across all lifecycle paths.
- Fault injection preserves package/data integrity and system recoverability.
- Fake telemetry never used to score physical battery/network performance.
- Physical power/thermal claims have controlled hardware-test evidence.

---

# ৩৭. বাংলা ভাষা, accessibility, localisation এবং ব্যবহারযোগ্যতা

OnuronOS-কে বাংলা-প্রথম ecosystem হিসেবে গড়তে চাইলে বাংলায় UI text দেখানোই যথেষ্ট নয়। Text layout, font fallback, keyboard/input, date/time format, numerals, screen-reader semantics, translation process, error messages এবং developer documentation—সব জায়গায় language support ধারাবাহিক হতে হবে। Bengali shaping, conjunct glyphs, vowel marks, reordering, line-break rules এবং emoji fallback পরীক্ষা করতে হবে। Linux font stack ও Android host fonts একই নয়; তাই font asset availability ও licensing যাচাই গুরুত্বপূর্ণ।

## ৩৭.১ localisation architecture

UI strings source code-এ hard-code না করে resource catalog-এ থাকবে। প্রতিটি key-র English/Bengali translation, context/comment, plural/number formatting rule এবং fallback language থাকবে। Missing translation build time-এ report হবে; critical security/permission prompt-এর fallback language ব্যবহারকারী বুঝতে পারবে তা নিশ্চিত করতে হবে। User language selection persist হবে; system UI ও third-party app language policy আলাদা হতে পারে। Bengali date formatting বা Bengali digits ব্যবহার করলে parsing/storage ISO/locale-neutral data format থেকে আলাদা থাকবে; numerals visual display বদলালেও phone number, version number বা diagnostic code corrupt করা যাবে না।

## ৩৭.২ input methods

Bengali input method, transliteration, phonetic keyboard, compose sequences, hardware keyboard support এবং Android hosted text input synchronization আলাদা design needs। `.nil` TextField actual IME/composition events গ্রহণ করবে; key press sequence character output-এ premature commit করবে না। Unicode normalization (NFC/NFD), cursor position grapheme clusters, backspace behavior এবং selection/copy-paste tests দরকার। Localized numbers input করলে decimal separator, currency formatting এবং arithmetic parsing explicit হবে। Keyboard accessibility ছাড়াও voice input future option হতে পারে; permissions ও privacy clear থাকবে।

## ৩৭.৩ accessibility

Minimum readable text sizes, scalable fonts, high-contrast theme, reduced motion, focus order, semantic roles, clear error messages এবং touch target sizes design system token হবে। Battery, network, security এবং backend status icon color-only দিয়ে বোঝানো যাবে না; text/accessible description থাকবে। Permission denial screen error code hidden না করে understandable explanation দেয়। Low vision বা motor impairment ব্যবহারকারীরা শুধু swipe gesture-এর উপর নির্ভর না করে accessible controls দিয়ে কাজ করতে পারবেন।

## ৩৭.৪ বাংলা-first developer experience

NilLang diagnostics-এ primary language English হতে পারে যদি error IDs stable থাকে; কিন্তু Bengali explanation/guide ও examples দেওয়া যেতে পারে। Code syntax locale-dependent করা উচিত নয়; program source encoding UTF-8। Tutorial-এ বাংলা examples, error message glossary এবং translated docs থাকতে পারে। Text shaping regression CI screenshot এবং glyph metrics test-এর সঙ্গে যুক্ত হবে।

## ৩৭.৫ acceptance criteria

- Every system string goes through localization catalogs.
- Bengali shaping/conjunct, punctuation, line wrapping and cursor navigation tests pass.
- Text scaling, contrast and keyboard/screen-reader semantics verified.
- Localization cannot alter machine-readable IDs, numeric protocol values or file paths.
- Permission/security error messages have a readable fallback language.

---

# ৩৮. SoftBus ও Android compatibility-এর বাস্তব সীমা

`softbus`-এর maturity entry-তে mDNS-SD peer discovery এবং Quinn QUIC/TLS 1.3 transport বাস্তব code হিসেবে উল্লেখ আছে, কিন্তু UI-র discovered-peer list fabricated। এই অবস্থায় networking transport থাকা মানে secure multi-device ecosystem complete নয়। Peer identity, device trust, key exchange, access-control capabilities, revocation, user confirmation, offline state, replay protection এবং data conflict semantics দরকার। UI demo peers-কে live discovered devices বলা যাবে না।

## ৩৮.১ SoftBus trust model

Peer discovery untrusted hint; discovered service নিজে trusted না। User বা managed trust policy peer identity certificate/public key verify করবে; pairing flow visual confirmation বা trusted key distribution ব্যবহার করবে। QUIC/TLS channel encrypted হলেও app-level authorization নেই—কোন peer clipboard, notification, file transfer বা remote input ব্যবহার করতে পারবে তা per-capability allowlist নিয়ন্ত্রণ করবে। Device revoked হলে outstanding session/caches invalidate হবে। Replay, duplicate messages, disconnected peers, clock skew এবং key rotation test দরকার।

## ৩৮.২ Android compatibility

`docs/maturity.toml` বলে Android compatibility layer এখনও stub; UI-তে placeholder container state এবং sandbox isolation verification আছে, কিন্তু UI থেকে LXC/Waydroid-style container launch করা হয় না। তাই Android app compatibility দাবি করা যাবে না। Container architecture-এর জন্য Linux kernel features, Android userspace image, binder, graphics, audio, input, permission mapping, namespaces/cgroups, filesystem, SELinux policy এবং app lifecycle integration প্রয়োজন। Native phone port track C-র সঙ্গে এটি distinct effort।

## ৩৮.৩ hosted Android bridge বনাম Android container

S25 APK host bridge uses Android API to access device capability; এটি Android app run করে Onuron desktop-এ দেখায় না। Container-based compatibility layer হলে Android framework/userland guest environment launch, process isolation, app window integration ও lifecycle bridging লাগে। Naming/documentation-এ `android-host` আর `android-compat` আলাদা রাখা উচিত। UI-তে “Android app running” শুধু তখনই দেখাবে যখন actual guest APK/app process started এবং window output received হয়েছে। Placeholder “Starting container…” message simulation mode-এ marked হবে।

## ৩৮.৪ acceptance criteria

- SoftBus UI live discovery source থেকে data পায়; fabricated peers explicit demo mode ছাড়া নেই।
- Peer authentication and capability authorization implemented and testable.
- Android compatibility status reports `not implemented/stub` until actual guest process and display tested.
- Hosted device capability bridge is not confused with Android application compatibility.
- remote control/clipboard/file transfer security review mandatory before release.

---

# ৩৯. CI/CD, automated test matrix এবং artefact provenance

## ৩৯.১ CI-র উদ্দেশ্য হলো বাস্তব অবস্থা মাপা

বর্তমানে repository-তে Code Quality, Security Audit, Windows Simulator, Farm CI, SELinux এবং Linux/QEMU workflow রয়েছে। এই বিভাজন উপকারী: formatting/lint, cross-platform compile, security checks এবং boot integration একই job-এর মধ্যে অপ্রাসঙ্গিকভাবে মিশে যায় না। কিন্তু workflow naming এমন হতে হবে যাতে maintainers বুঝতে পারে কোন job boot পরীক্ষা করে, কোনটি শুধু compile, আর কোনটি mock backend test করে। “Farm CI passed” বা “Security Audit passed” যথেষ্ট detail নয়; README/PR-তে exact run, commit SHA, target এবং tested scope দেখতে পাওয়া উচিত।

## ৩৯.২ required check matrix

প্রস্তাবিত matrix:

| Gate | x86_64 host | Linux x86_64 QEMU | Linux AArch64 QEMU | Android emulator | Physical S25 | Physical `fajita` |
|---|---|---|---|---|---|---|
| Format/Clippy/static checks | বাধ্যতামূলক | source shared | source shared | Java/Kotlin/lint | not relevant | not relevant |
| Rust workspace tests | বাধ্যতামূলক | guest integration separate | guest integration separate | JNI/native tests | device tests | device tests |
| target compile | host target | `x86_64-musl` | `aarch64-musl` | `aarch64-linux-android` | actual ABI | device userspace |
| initramfs pack | no | বাধ্যতামূলক | বাধ্যতামূলক | no | no | device image pack |
| PID1 boot | no | বাধ্যতামূলক | বাধ্যতামূলক | Android activity lifecycle | host process | bootloader/kernel |
| persistence | local temp tests | live disk reboot | live disk reboot | app storage sanity | host storage semantics | actual userdata |
| camera/audio/network | fake/unit | fake/virt devices | fake/virt devices | emulator API tests | hardware validation | hardware validation |
| verified boot/flashing | tool tests | simulated boot-control | simulated boot-control | APK signing | not native flashing | only after safety gate |

Matrix cells such as `not relevant` must be explicit; skipped jobs cannot silently count as successful hardware validation. CI capabilities evolve, so artifact retention and test data schemas should be consistent across jobs.

## ৩৯.৩ fail-closed workflow design

ARM64 job currently stops during workspace build; packaging and boot steps correctly skip after failure. Keep this sequential dependency. Do not use `continue-on-error` on cross-build, checksum verification, persistence or boot smoke. For optional hardware tests, separate job name `optional-...` and visible `skipped` result acceptable if no hardware runner exists; but then release report says “not run”, not “passed”. CI step `if: always()` can upload logs and manifests, but must not modify prior failing conclusion. Artifacts not found when build failed are acceptable in diagnostic job; missing required artifacts after a successful build should fail the job. 

## ৩৯.৪ artifact provenance

Every generated release bundle should have signed provenance that binds: Git SHA, repository/branch, target profile, compiler/toolchain versions, build script revision, dependency lockfiles, kernel source and checksum, `nilinit`/daemon hashes, rootfs manifest, initramfs hash, data/system image hashes, vbmeta or signature metadata, test results, CI workflow run and build timestamp. A manifest that simply says `verified: true` is too vague. Use fields that express verification kind and authority: `source_integrity=passed`, `elf_architecture=passed`, `qemu_boot=passed`, `storage_persistence=passed`, `avb=not_implemented`, `hardware_validation=not_run`. Release note can summarize these dimensions. Manifest must not include private key bytes or secrets. 

## ৩৯.৫ SBOM এবং dependency hygiene

Rust `Cargo.lock`, Gradle dependency graph, Android NDK version, Python requirements (if any), downloaded kernel, compiler toolchains and system packages all influence builds. CycloneDX/SPDX-style SBOM can list crate/package licenses, versions and hashes. Dependency scanning should be repeatable and known false positives documented; new RustSec advisory gets triaged. Update dependency patch version and rerun full platform matrix if it touches FFI, crypto, compression, networking, Android bridge or `nilprotocol`. Third-party toolchain actions in workflow should be pinned by immutable commit where appropriate or reviewed version pin policy; SHA pin reduces supply-chain ambiguity. 

## ৩৯.৬ acceptance criteria

- `main` required checks fail if mandatory compile, QEMU, storage or security tests fail.
- ARM64 job must run boot/persistence steps after compile success and attach logs.
- Each green badge clearly identifies its target and test scope.
- Build manifests bind binaries to exact Git SHA and toolchain.
- Release artefacts signed with separately managed keys; unsigned test builds are visibly test-only.
- SBOM, checksums, source provenance, license inventory and test evidence accompany release candidate.

---

# ৪০. প্রকল্প-পরিচালনা, PR policy, issue discipline এবং documentation

## ৪০.১ issue format

প্রতিটি engineering issue-তে অন্তত এই headings থাকবে: **Observed state**, **Expected state**, **Reproduction**, **Scope**, **Dependencies**, **Security impact**, **Tests**, **Acceptance criteria**, **Evidence required**। “Implement camera fully” ধরনের issue কাজ করার মতো ছোট নয়। সেটিকে permission flow, camera ID discovery, session open, one-shot capture, preview stream, image save, torch control, lifecycle cleanup এবং hardware test evidence-এ ভাগ করতে হবে। প্রতিটি issue-র end state measurable হবে; vague “improve” নয়।

## ৪০.২ PR size ও review

Critical boot path, flash tool, SELinux, package signature, sandbox এবং updater-এর changes আলাদা PR-এ রাখা উচিত। UI string change-এর সঙ্গে flashing logic ঢোকানো উচিত নয়। Type mismatch ঠিক করার PR-এ unrelated cleanup না করলে diff review সহজ হয়। Security-sensitive PR-এ threat/permission consequence এবং negative test proof থাকতে হবে। Flasher বা verified boot changes-এর জন্য অন্তত দুইজন reviewer থাকলে ভালো; ছোট open-source project-এ দ্বিতীয় reviewer unavailable হলে maintainer নিজে checklist পূরণ ও test evidence inspect করবেন, কিন্তু এটিকে independent review বলা যাবে না।

## ৪০.৩ ADR discipline

ADR-0001 থেকে ADR-0009 architecture decision log শুরু করেছে। নতুন বড় decision যেমন “Java Canvas prototype থেকে native Rust frame presentation”, “Alap standard library ABI”, “system partition read-only policy”, “A/B updater backend”, “SELinux optional vs mandatory target mode”, “Wi-Fi configuration API”—ADR দরকার হতে পারে। ADR-এ alternatives ও validation থাকা উচিত। Current `docs/adr/README.md` index-এ absolute `file:///c:/Users/...` links আছে—এগুলো public GitHub browsing context-এ কাজ করবে না। Relative GitHub paths ব্যবহার করতে হবে এবং `docs/adr/README.md` link checker CI-তে যোগ করা উচিত।

## ৪০.৪ maturity table এবং README

`docs/maturity.toml` source-of-truth হওয়ায় README generated table manually edit করা যাবে না। নতুন screen, backend বা hardware target যোগ হলে maturity entry, evidence path, `simulated` flag এবং screen registry update হবে। Generator `--check` CI-তে চালু থাকবে। Maturity level উন্নীত করার আগে corresponding evidence artifact থাকা দরকার। উদাহরণ: `functional-prototype` থেকে `tested-in-QEMU` মানে live QEMU test, not only unit test; `tested-on-physical-device` মানে actual hardware model/firmware/test steps; `production` মানে failure handling, security review, compatibility policy, support lifecycle এবং regression coverage পাস।

## ৪০.৫ architecture doc বনাম implementation

একটি architecture diagram ভবিষ্যৎ target state বলতে পারে, কিন্তু current code path-ও আলাদা diagram-এ দেখাতে হবে। `docs/architecture.md`-তে তিনটি view দরকার: (A) intended architecture, (B) currently implemented architecture, (C) known gaps. Feature maturity document code-level evidence path উল্লেখ করবে। `alap/` path না থাকলে diagram-এ Alap implementation stage `planned` হবে। Native phone profile থাকলেও physical board hardware matrix `planned` থাকবে। Evidence folder-এ stale JSON বা manually created `all_healthy=true` থাকলে generated artifact provenance ছাড়া it is only illustrative. 

## ৪০.৬ dependency graph ও project board

Project board-এ columns হতে পারে: Backlog → Ready → In Progress → Needs Review → CI Failed → Evidence Pending → Done. `Done`-এর অর্থ code merged plus acceptance evidence, not merely PR merged. P0 blockers top pinned থাকবে। প্রতিটি feature issue parent epic-এর সঙ্গে linked হবে; dependencies graph in CI/Docs publish করা optional কিন্তু useful। Focus রাখতে WIP limit স্থাপন করুন—এক maintainer এক সময়ে সর্বোচ্চ একটি major subsystem এবং দুই ছোট bug ধরবেন। এটা individual speed নয়, context-switch কমিয়ে delivery reliability বাড়ানোর ব্যবস্থা।

## ৪০.৭ acceptance criteria

- Issues contain reproduction/test/evidence criteria.
- PR touches one coherent subsystem; security-sensitive files require stronger review.
- ADR links are relative and verified by link checker.
- Maturity table is generated, current and evidence-backed.
- Documentation clearly separates intended/current/unverified architecture.
- Done means integrated and tested, not just source code written.

---

# ৪১. ৩০/৬০/৯০ দিনের বাস্তব কর্মপরিকল্পনা

এই সময়রেখা অনুমানভিত্তিক engineering cadence, প্রতিশ্রুত deadline নয়। Maintainer bandwidth, hardware access এবং CI failure অনুযায়ী তারিখ বদলাবে; গুরুত্বপূর্ণ হলো milestone gates ও dependency order। অক্টোবর ১০, ২০২৬-কে Day 0 ধরা হয়েছে। Project owner একা কাজ করলে scope আরও ছোট রাখতে হবে।

## Day 0–30: build ও virtual targets স্থিতিশীল করা

**সপ্তাহ ১:** NilHAL ARM64 compile error fix; target compile matrix; architecture-specific C types; workspace + Clippy; `build/mkinitramfs.py` clean output; stale artifact rejection। ARM64 job-এ compile failure গেলে error fix ছাড়া next sprint feature freeze থাকবে।

**সপ্তাহ ২:** AArch64 rootfs/package validation; real ext4 disk mode বাধ্যতামূলক; QEMU ARM64 serial boot; structured boot status; test results in CI. Error logs, kernel hash, rootfs manifest, build revision artefacts generate হবে।

**সপ্তাহ ৩:** persistent-data integration test, write/reboot/read; `nilinit` early mount fail handling; service readiness handshake; cgroup/SELinux target mode explicit। Process alive versus readiness distinction implement। Socket activation integration tests add।

**সপ্তাহ ৪:** target manifest cleanup, x86_64 regression suite, docs/evidence provenance, maturity table audit। QEMU x86_64 + ARM64 gate green না হলে native flashing work start করবে না। Day 30 target outcome: two virtual targets build and validate, with transparent remaining failure list. Physical boot claim এখনও নয়।

## Day 31–60: NilLang → package → sandbox → UI vertical slice

**সপ্তাহ ৫:** `nilpkg` integration tests reinforce; trusted publisher key, install/verify/upgrade/rollback crash injection; package manifest/runtime contract formalize।

**সপ্তাহ ৬:** `nilrt-launch` actually invoked in Linux integration tests; sandbox permissions default strict in production; UID registry lock/atomic durable write; seccomp/namespace failure abort behavior verify।

**সপ্তাহ ৭:** NilLang `.nil` compile, signed `.nilax` install, sandboxed VM launch; actual event dispatch (Button click changes state); NilUI layout scene rendering in QEMU or test renderer; source map and VM resource limits initial version.

**সপ্তাহ ৮:** Alap minimal crate (App/Text/Button/Column/Row/State); cross-backend contract; no large component catalogue. Maturity table only after tests. Day 60 target: a small sample app package can be signed, installed, launched in actual sandbox, scene rendered and input event changes visible state in one documented end-to-end test. This does not yet mean every app or every device works.

## Day 61–90: S25 hosted bridge engineering

**সপ্তাহ ৯:** NDK build linked with Gradle/CI, APK artifact manifest, JNI version mismatch rejection, Android emulator smoke test. Clean build has no stale `.so` use.

**সপ্তাহ ১০:** host lifecycle, permission flow, battery/network live telemetry; remove hard-coded sample network state from non-simulated backend; degraded state visible. Test denied/revoked permissions.

**সপ্তাহ ১১:** display decision: Java Canvas prototype label vs Rust-frame presentation. Choose one authoritative renderer for hosted UI; add Surface lifecycle pipeline and frame test. Then camera one-shot capture and audio playback/record minimal vertical slices, each with `simulated` vs `real` result.

**সপ্তাহ ১২/১৩:** S25 physical device manual test plan, logs sanitized, battery/network/camera/audio checks, repeated pause/resume, native bridge load/unload. Day 90 target: reproducible hosted APK + documented S25 test evidence for verified capabilities, not claim full native OS. If no physical device test can be run, status says emulator-verified/physical-not-tested.

## ৯০ দিনের শেষে যেগুলো এখনও later track

OnePlus 6T physical flashing, AVB root-of-trust implementation, complete telephony, all camera sensors, broad Android compatibility container, app store catalogue, SoftBus remote-control capability এবং OTA production rollout—এসব 90-day definition of success নয়। এগুলোকে পরের phase-এ নেওয়া হবে শুধুমাত্র P0 gates and hosted vertical-slice stability অর্জনের পর।

---

# ৪২. ছয় মাস ও বারো মাসের milestone plan

## ৪২.১ মাস ১–২: virtual boot এবং storage reliability

প্রথম দুই মাসের প্রধান outcome হলো “clean checkout → correct architecture image → QEMU boot → service readiness → persistent data verified”। এটি project-এর engineering backbone। Cross-compile, initramfs packer, kernel checksum, target manifest, ext4 disk, recovery boot counter, structured logs এবং deterministic tests এখানে stable হবে। এই stage-এ shell demo feature freeze থাকতে পারে। AArch64 green হতে সময় বেশি লাগলে month-2 UI milestones পিছিয়ে যাবে; critical path shortcut করে release status upgrade নয়।

## ৪২.২ মাস ৩–৪: application runtime slice

NilLang compiler/VM, Alap minimal API, NilUI renderer, `.nilax` package manager, sandbox launcher ও system capability broker-এর vertical slice complete করা হবে। প্রথম sample app-কে keyboard/touch event, state update, navigation, persistent note write এবং permission denied behavior demonstrate করতে হবে। Package store এর আগে secure install/update/rollback stabilize হবে। App API stable না হওয়া পর্যন্ত hundreds of widgets, template marketplace এবং complex app distribution add না করাই ভালো।

## ৪২.৩ মাস ৪–৫: hosted Android quality

Android-hosted application build, JNI protocol, Activity lifecycle, host telemetry, display surface, camera capture, audio playback/record, network state এবং permission UX improve করতে হবে। প্রতি subsystem-এ feature matrix: implemented on host, tested in emulator, tested on S25 physical, permissions, failure semantics, privacy impact। Hosted mode-এর success native mode-র success থেকে সম্পূর্ণ পৃথক report হবে।

## ৪২.৪ মাস ৫–৬: native reference preparation

এই phase শুরু হবে only when QEMU x86_64/ARM64 gates green and hosted runtime baseline stable. OnePlus 6T exact unit acquisition, stock firmware backup, bootloader status, kernel source research, device tree config, kernel build, boot image compatibility and recovery test. প্রথম acceptance boot to serial/`nilinit`; তারপর data storage; তারপর display/touch। Network/audio/camera/modem পরে। যদি hardware unavailable হয়, repository device port remains design-only; roadmap should not mark completed based on docs. 

## ৪২.৫ মাস ৬–৯: physical bring-up increments

Physical boot and serial logs confirm early boot. Next userland, persistent storage and safe recovery. Then display/touch input, screen blank/resume, battery/charger, USB, Wi-Fi/Bluetooth, audio, modem and camera. Each subsystem separately. Any physical flash action requires backup and rollback and target image signed/validated. OTA A/B should not deploy on real device until boot-control integration has been exercised in repeated failure tests.

## ৪২.৬ মাস ৯–১২: reliability and early beta

At month 9–12, if foundational criteria achieved, define supported hardware/feature set. Add app developer docs, stable NilLang/Alap API, accessibility/localization, crash diagnostics, performance baselines, system update flow and reproducible release bundle. Avoid declaring Android compatibility unless actual guest app launch/display/audio/input proven. Avoid broad device matrix early; one physical reference target plus QEMU targets is more valuable than half-working support for many phones.

## ৪২.৭ milestone gates

- **M1:** QEMU x86_64 + AArch64 boot and persistence green.
- **M2:** Core daemon readiness, secure package path and sandbox tests green.
- **M3:** NilLang sample application executed through actual runtime and shown through NilUI.
- **M4:** Android-hosted APK build and lifecycle/device-permission tests validated.
- **M5:** `fajita` board config + kernel/DTB + recovery path reviewed; physical boot begins only after gate approval.
- **M6:** physical boot to UI with storage/input; hardware matrix updates evidence for each subsystem.
- **M7:** update rollback, signed release artifacts and threat-model review complete.

A gate is passed only if acceptance evidence is attached and reproducible. Calendar milestone dates without the gate do not upgrade project maturity.

---

# ৪৩. P0/P1/P2/P3 ticket backlog ও dependency graph

## P0 — অবিলম্বে, release/flash blocker

**P0-A: NilHAL AArch64 C ABI compile fix.** Files: `runtime/nilhal/src/lib.rs` and plugin ABI declarations. Acceptance: AArch64 workspace build. Dependency: none. 

**P0-B: ARM64 CI green path.** Files: `.github/workflows/linux-qemu.yml`, `build/mkinitramfs.py`, `build/qemu-smoke.py`. Acceptance: compile, package, real filesystem, boot, readiness, persistence. Dependency: P0-A.

**P0-C: target identity manifest.** Files: build orchestrator, initramfs/disk/boot tools. Acceptance: alias mapping, exact output target, wrong architecture refused. Dependency: P0-B.

**P0-D: storage fallback honesty.** Files: `build/mkdisk.py`, `nilinit/src/main.rs`, QEMU harness. Acceptance: real ext4 required for persistent tests; fallback reported as volatile; marker survives reboot. Dependency: P0-B.

**P0-E: disable physical flashing until validated.** Files: Bash/PowerShell flashers and release packaging. Acceptance: release bundle cannot flash `fajita` until device profile, image signature, partition map and recovery evidence complete. No S25 native flash path. Dependency: P0-C, AVB work later.

**P0-F: remove false success labels for hardware operations.** Files: NilHAL backends, shell status, Android hosted APIs. Acceptance: camera/network/call/audio fake statuses visible and not reported as live success. Dependency: common capability/error model.

## P1 — critical product path

**P1-A: true readiness handshake** (`nilinit`, core services, `nilprotocol`).  
**P1-B: SELinux/cgroups enforcing verification** (build policy, early boot, runtime check).  
**P1-C: package trust and sandbox launch integration** (`nilpkg`, `nilrt-launch`, `nillang`).  
**P1-D: NilLang VM events and Alap minimal runtime.**  
**P1-E: NilUI layout-to-pixel and present confirmation.**  
**P1-F: Android NDK/Gradle CI and JNI versioning.**  
**P1-G: S25 lifecycle, live battery/network and permission tests.**  
**P1-H: camera/audio vertical slice with real/fake backend distinction.**  
**P1-I: canonical `fajita` profile and exact boot image/partition mapping.**

Dependencies: P1-C needs P0-B/P0-C; P1-D needs compiler/VM tests; P1-E needs P1-D; P1-F can run after P0-A because `nilhal` shared by Android crate maybe compile issue; P1-G and P1-H depend on P1-F; P1-I starts design/research but physical writes wait P0-E plus M5 gate.

## P2 — beta readiness

- App lifecycle state restoration and crash recovery.
- Full accessibility, Bengali localization and IME tests.
- Structured logs, crash reporting, metrics and ring-buffer policy.
- Network address/route/DNS validation, Bluetooth pairing.
- Real telephony state callbacks and SMS delivered state.
- Audio focus, route change, microphone recording, suspend/resume.
- package store signed catalogue, dependency solver and offline cache.
- performance/thermal/power baselines on supported device.
- OTA A/B transaction test and physical rollback evidence.
- docs link checker, SBOM, license report and release notes generator.

## P3 — defer until the foundation works

- Extensive widget library, dynamic UI effects, animated launcher customizations.
- Large music streaming demo and mock camera settings.
- Broad Android compatibility container/Waydroid feature.
- Multi-device SoftBus control, remote clipboard/file sync, advanced mesh routing.
- Multiple new phones at once.
- App store rankings/recommendations before package trust and runtime stable.

## ৪৩.১ backlog handling rules

প্রতি P0 item এক বা কয়েকটি ছোট PR হবে এবং evidence-backed review পাবে। P1 issue dependency ready না হলে “blocked” status-এ থাকবে। P2/P3 work critical-path feature freeze ভাঙতে পারবে না, unless it is needed for debugging a blocker. Scope creep ধরতে code review-এ প্রশ্ন হবে: “এই পরিবর্তন কি বর্তমানে failing/required acceptance criterion-কে unblock করছে?” যদি না করে, issue-তে আলাদা করা ভালো।

---

# ৪৪. Acceptance criteria: কোন প্রমাণে কোন কাজ “সম্পন্ন” বলা যাবে

## ৪৪.১ code complete বনাম feature complete

Code complete মানে planned code changes merged হয়েছে এবং unit tests pass। Feature complete হতে integration test, error handling, docs, maturity state এবং target evidence লাগবে। Production-ready বলতে আরও বোঝায়: supported configurations, security review, migration/rollback path, failure diagnostics, maintainable tests, compatibility policy এবং release artefact provenance। এই স্তরগুলো project board-এ পৃথক status হওয়া উচিত।

## ৪৪.২ status ladder

- `planned`: design/issue রয়েছে; working code নেই বা branch-specific proof নেই।
- `experimental`: implementation exists but API or behavior may change, tests limited.
- `simulated`: UI/backend demo data; user-facing state synthetic.
- `functional-prototype`: component functions in a limited test environment.
- `tested-in-QEMU`: explicit QEMU boot/integration test passed on named architecture.
- `tested-on-Android-emulator`: APK/permission/device APIs tested in emulator.
- `tested-on-physical-device`: exact model, firmware/API version and procedure documented.
- `release-candidate`: release build and signing flow, upgrade/recovery, security gate, regression suite and support notes pass.
- `production`: sustained use evidence, no known critical gaps, incident/support policy and enforced security model established.

A subsystem can have multiple backend tiers. `Network: tested-in-QEMU` does not mean Wi-Fi association hardware verified. `Camera: tested-on-S25` does not mean OnePlus V4L2 camera working. `nilrt` sandbox can be tested on Linux x86_64 while ARM64 seccomp remains separately unvalidated. Status matrix should expose dimensions, not one vague global word.

## ৪৪.৩ evidence packet

Each completion packet should include: commit SHA; target profile; build command; toolchain versions; test names/results; log link or redacted excerpt; manifest/checksums; environment details; failure test coverage; known limitations; reviewer sign-off; maturity update. Physical tests additionally include device codename/model/region, firmware build, bootloader state (no secret tokens), recovery readiness, power/temperature conditions, photos/screenshot only if appropriate, serial log sanitized, and repeated-run count. Sensitive identifiers must be redacted. Evidence cannot consist solely of a manually written statement “PASS”; raw artifact/runner result needed.

## ৪৪.৪ test skip policy

Tests may skip if hardware unavailable, but must say why and keep status `not_run`. A required CI gate cannot turn skip into pass. Fake HAL tests are useful but clearly name themselves `fake_backend`. QEMU virtual peripherals are not physical hardware. An Android emulator is not the S25. A successful host-compile is not a successful ARM64 execution. A passing pack/unpack unit test is not device boot compatibility. These four distinctions should be present in PR template and release checklist.

## ৪৪.৫ Definition of Done

একটি issue তখনই Done: implementation merged; tests added; required targets green; logs/evidence attached; error/failure behavior reviewed; security/privacy impact addressed; docs/maturity updated; no stale build artifacts; no new fake success semantics; related parent acceptance gates advanced. If a hardware requirement cannot be validated for lack of device, subtask can be “code ready, hardware not verified”, but parent feature remains not fully done. This prevents paper completion.

---

# ৪৫. release readiness checklist ও stop-ship শর্ত

একটি OS release candidate artifact প্রকাশ করার আগে নিম্নলিখিত সব gate পাস করতে হবে অথবা release notes-এ স্পষ্টভাবে excluded/experimental scope হিসেবে ঘোষণা করতে হবে। Native phone flash bundle-এর জন্য excluded hardware/verification gap optional note নয়—যে feature না থাকলে boot/data risk হয়, সেটি stop-ship।

## ৪৫.১ build and artifact

- [ ] Clean checkout থেকে canonical target build repeatable.
- [ ] Kernel checksums exact pinned match.
- [ ] All ELF architecture/interpreter checks pass.
- [ ] Rootfs/initramfs manifests and checksums available.
- [ ] no host fallback or untracked stale artifact in release mode.
- [ ] complete target profile and source revision in manifest.
- [ ] SBOM, license inventory, build provenance and release signature available.

## ৪৫.২ boot and storage

- [ ] QEMU target boot smoke passes.
- [ ] PID 1 state machine reaches readiness, required services answer readiness requests.
- [ ] mount success/failure logs reflect actual kernel outcomes.
- [ ] persistent storage write/reboot/read passes.
- [ ] no persistent-storage fallback when gate requires real data disk.
- [ ] recovery/failed boot counter tests pass.
- [ ] corrupted image/missing core service causes failure/recovery, not false green.

## ৪৫.৩ security

- [ ] trusted app publisher keys and signature validation are enforced.
- [ ] sandbox strict production mode, UID mapping and seccomp verified.
- [ ] SELinux mode verified if target security policy requires it.
- [ ] cgroups and device access limits verified if claimed.
- [ ] AVB only claimed if bootloader enforces real trust chain.
- [ ] update keys isolated; private secrets not present in repo/artifacts.
- [ ] flash wrong-target/missing-key/corrupted-image preflight performs no write.

## ৪৫.৪ user experience and hardware claims

- [ ] UI has no unlabelled fabricated battery/network/camera/audio/call status.
- [ ] permissions and denials visible and tested.
- [ ] input, display surface and lifecycle tests pass for target mode.
- [ ] Accessibility and localization status documented.
- [ ] Native hardware matrix claims have physical log evidence.
- [ ] Android hosted runtime is described as hosted app, not native boot.
- [ ] No unsupported compatibility layer advertised as working.

## ৪৫.৫ stop-ship examples

- ARM64 image build failure or wrong-architecture binary present.
- Kernel hash mismatch or unknown kernel provenance.
- Boot image target profile/partition map incomplete.
- No trusted signing key verification for release artifacts.
- Persistent data silently stored in tmpfs in a release target.
- Sandbox security setup fails but app launch continues unconfined.
- Flasher bypasses verified boot, writes generic image, or erases data unexpectedly.
- UI reports connected/recording/call active when only a demo state was set.

---

# ৪৬. ঝুঁকি-নিবন্ধন এবং ব্যর্থতার প্রতিক্রিয়া

ঝুঁকি-নিবন্ধনের উদ্দেশ্য সম্ভাব্য সমস্যার তালিকা বানিয়ে ভয় দেখানো নয়। উদ্দেশ্য হলো কোন সমস্যায় user data, device recovery, system integrity বা maintainer time সবচেয়ে বেশি ক্ষতিগ্রস্ত হতে পারে তা আগেভাগে চিহ্নিত করা এবং concrete mitigation নির্ধারণ করা। নিচের risk register প্রতি milestone-এ update করতে হবে; ঝুঁকির অবস্থান বদলালে owner, mitigation এবং residual risk-ও বদলাবে।

## ৪৬.১ R1 — Cross-target build false positive

**কারণ:** host binaries বা stale `out/` artefact ভুল target-এ copy/reuse হওয়া, manifest না থাকা, wrong target triple। **প্রভাব:** ARM64 image তৈরি হয়েছে বলে মনে হলেও boot না হওয়া; flash package ভুল architecture বহন করা। **বর্তমান controls:** `mkinitramfs.py`-এ target fallback restrictions ও ELF architecture check যোগ হয়েছে; architecture matrix CI চলছে। **অবশিষ্ট ঝুঁকি:** scripts-এর target identifiers এক নয় এবং target output mapping আলাদা থাকতে পারে। **Mitigation:** canonical target registry; exact ELF validation for every executable; stale artifact purge or revision binding; wrong-architecture negative tests. **Owner:** build pipeline maintainer. **Residual acceptance:** only after x86_64 and AArch64 CI both show fresh artefact manifests from same source revision.

## ৪৬.২ R2 — Kernel supply-chain error

**কারণ:** externally downloaded kernel, checksum pin change without review, source mismatch. **প্রভাব:** untrusted kernel execution, unexplained boot failure, security exposure. **Controls:** pinned SHA-256, fail-closed mismatch. **Mitigation:** recorded source, version, checksum and update PR; signed release source if available; independent artifact verification; no fallback when network fails. **Stop-ship:** checksum/source mismatch or `manifest.kernel.sha256` inconsistent with release payload.

## ৪৬.৩ R3 — Persistent storage silently volatile

**কারণ:** `mkdisk.py` synthetic fallback, device mount fail, `/data` tmpfs. **প্রভাব:** user configuration, PIN, app state, contacts/notes disappear after reboot; misleading success. **Controls:** warning in `nilinit`, ext4 image builder. **Mitigation:** real filesystem required in persistent target; boot status includes persistence flag; test live write/reboot/read; stop-ship if data mode mandatory and mount failed. **Stop-ship:** any release path which can mark OOBE or app install complete while backing data exists only in tmpfs.

## ৪৬.৪ R4 — Flasher writes wrong partitions

**কারণ:** generic fastboot commands, target alias mismatch, wrong product/slot, missing dtbo/vendor_boot partition awareness. **প্রভাব:** boot loop, user data loss, device unusable until recovery. **Controls:** product identity gate, generic target block, userdata wipe opt-in. **Mitigation:** keep default `flash_allowed=false`; exact profile map; dry-run; independent image header/AVB verification; stock image backups; recovery test; no `--force-unsupported` for release. **Stop-ship:** missing target manifest, unknown partition map, untested restore path or invalid signature.

## ৪৬.৫ R5 — Fake hardware status is mistaken for live state

**কারণ:** sample Wi-Fi/AP, battery constants, demo call state, fallback JPEG, static music tracks or UI badges overlooked. **প্রভাব:** user believes calls/camera/network/security work when they do not. **Controls:** `docs/maturity.toml` and `simulated` markers. **Mitigation:** unify HAL error model; live telemetry source + timestamps; simulated backend explicit; release mode refuses demo-only operation where user expects real capability; UI uses uncertainty/unavailable states. **Stop-ship:** false “call active”, “photo saved”, “Wi-Fi connected”, “encrypted” or “SELinux enforcing” claim without backend evidence.

## ৪৬.৬ R6 — Security boundary silently degrades

**কারণ:** SELinux load warning and continue, cgroup directories mistaken for active isolation, sandbox setup failure ignored, broad `/dev` access. **প্রভাব:** app runs outside expected containment; elevated privileges or device abuse. **Mitigation:** required feature capability checks; fail closed for security-critical setup; namespace/seccomp integration tests; per-app UID registry lock; release gate for runtime security state. **Stop-ship:** sandbox launch continues after critical privilege/isolation step failure.

## ৪৬.৭ R7 — JNI lifecycle races/resource leaks

**কারণ:** Java Activity surface lifecycle and Rust global frame queues independent; command thread not bounded; shared library loading with mismatched protocol version. **প্রভাব:** crashes, frozen screen, queue growth, stale Surface, duplicate commands. **Mitigation:** protocol version negotiation, surface generation IDs, cancellation tokens, bounded queues, lifecycle integration tests, thread cleanup and native sanitizer/debug tools where possible. **Stop-ship:** native crash, use-after-free evidence, queue memory growth without bound, permission bypass.

## ৪৬.৮ R8 — A/B update reports success prematurely

**কারণ:** software staged-slot state not connected to bootloader slot selection; update UI trusts downloaded file. **প্রভাব:** failed update, unbootable target, lost recovery path. **Mitigation:** simulated QEMU boot-control clearly marked; real boot-control implementation and power-loss testing required before native OTA. **Stop-ship:** no rollback proof or slot selection unverified.

## ৪৬.৯ R9 — Architecture grows wider than maintainer capacity

**কারণ:** many daemons and demo screens added before core pipeline works. **প্রভাব:** context switching, inconsistent contracts, large unfinished surface, user confusion. **Mitigation:** WIP limit, freeze noncritical UI, one vertical slice, P0/P1 backlog gates. **Success signal:** every sprint improves at least one mandatory acceptance criterion rather than only adding code paths.

## ৪৬.১০ incident playbook

Build regression হলে last known green commit ও diff isolate করতে হবে; flash-related failure হলে further writes বন্ধ করে device state/slot/log collect করতে হবে; persistence loss হলে stop writes, clone/image backup first, avoid automatic reformat; signing key issue হলে affected releases revoked/blocked; permission/privacy leak হলে data collection/telemetry immediately disable, logs preserve in a sanitized secure location and patch before release. Incident report should describe impact, timeline, root cause, recovery, preventive test and residual risk—not blame the contributor. 

---

# ৪৭. কোড-স্তরের উদাহরণ ও প্রস্তাবিত test commands

এই অধ্যায় ব্যবহারযোগ্য command snippets ও implementation patterns দেয়। এগুলো repository-র বর্তমান interface-এর সঙ্গে version অনুযায়ী adjust করতে হবে; কোনো command রান হয়েছে বলে এখানে দাবি করা হচ্ছে না। উদ্দেশ্য হচ্ছে PR ও local validation-এর ধারাবাহিকতা আনা।

## ৪৭.১ clean-tree ও revision capture

প্রতিটি audit/debug session-এর শুরুতে:

```bash
git status --short
git rev-parse HEAD
git log -5 --oneline
```

Output-টি issue অথবা test artifact-এ থাকবে। Local uncommitted changes থাকলে “clean checkout reproducer” দাবি করা যাবে না। CI pipeline `git rev-parse HEAD` থেকে manifest revision লিখবে; `.git` directory absent packaged source archive হলে `SOURCE_REVISION` build argument explicit নিতে পারে, কিন্তু default `unknown` নিয়ে release artifact publish করবে না।

## ৪৭.২ Rust validation sequence

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -W clippy::all
cargo check -p nilhal --target aarch64-unknown-linux-musl
cargo build --release --workspace --target aarch64-unknown-linux-musl
cargo build --release --workspace --target x86_64-unknown-linux-musl
```

এই commands সব local OS-এ একই রকম কাজ করবে এমন নয়; ARM64 target linker ও musl toolchain লাগতে পারে। Windows host-এ unit tests ও simulator build আলাদা; Linux-only mount/sandbox behaviour QEMU-তে চালাতে হবে। Build output failure হলে subsequent packaging commands manually পুরোনো files দিয়ে চালাবেন না।

## ৪৭.৩ Python build-harness validation

```bash
python3 -m unittest \
  build/test_mkinitramfs.py \
  build/test_mkbootimg.py \
  build/test_mkvbmeta.py \
  build/test_qemu_smoke.py \
  build/test_reproducible.py \
  build/test_persistent_data.py \
  build/test_gen_maturity.py
```

Project-এ Python test name বদলালে canonical test entrypoint (`python -m unittest discover` বা `make test-build`) ব্যবহার করা উত্তম। Individual test file-এর exact names CI থেকে derive হবে। A future Makefile/task runner যেন Linux, Windows ও CI-তে documented entrypoint দেয়।

## ৪৭.৪ initramfs validation sequence

```bash
cargo build --release --workspace --target aarch64-unknown-linux-musl
python3 build/mkinitramfs.py --arch aarch64
python3 build/mkdisk.py --output out/aarch64-qemu/data.img \
  --size-mb 512 --real --force
python3 build/qemu-smoke.py --arch aarch64
```

`--force` disk image format করার জন্য data ধ্বংস করতে পারে; উপরোক্ত sequence শুধু disposable test output directory-তে চালাতে হবে। Production/personal-data disk কখনও force করে format করা যাবে না। `mkinitramfs.py`-তে বর্তমান CLI যদি explicit custom output path না নেয়, target config default path ব্যবহার করছে—এই আচরণ canonical target refactor-এ উন্নত করতে হবে।

## ৪৭.৫ command result contract-এর conceptual model

Hardware command method এমন status দিতে পারে:

```rust
pub enum OperationStatus<T> {
    Completed(T),
    Pending { request_id: String },
    Unsupported { reason: String },
    Unavailable { backend: String, reason: String },
    PermissionDenied { permission: String },
    TimedOut { request_id: String },
    Failed { code: String, message: String },
}
```

এটি exact required final API নয়; core ধারণা হলো `Result<()>` দিয়ে সব semantics গুঁজে না দেওয়া। Camera capture-এর accepted request, delivered frame এবং saved image ভিন্ন status। Dialer launched ও call active ভিন্ন। Network config request ও successful association ভিন্ন। Feature UI operation status ও current state snapshot দুইটাই consume করবে। Error message user-readable হলেও error code stable থাকবে।

## ৪৭.৬ readiness handshake-এর conceptual flow

```text
nilinit starts daemon
        ↓
service parses config and initializes dependencies
        ↓
service binds its canonical Unix socket
        ↓
service sends READY(version, capabilities, backend_state)
        ↓
nilinit validates peer identity + readiness schema
        ↓
core boot state advances
```

একটি file `/run/onuron/netd.ready` তৈরি করা simplification হতে পারে, কিন্তু file ownership, stale marker removal, boot ID match এবং service restart-এ marker cleanup প্রয়োজন। Protocol handshake বেশি কার্যকর, কারণ এটি service নিজে request process করতে প্রস্তুত কি না বলতে পারে।

## ৪৭.৭ storage persistence test pseudocode

```text
create disposable ext4 disk image
boot QEMU with image attached
wait for structured READY state and data_mount=persistent
write unique marker in /data; fsync file and parent directory
request clean reboot through tested shutdown path
wait for QEMU exit
boot again with same disk image
read marker and compare exact payload/hash
write second marker and repeat after abrupt guest kill
```

Abrupt kill-এর পরে filesystem journal recovery এবং committed write semantics আলাদা test হবে। Expected loss window documented হবে; data already acknowledged as durable should survive. `debugfs` offline inspection ancillary proof, live mount remains necessary.

## ৪৭.৮ APK packaging validation

```bash
cd android-host
./build-ndk.sh
cd app
./gradlew clean assembleDebug
```

Windows equivalent PowerShell script and `gradlew.bat` commands can be listed in README. After build, inspect APK zip contents and verify `.so` ABI. Run emulator with `adb install`/`adb shell am start` only in a controlled test environment; exact application component and package identity should come from Gradle config. Production signing keys should not be used in debug CI. This plan does not assume an emulator has been run; it recommends adding that gate.

## ৪৭.৯ flasher test without device writes

Flash tool requires a dry-run mode that does not call `fastboot flash`; unit/integration testing should mock command responses or use a fake executable in PATH. Test cases: no device, multiple devices, unknown product, product mismatch, target alias, missing boot image, missing system image, invalid manifest, invalid signature, wrong profile, missing vbmeta, wrong slot, insufficient partition size, user declines, and `--wipe-userdata` absent/present. Every failure test must assert that no flash/erase/reboot command was issued. Hardware-connected destructive testing is not part of ordinary CI.

---

# ৪৮. সর্বোচ্চ-ফলদায়ক কাজের ক্রম: এখন কী করা উচিত

বর্তমান অবস্থায় নতুন feature list বাড়ানো প্রকল্পকে যতটা দৃশ্যমান অগ্রগতি দেয়, core acceptance gates ততটাই বেশি প্রকৃত সক্ষমতা বাড়াবে। আগামী কাজের practical order নিচে দেওয়া হলো।

## Step 1 — ARM64 compile fix

`runtime/nilhal/src/lib.rs`-এ `CStr`/C ABI pointer type consistency ঠিক করুন। Plugin ABI declaration ও C-string lifetime validate করুন; target-specific `c_char` semantics use করুন। Cast মাত্র যোগ করবেন না। Test: host workspace, AArch64 target check/build. PR title যেন শুধু “fix arm64 build” হয় এবং unrelated demo UI changes অন্তর্ভুক্ত না করে।

## Step 2 — full ARM64 CI run

`aarch64-qemu` job-এ compile পাসের পর packaging, real ext4 formatting, QEMU boot এবং persistence test চালান। Failure হলে job logs সংগ্রহ করুন; evidence JSON manual `true` রাখবেন না। ARM64 job green না হওয়া পর্যন্ত “ARM64 QEMU supports OnuronOS” বদলে “ARM64 QEMU target in validation” বলুন।

## Step 3 — target names and outputs

Canonical target manifest/registry তৈরি করুন। `build/build.sh` যেন `fajita` target-কে AArch64 + device profile হিসেবে চিনতে পারে; `mkbootimg.py`, `mkinitramfs.py`, `mkdisk.py` এবং flasher output directory agree করে। Wrong target image build tests পাস না করলে flash script disabled থাকবে।

## Step 4 — boot and storage honesty

`nilinit` early mount errors ignore করা বন্ধ করুন। Persistent test requires real ext4 disk; `/data` mount fail হলে QEMU persistence test fail। Live data marker survives reboot. Manifest indicates actual persistence mode; screenshots or static evidence are not enough.

## Step 5 — readiness and security gate

Core service readiness handshake; cgroups actually configured; SELinux policy load/enforcing confirmed; security-critical sandbox failures launch abort করে। `nilrt` UID registry concurrent-safe, package install runs under actual sandbox. This is more important than adding a media app catalogue.

## Step 6 — true app vertical slice

NilLang source → compile → `.nilax` package → trusted publisher verification → install → `nilrt-launch` → sandboxed VM → NilUI scene → compositor frame → input event changes state. One tiny application fully works and is tested across QEMU. This provides more architectural value than ten stub screens.

## Step 7 — S25 hosted Android flow

NDK and Gradle integration, JNI ABI/version, native frame presentation or clearly labelled Java Canvas mode, real battery/network callbacks, permission gating, camera capture and audio path. Each backend operation must return real status. Physical S25 test evidence separate from emulator results.

## Step 8 — `fajita` native port preparation

Only after previous gates, exact hardware/firmware, backup/recovery and profile. First boot to serial, then userspace/data, then display/touch, then power/network/audio/modem/camera. Build/flash safety stays a separate gate. Do not try to make all phone subsystems functional simultaneously.

## Step 9 — maturity and release discipline

Every code path updates `docs/maturity.toml` and the `simulated` registry. Release artefacts carry checksums and provenance; no production claims without evidence. This is a continuous activity, not a final documentation day.

---

# ৪৯. সংযোজনী: evidence template, issue template ও Definition of Done

## ৪৯.১ evidence record template

প্রতিটি target test-এর জন্য এই schema অনুসরণ করা যায়:

```json
{
  "schema_version": 1,
  "target": "qemu-aarch64",
  "source_revision": "<git-sha>",
  "workflow_run_id": "<run-id>",
  "run_timestamp_utc": "<timestamp>",
  "toolchain": {
    "rustc": "<version>",
    "cargo": "<version>",
    "python": "<version>",
    "qemu": "<version>"
  },
  "artifacts": {
    "kernel_sha256": "<hash>",
    "initramfs_sha256": "<hash>",
    "data_image_sha256": "<hash>"
  },
  "checks": {
    "architecture_validation": "passed|failed|not_run",
    "boot_smoke": "passed|failed|not_run",
    "service_readiness": "passed|failed|not_run",
    "persistence_reboot": "passed|failed|not_run",
    "selinux_enforcing": "passed|failed|not_run"
  },
  "known_limits": ["<truthful limitations>"],
  "log_artifact": "<url or artifact name>"
}
```

এই JSON উদাহরণে placeholder আছে; release evidence-এ placeholder রেখে publish করা যাবে না। Boolean `true` বা generic `verified:true`-এর বদলে each check string scope-সহ রাখা review সহজ করে। Hardware evidence-এ device model/codename, firmware version, bootloader/slot context এবং recovery status যোগ হবে। Sensitive device IDs, phone numbers, Wi-Fi SSIDs, serial number বা keys public evidence থেকে বাদ দিতে হবে।

## ৪৯.২ issue template

**Title:** subsystem + observed defect + target.  
**Current state:** exact commit, file/line, existing behavior.  
**Expected state:** desired result; fake/real/backend context.  
**Reproduction:** commands, environment, inputs, logs.  
**Risk:** data loss, privacy, security, build regression, hardware risk.  
**Implementation scope:** files/components that may change; excluded scope.  
**Dependencies:** upstream blockers and required target.  
**Acceptance tests:** unit, integration, negative, failure injection, hardware (if required).  
**Evidence:** CI link, output artifact, sanitized logs.  
**Maturity update:** tier/simulated status and source-of-truth path.  
**Rollback:** how to revert without leaving corrupt state or stale artifacts.

## ৪৯.৩ pull request checklist

- [ ] This PR advances a named acceptance criterion or fixes a reproduced issue.
- [ ] New code compiles for relevant targets, not only the developer host.
- [ ] Fake backend, stubbed data and fallback behavior are explicit.
- [ ] Input validation and negative tests are included.
- [ ] Security boundary and permissions are documented.
- [ ] No private key, APK, `.gradle` cache, user data or generated dump accidentally tracked.
- [ ] Artifact manifest/revision is correct and reproducible enough for its declared tier.
- [ ] `docs/maturity.toml`, ADR or user docs update when needed.
- [ ] Flash/update/storage impacts have been reviewed; no destructive action hidden in defaults.
- [ ] Failed/skipped tests are described honestly.

## ৪৯.৪ test evidence hierarchy

Evidence strength generally rises from design note, code inspection, unit test, component integration, QEMU live execution, Android emulator, physical device controlled test, repeated hardware soak and independent reproduction. Different evidence types answer different questions; physical photo of a boot screen may prove visible boot but not persistence, and CI compile may prove syntax but not runtime behavior. Evidence matrix should be appropriate to each claim rather than ranking one evidence type as universal. Security claims often need code review plus adversarial negative tests; storage claims need persistence tests; display claims need actual frame-presentation observation; real cellular claims need modem/host state callbacks and device tests.

---

# ৫০. শেষ সিদ্ধান্ত: কোন অবস্থায় OnuronOS-কে কোন নামে পরিচয় দেওয়া যাবে

OnuronOS-এর দীর্ঘমেয়াদি vision—Linux kernel, Rust-first userspace, NilLang + Alap app ecosystem, NilUI, secure package install, QEMU and phone ports—একটি coherent direction। এটিকে অর্জন করতে হলে “অনেক components আছে” থেকে “end-to-end tested system” পর্যায়ে যেতে হবে। Current repository already contains meaningful code in build tools, init/supervision, package management, runtime sandbox, NilLang parser/VM, UI/compositor, hosted JNI integration and system daemons. The next stage requires fewer disconnected additions and more verified vertical slices.

## ৫০.১ সৎ পরিচয়

বর্তমান evidence অনুযায়ী OnuronOS-কে Linux-based mobile OS prototype ও development project বলা যায়। x86_64 QEMU boot smoke test সর্বশেষ CI-তে পাস করেছে। ARM64 QEMU integration configured but current CI fails at NilHAL compilation, so ARM64 boot has not yet been verified in the latest run. Samsung S25 is a hosted Android application test target; it is not the native flashing target. OnePlus 6T `fajita` is selected as the native reference candidate, but its hardware matrix currently has planned subsystems rather than demonstrated physical bring-up.

## ৫০.২ সফলতার মানদণ্ড

এই প্রকল্পের সবচেয়ে গুরুত্বপূর্ণ সাফল্য হবে একটি ছোট but real vertical slice যা fresh checkout থেকে reproducibly build হয়, correct architecture image তৈরি করে, QEMU-তে PID 1 boot করে, required service readiness handshake pass করে, persistent storage-এ committed write survives reboot, signed NilLang package trusted key দিয়ে install হয়, actual sandbox-এ চলে, NilUI frame render হয় এবং user input state change ঘটায়। তারপর একই high-level app model S25 hosted runtime-এ tested backend দিয়ে চলে। এরপর native phone boot path আলাদাভাবে device-specific profile, verified recovery এবং physical test evidence দিয়ে উন্নত হয়।

## ৫০.৩ শেষ পরামর্শ

এখনই সবচেয়ে বেশি return দেবে NilHAL ARM64 compile fix, ARM64 QEMU full green gate, persistence validation এবং target/profile mismatch cleanup। এগুলো পাস না হওয়া পর্যন্ত নতুন shell screen যোগ করার চেয়ে build/boot correctness-এ সময় দেওয়া বেশি লাভজনক। তারপর package-to-runtime-to-screen vertical slice; তারপর S25 bridge; তারপর OnePlus 6T native bring-up। এই ক্রমে গেলে প্রতিটি পরের ধাপ আগেরটির উপর দাঁড়াবে এবং project-এর maturity measurable হবে। “Complete OS” দাবি নয়—একটি করে tested capability এবং reproducible evidence-ই হবে OnuronOS-এর বিশ্বাসযোগ্য অগ্রগতির মাপকাঠি।

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী A: subsystem অনুযায়ী বিস্তারিত acceptance matrix

এই matrix project board-এ bulk import করা যায়। প্রতিটি subsystem-এর “must pass” পরীক্ষা আলাদা রাখা জরুরি; একটি generic workspace test suite সকল behavior cover করে না।

## Build tooling

- Unit: target normalization, CLI argument validation, path traversal prevention, kernel hash checking, ELF machine parsing, archive permissions, manifest generation.
- Integration: clean cross-build → initramfs → QEMU; no previous output directory; missing required binary; wrong target binary; checksum mismatch; network unavailable with no cached kernel.
- Acceptance: correct failure code and no stale artifact packaged; manifest names exact source revision.

## PID 1 / services

- Unit: service config parser, command-line splitting, restart backoff, core dependency evaluation, boot counter logic.
- Integration: core service fails to launch; service starts slowly; service becomes ready; process exits after ready; socket activation FD; shutdown request; data mount failure.
- Acceptance: boot state never claims ready before service readiness; restarts bounded; failed required service yields failure/recovery state.

## Storage

- Unit: volume/profile selection, label/UUID parser if implemented, state marker schema, atomic file utility.
- Integration: real ext4 mount, write/fdatasync/fsync, reboot persistence, disk full, corrupt superblock, missing device, tmpfs fallback, existing image reuse.
- Acceptance: user-visible saved state aligns with durable storage result; no accidental format.

## NilLang + runtime

- Unit: parser grammar, typed AST validation, bytecode loader bounds, VM opcodes, event dispatch, instruction budget.
- Integration: compile, package sign, install trusted package, launch sandbox, render scene, click button, persist app data, kill/restart app.
- Acceptance: untrusted source cannot execute outside app sandbox; errors are structured; installed content is what signature verified.

## Android hosted bridge

- Unit: JNI protocol parser, queue bounds, frame buffer sizing, command schema validation.
- Integration: Gradle APK, native load, surface create/change/destroy, battery update, permission denial, network callback, camera capture, audio playback, pause/resume.
- Acceptance: source states are true and timestamped; no stale `.so`; no fake live operation success.

## Native device

- Unit: device profile schema, partition mapping logic, image manifest verification.
- Lab integration: board-specific kernel build, boot image parse independently, `fastboot getvar` match, dry-run, stock restore.
- Physical integration: serial boot, persistence, display/touch, power, network, audio, modem, camera, suspend/resume.
- Acceptance: no subsystem promoted beyond available evidence; flash gate remains disabled until device safety checklist pass.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী B: staged delivery bundles

একটি release bundle-এর ভিতরে কোন files থাকা উচিত, তা আগে থেকে নির্ধারণ করলে build output এবং release promise উভয়ই পরিষ্কার হয়।

## B.1 QEMU development bundle

- Kernel file (pinned hash).
- Initramfs CPIO gzip (manifest checksum).
- Disposable real ext4 `data.img` অথবা clearly synthetic test image.
- `manifest.json` with source revision, target triple, tools and hashes.
- `checksums.txt`.
- QEMU launch script and exact command line.
- Serial boot log and test JSON.
- Test suite result and known limitations.

এই bundle physical phone-এ flash করার জন্য নয়। QEMU machine type, virtio devices এবং console parameter explicit থাকবে। Development signing keys বা personal app data bundle-এ থাকবে না।

## B.2 Android hosted APK bundle

- Debug APK or signed release APK, type clearly marked.
- Android ABI/native library manifest.
- APK hash and signature info.
- Gradle/NDK/JDK versions.
- JNI protocol version and capability list.
- Emulator smoke results and physical S25 test report when available.
- Permission list and known unsupported operations.
- Privacy statement explaining hosted mode and telemetry/no-telemetry behavior.

APK without `libandroid_host.so` might still launch a Java-only prototype, but it must not be labelled native bridge-enabled. If pure hosted fallback is allowed, manifest and UI should say that.

## B.3 Native device development bundle

- Board-specific kernel/DTB/DTBO and documented source.
- initramfs and rootfs/system image for exactly that device.
- device profile/partition map and slot state requirements.
- checksums, signatures, trust/key info, build provenance.
- boot image manifest and independently validated format.
- bootloader/recovery/stock-restore guide and required host tools.
- physical-device test matrix and known limitations.
- Explicit warning about userdata wipe, bootloader unlock and recovery boundaries.

No such bundle should be published while profile/partition mapping remains speculative. A sample `boot.img` created by packaging tool does not automatically belong in the physical bundle.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী C: ব্যবহারযোগ্য design review questions

Major code review-এ নীচের প্রশ্নগুলো একসঙ্গে জিজ্ঞাসা করতে হবে। সব প্রশ্নের উত্তর প্রতিটি tiny PR-এ লাগবে না, কিন্তু affected scope অনুযায়ী reviewer checklist-এ সেগুলো পূরণ হবে।

1. এই পরিবর্তন কোন target-এর জন্য—QEMU x86_64, QEMU AArch64, Android-hosted, না native phone?
2. Is a success status based on an actual backend response or a locally mutated field?
3. What happens if the hardware/service is absent, permission denied, times out, or returns malformed data?
4. Does a failed security initialization stop execution or silently use a weaker fallback?
5. Are file writes atomic and durable where the UI promises “saved”?
6. Can the operation be retried safely? Could retry duplicate SMS, install, update or destructive actions?
7. Are all input buffers bounded? Are integer overflow and malformed encodings handled?
8. Is the code target-portable, including FFI types, syscalls, libraries and `cfg` branches?
9. Does test cover negative paths, not only intended success?
10. Does the test run target environment or just compile on developer machine?
11. Are fake/test fixtures clearly labelled and excluded from production paths?
12. Are generated artifacts tied to source revision and target profile?
13. Does this require a new ADR or update existing architectural contract?
14. Are documentation and maturity labels accurate after this change?
15. Is there a safe rollback if this subsystem changes persistent state or device partitions?

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী D: পরিমাপযোগ্য weekly review

প্রতিটি সপ্তাহের শেষে status update চারটি ভাগে দিতে হবে। **Completed with evidence:** merged work and exact CI/test links. **Blocked:** explicit blocker and next action. **Risks discovered:** security, build, storage, interface or test coverage issue. **Next week’s one critical outcome:** one primary acceptance criterion. Status report-এ code line count, commit count বা added screens count কে success metric বানাবেন না। Milestone progression এবং evidence quality-কে metric করুন।

একটি উদাহরণ report shape:

- **Current revision:** commit SHA and date.
- **QEMU x86_64:** build / boot / readiness / persistence results separately.
- **QEMU AArch64:** compile / package / boot / persistence results separately.
- **Android-hosted:** APK build / native load / lifecycle / permissions / camera/audio capability evidence.
- **Native `fajita`:** preparation / kernel / boot / storage / display / input / other subsystem status.
- **Security gate:** package signing, sandbox, SELinux/cgroups, AVB status.
- **Top three blockers:** each linked to an issue.
- **Next gate:** what exact test result would move the project forward.

এই review format ensures feature status does not become marketing language and new maintainer can understand what still doesn't work.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী E: শেষের জন্য সবচেয়ে ছোট possible critical path

যদি project maintainer resources সীমিত হয়, তবে এই ক্রমে কাজ করা সবচেয়ে practical:

1. `nilhal` AArch64 C pointer type fix এবং compiler matrix.
2. ARM64 QEMU job green, initramfs architecture audit, live boot.
3. Real ext4 write/reboot/read test and early mount logging corrections.
4. `nilinit` readiness handshake and required-service failure policy.
5. `nilrt` UID registry locking, strict sandbox defaults, and package verification integration.
6. One tiny NilLang/Alap app installed via `.nilax`, executed through sandbox, shown by NilUI, button event changes state.
7. Android-host NDK/Gradle CI plus JNI protocol and lifecycle integration.
8. Real S25 battery/network telemetry; then one camera capture and one audio playback/record path.
9. OnePlus 6T device profile, kernel/DTB research and recovery validation—no flash until safe bundle/verified boot review.
10. Physical hardware bring-up one subsystem at a time, backed by exact evidence.

এই দশটি কাজ সম্পন্ন হলে OnuronOS-এর প্রকল্প-পরিচয় আরও শক্ত হবে কারণ এটি শুধু code repository নয়, tested architecture ও reproducible process-সহ system prototype হবে। তার আগেও project-এর অগ্রগতি মূল্যবান, কিন্তু maturity claim evidence-এর সঙ্গে সামঞ্জস্যপূর্ণ থাকতে হবে।

# অতিরিক্ত বাস্তবায়ন-সংযোজনী F: NilHAL capability model-কে স্থায়ী platform contract করা

`NilHAL` হচ্ছে native system service, QEMU backend এবং Android-hosted backend-এর মধ্যে abstraction layer। এটির quality পুরো platform-এ প্রভাব ফেলবে। যদি HAL API সব backend-এর জন্য too optimistic হয়, তবে upper-level app logic বাস্তব hardware না থাকলেও success ধরে নেবে। যদি API খুব platform-specific হয়, তবে NilLang/Alap portability ভেঙে যাবে। তাই capability model-কে একটি versioned contract হিসেবে পরিচালনা করতে হবে।

## F.1 capability discovery এবং support state

একটি target startup-এ HAL backend capability snapshot প্রকাশ করবে। প্রতিটি capability হতে পারে `supported`, `supported_with_limits`, `unavailable`, `unsupported`, `permission_denied`, `not_initialized`, `simulated`। যেমন QEMU target-এ virtual display supported, physical camera unavailable; hosted Android এ camera permission granted হলে camera capture supported, কিন্তু phone call control unavailable বা user-confirmation-only; `fajita`-তে camera driver not validated until physical test. Capability list static compiled enum হয়ে থাকতে পারে, কিন্তু runtime result backend discovery দ্বারা validate হবে। App UI capability snapshot consume করবে; নিজস্ব hard-coded spec নয়।

## F.2 operation result semantics

একটি operation `Result<T, HalError>` ফেরালেই সব প্রশ্নের উত্তর নাও পাওয়া যেতে পারে। কিছু command asynchronous: camera session open, Wi-Fi connect, audio route switch, call dial, system update. API-তে request acknowledgement ও completion event আলাদা হবে। Request-এর unique ID থাকবে; duplicate ID handling ও cancellation state defined হবে। Request timeout হলে operation actual hardware side-এ চলমান থাকতে পারে; caller retry করার আগে query operation status করতে পারবে। Hardware state change event out-of-order এলে monotonic sequence/timestamp দিয়ে stale event discard করা হবে।

## F.3 backend adapter architecture

Common trait logical behavior expose করবে; platform adapter API mapping করবে। `FakeHAL`, `QemuHAL`, `LinuxHAL`, `AndroidHostHAL` প্রত্যেকের explicit constructor ও identity থাকবে। `NilHal::auto()` dev convenience; release images-এ environment variable বা filesystem existence-এর উপর ভুল backend auto-select হওয়া রোধ করতে target manifest/boot config authoritative হবে। Backend fallback chain stable and visible হবে। `LinuxHAL` system path পাওয়া যায়নি বলে Android bridge চেষ্টা করতে পারে; কিন্তু শুধু `/system/lib64/libandroid_runtime.so` file exists থাকলেই libhybris runtime usable নয়—availability probe ও link tests দরকার।

## F.4 error model governance

`HalError`-এ `Timeout`, `InvalidArgument`, `BackendUnavailable`, `ProtocolError`, `NotReady`, `Cancelled` যোগ হয়েছে বলে current commit description জানায়। এই variants display formatting, serialization, IPC protocol এবং UI mapping-এ consistently support করতে হবে। Unknown remote error code generic error fallback পাবে, panic নয়। User-facing message localization layer-এ থাকবে, machine code stable। `UnsupportedOperation` এবং `BackendUnavailable` আলাদা: unsupported means API has no implementation in backend; unavailable means backend could support it but not ready/present/permissioned. `PermissionDenied` should not be folded into generic I/O error. `Timeout` does not imply operation was cancelled; call state may need reconciliation.

## F.5 mandatory tests

- Every backend has a capability contract snapshot test.
- Fake backend always emits `simulated=true` where data is synthetic.
- No live backend returns a fixed data value as live without explicit mock flag.
- Event ordering tests include duplicate, delayed, out-of-order and missing completion.
- Timeout/cancel tests ensure buffers and resources are released.
- Serialization tests ensure every error variant round-trips across JNI/IPC boundaries.
- `auto()` selection tests prove an unavailable host backend cannot silently fall back to an unmarked fake backend in release mode.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী G: interface contracts ও backward compatibility

OnuronOS-এর অনেক component আলাদা crate/process হওয়ায় API compatibility ইচ্ছাকৃতভাবে পরিচালনা করতে হবে। Crate-level Rust types, `nilprotocol` messages, `.nilax` manifest schema, NilLang bytecode, Alap component declarations, service configs, JNI JSON events, target profiles এবং build manifests—সবই versioned interfaces। এক জায়গায় field rename করলে packages, host app বা services silently break করতে পারে।

## G.1 protocol version policy

প্রতিটি wire/schema interface-এ `major`, `minor` বা `schema_version` থাকবে। Major incompatibility হলে endpoints reject করে clear diagnostic দেবে; minor optional fields backward-compatible ভাবে parse হবে। Unknown enum state-এর behavior define করতে হবে। JSON parser unknown field ignore করবে নাকি error করবে—security boundary অনুযায়ী explicit choice। For hardware commands, unknown command must reject; telemetry consumer unknown optional field ignore করতে পারে। Request ID/boot ID/event sequence format stable হবে।

## G.2 migration policy

Services config, publisher trust store, app permission database, UID registry, OOBE config, notes data এবং boot counter file format পরিবর্তন হলে schema versioned migration লাগবে। Migration idempotent, journaled এবং recoverable হবে। Old file overwrite করার আগে backup/quarantine; corrupt database silent reset নয়। Downgrade scenario-তে older app/OS new schema পড়তে না পারলে behavior defined হবে। User data migration-এর জন্য “successful update” দেখানোর আগে durable state transition হবে।

## G.3 compatibility matrix

Release note-এ current system version, supported NilLang bytecode versions, `.nilax` schema versions, Alap framework ABI, `nilprotocol` version, Android host protocol version এবং device profile versions লেখা থাকবে। App store catalogue package minimum OS/API constraints নির্ধারণ করবে। Compatibility promises শুধুমাত্র tested combinations-এর জন্য; future compatibility assumed নয়।

## G.4 acceptance criteria

- Every persisted/wire format has schema version and migration tests.
- Incompatible clients fail predictably instead of crashing.
- Downgrade behavior documented; irreversible migrations blocked or backup provided.
- Release artifacts report all relevant interface versions.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী H: field validation ও physical lab protocol

QEMU এবং Android emulator automated validation-এর জন্য প্রয়োজনীয়, কিন্তু device-specific interaction যাচাই করতে physical lab protocol দরকার। Test recording repeatable না হলে two different runs compare করা কঠিন হবে। Hardware matrix-এর প্রতিটি row-র জন্য preconditions, procedure, expected output, failure modes, recovery operation এবং evidence list থাকতে হবে।

## H.1 before every physical test

Device model/codename, SKU/region, firmware/build number, bootloader unlock state, current slot, battery percent, charger state, ambient temperature, host OS/platform tools versions, USB connection mode এবং stock restore package hash record করতে হবে। Personal account details, SIM phone number, IMEI, serial number ও Wi-Fi password public logs-এ প্রকাশ নয়। Device backup intact কিনা verify করতে হবে। Test command destructive কিনা, userdata wipe হতে পারে কি না এবং recovery mechanism আছে কি না preflight করবেন। A developer who cannot explain the restore path should not execute a flash command.

## H.2 boot test

Boot test-এ cold boot এবং warm reboot আলাদা। Serial/console output capture, kernel version/commit, initramfs revision, rootfs manifest, first panic/fatal message এবং boot elapsed timings record হবে। Bootloader warning state, slot identity এবং kernel cmdline sanitized but readable format-এ থাকবে। Boot to `nilinit` is one criterion; boot to service-ready is next; boot to UI is another. If display absent but serial works, the boot result is `kernel+userspace booted, display not ready`, not “failed OS” or “UI succeeded.”

## H.3 storage test

Device-specific storage tests should avoid destructive full-disk or raw write benchmarks until there is a safe test area. Start with read-only identify and mount probe, then disposable test file, fsync, clean reboot and hash check. Test low free space, unclean shutdown, filesystem journal recovery, readonly remount, permission/ownership, encryption status and app data isolation. On devices where storage is part of userdata, restore original data after validation. Never run `mkfs` against an ambiguous candidate block device. Device node path must be verified by udev/sysfs mapping and target profile; not guessed from `/dev/vda` or `/dev/sda`.

## H.4 display/touch test

Display validation includes connector/panel detection, resolution/refresh rate, orientation, brightness, blank/unblank, suspend/resume, red/green/blue/white/black test patterns, text glyphs and redraw after process restart. Touch validation records raw coordinates and mapped logical coordinates, single/double taps, long press, swipe, multi-touch if hardware supports, edge gestures and touch release/cancel events. Calibration data belongs to the device profile. Photograph/screenshot is useful but should not be the only proof; logs and event capture establish input actually reached the compositor.

## H.5 camera/audio/network/peripheral test

Camera test: enumerate physical camera IDs, open, stream frames, verify resolution/format/timestamp, capture, save through user-approved storage, close, repeat after pause/resume; assert test frame not returned as live image. Audio: play deterministic tone at low safe volume, measure underruns/latency, route to speaker/headset, record a consented local sample and verify data format; no voice data in public artifacts. Wi-Fi: scan only with valid permission, connect to controlled SSID, confirm IP/default route/DNS and HTTP/TLS reachability, disconnect and reconnect. Telephony tests do not place real emergency calls, and SMS test messages use controlled recipients and explicit consent. Bluetooth pairing uses controlled test accessory and reset procedure. Each subsystem’s absence yields “not supported/unavailable” not false success.

## H.6 recovery test

Before any test likely to change boot/system partition, execute stock boot and recovery access checks, verify backup hashes, check charge level, and confirm restore host can see device. If boot test fails, avoid rapid random flash attempts. Gather logs, identify current slot, use known recovery procedure and verify restored stock system boot and data state. Recovery documentation should say what was physically tested and what is inferred from source/docs. A procedure that has not been executed stays untested.

## H.7 acceptance criteria

- Every physical test script has preconditions, expected results and restore plan.
- All artifacts sanitize user/device identifiers.
- Repeated runs and failures are logged, not only successful screenshots.
- No destructive formatting without explicit target verification and human confirmation.
- Hardware matrix maturity updated only after test record is reviewed.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী I: UI design system ও screen specification governance

Shell-এ নতুন screen তৈরির গতি বেশি হতে পারে, কারণ screen text ও state fields দ্রুত যোগ করা যায়। কিন্তু design system না থাকলে একই system function আলাদা app-এ ভিন্ন semantics পাবে। Buttons, errors, dialogs, status badges, SIM labels, loading states, empty states, confirmation prompts এবং accessibility roles-এর reusable design token থাকা উচিত।

## I.1 semantic state first

প্রতিটি screen প্রথমে state model define করবে: `loading`, `available`, `empty`, `permission_denied`, `error`, `simulated`, `offline`, `saving`, `saved`, `unsupported` ইত্যাদি। তারপর UI states map হবে। Data source unavailable হলে placeholder content দিতে চাইলে explicit demo toggle লাগবে; otherwise empty/error view. Status badge শুধু decorative নয়; badge text accessible tree-তেও থাকতে হবে।

## I.2 shared tokens

Spacing, typography scale, color roles, elevation/shadow, icon style, motion duration, safe area, top/bottom navigation এবং touch target size central token package-এ থাকবে। Theme পরিবর্তন, font scaling, Bengali localization এবং high contrast যেন সমস্ত screens-এ consistent হয়। UI screenshot tests canonical viewport, device density এবং font version record করবে। Pixel-perfect screenshot test brittle হতে পারে; geometry/semantic test-ও রাখুন।

## I.3 screens need real contracts

Camera screen “Sony IMX 50MP” দেখালে source capability response থেকে model info আসতে হবে; otherwise it is example content labeled as a demo. Notes list database query থেকে আসে; save status durable transaction result থেকে। Music track duration media metadata থেকে, play state audio playback callback থেকে। Calculator number field sanitized parser-এ যায়। Settings status kernel/service endpoint থেকে আসে। Shell Terminal `services` command supervisor status endpoint query করে। এই contract না থাকলে screen UI-only prototype, তা ঠিকভাবে labelled থাকবে।

## I.4 acceptance criteria

- Screen state matrix documented for all dynamic screens.
- All operation success labels backed by result callback/persistent operation.
- localization/accessibility badges and controls shared across app screens.
- demo data separated from production datasets and prominently marked.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী J: “কম কিন্তু সম্পূর্ণ” vertical slice-এর পূর্ণ test script

Project-এ এটাই সবচেয়ে বেশি leverage দিতে পারে। একটি sample app `org.onuron.hello` ধরে এই test pipeline তৈরি করুন।

## J.1 build phase

1. Clean checkout and source revision capture.
2. Compile `.nil` source using `nilc`.
3. Validate diagnostics and inspect bytecode metadata.
4. Create signed `.nilax` with test publisher key only for CI fixtures.
5. Verify package with trusted public key store.
6. Attempt package with untrusted key and verify rejection.
7. Install valid package to disposable app root and validate payload hashes.

## J.2 launch phase

8. Start system/QEMU target to ready state.
9. Invoke `nilrt-launch org.onuron.hello` as the actual installed entrypoint, not directly call `NilVM::load_package` from the test process.
10. Validate per-app UID/GID, namespaces, mount table, permissions, seccomp status and data root.
11. Ensure untrusted app cannot read a sentinel file placed outside its app root.
12. Ensure denied permission makes service request fail with `PermissionDenied`.

## J.3 UI phase

13. Runtime loads bytecode and creates a UI scene through Alap/NilUI bridge.
14. Renderer layout computes test scene bounds.
15. Compositor draws into a known framebuffer.
16. QEMU test renderer captures frame hash or screenshot.
17. Input test injects one tap on button bounds.
18. Callback mutates `@State` string/counter.
19. Next frame shows updated state; frame sequence number advances.
20. Unrelated tap does not activate the button; cancelled pointer does not create click.

## J.4 lifecycle/data phase

21. App writes a small preference to app-private persistent data only when permitted.
22. App pause/resume does not duplicate callbacks or lose a committed write.
23. App dispose releases resources; child process exits or transitions according to lifecycle.
24. Restart app and verify persistent state. 
25. Kill VM mid-operation and check system remains alive; state is either last committed version or explicit recovery outcome.

## J.5 package update phase

26. Create version 1 and install; launch and verify.
27. Upgrade to signed version 2; validate atomic swap.
28. Inject crash at each transaction phase; relaunch manager and verify recovery.
29. Roll back to version 1 and verify integrity.
30. Attempt downgrade/untrusted publisher/tampered payload and confirm rejection.

## J.6 result semantics

The test should emit structured JSON with `compile`, `signature`, `install`, `sandbox`, `render`, `input`, `state_update`, `persistent_data`, `crash_recovery`, `upgrade`, `rollback` statuses. The test can pass overall only if every mandatory status passes. This vertical slice will validate major architecture interfaces across NilLang, package management, runtime, system services and UI in a way that a multitude of unrelated unit tests cannot. It will also guide Alap API design by showing real application needs.

# অতিরিক্ত বাস্তবায়ন-সংযোজনী K: daemon-by-daemon integration backlog

এই সংযোজনী প্রতিটি daemon-এর জন্য minimum viable production-shaped contract নির্ধারণ করে। “production-shaped” মানে এখনই production maturity নয়; বরং interface, errors, tests এবং evidence এমনভাবে ডিজাইন করা, যাতে পরের backend যোগ করার সময় fake semantics সরিয়ে ফেলতে না হয়।

## K.1 `nild`, `nilbus` এবং `nilkeyd`

`nild`-এর role system-level service coordination ও central health/status orchestration-এর সঙ্গে যুক্ত হলে service registry, request authorization, daemon identity, configuration version এবং audit events এক জায়গায় পরিষ্কার হওয়া দরকার। `nilbus` message framing, backpressure, peer credentials, bounded queues, version negotiation এবং connection lifecycle-এর source of truth হবে। `nilkeyd` private signing keys, publisher public keys, device/system keys এবং application access-কে পৃথক করতে হবে। App process private OS root key পড়তে পারবে না; key use privileged broker function-এর মাধ্যমে authorized signing/decryption operation হিসেবে expose করা উচিত। Key daemon unavailable হলে signed package install/update fail হবে; signature check bypass নয়।

Test: daemon absent; protocol mismatch; forged peer UID; excessive requests; malformed payload; untrusted app requesting system key; restart with active connections; key-store corruption; signing key rotation. Acceptance: service reports actual ready status, unauthenticated IPC request rejected, no key material appears in logs, system key API cannot be called directly by app sandbox.

## K.2 `netd`

`netd` API network state, connectivity, DNS, Wi-Fi scan/connect, routing and interface list expose করতে পারে। Read-only telemetry phase আগে; configuration phase পরে। Interface presence, link, IP address, default route, DNS response and actual internet validation আলাদা field। QEMU virtual NIC test করলে backend status “QEMU virtual network”; Android host uses ConnectivityManager; native phone uses Linux APIs/supplicant or modem-aware backend। Operation ফল actual network stack থেকে আসে।

Test: interface absent, link down, DHCP timeout, captive portal, DNS failure, VPN active, permission denied, Android connectivity callback out of order, daemon restart during active scan. Acceptance: no false internet success; passwords redacted; state timestamps update; host/native status sources labelled.

## K.3 `powerd`

`powerd` battery telemetry, screen timeout, performance mode and wakelock governance manage করতে পারে। কিন্তু static default battery values development-only। `BatteryInfo`-তে `is_simulated` field আছে বলে maturity file-এ উল্লেখ; UI ও status API-কে এই flag respect করতে হবে। `capacity` range 0–100, temperature units Celsius, voltage/current units, charging status enum and sensor source documented হবে। Unsupported property `0` হয়ে গেলে user misled হতে পারে; `Option`/availability bit ব্যবহার করতে হবে।

Test: no battery sysfs; invalid numeric sensor value; charging transition; temperature unavailable; wakelock leak; duplicate acquire/release; screen timeout with activity; forced suspend/resume. Acceptance: unknown battery state reported unknown, wake locks have owner and bounded lifetime, system doesn't claim performance mode applied if backend rejected.

## K.4 `audiod`

`audiod` audio focus arbitration, stream routing, volume curves, ALSA discovery এবং Android hosted playback/record coordination করবে। IPC command volume set করলে hardware volume actual response অনুযায়ী state update হবে। Supported route values speaker/earpiece/headset/Bluetooth may not be present on each backend. `AudioRoute` state must expose available and active route separately. Emergency/voice call priorities can preempt media but need actual telephony event source. Queue overflow/underrun metrics and route failures must not crash the daemon.

Test: no sound card, device busy, format unsupported, output open failure, focus preemption, headset remove, Bluetooth route drop, AudioTrack write error, recording permission revoked, queue overrun. Acceptance: deterministic tone plays on supported test device; unsupported route gets explicit reason; no raw audio logging.

## K.5 `camerad`

`camerad` should enumerate supported devices, expose capture modes, control torch and manage preview/capture lifecycle. Linux V4L2 enumeration is not enough; actual image format, size, color conversion and stream result need tests. Android hosted uses Camera2 and must not route a fake Rust fallback JPEG through an interface that indicates physical camera. A `CameraFrame` record should include source backend, frame timestamp, dimensions, pixel format/compression, simulated flag and request ID. Memory-limited streaming needs backpressure; app stops receiving frames should release buffers. 

Test: no devices, permission denied, camera busy, format renegotiation, frame timeout, invalid frame, stop preview during capture, process death during open. Acceptance: real capture test verifies decoded frame is non-test content; backend does not mark test-pattern as real; torch state reflects actual host confirmation.

## K.6 `telephonyd`

`telephonyd` must distinguish simulated manager, Android host API, AT serial, QMI/MBIM and any other real modem transport. SIM-ready, registered, signal strength, outgoing call and SMS message status all require source and update time. AT command parser should remain a standalone component, but it cannot imply that a modem is connected. Carrier names, number, SIM slot and call history require privacy. `ATH`/hang-up command only reports success after active call backend acknowledges; model-only state changes are simulation. 

Test: no modem, device busy, SIM locked, no network, registration denied, AT timeout, malformed unsolicited line, SMS send callback false, duplicate request, host permission denied. Acceptance: active-call state sourced from modem/Android callback, not `dial()` returning; SMS not considered delivered from enqueue success.

## K.7 `inputd`

`inputd` abstracts raw event devices, Android MotionEvent and higher-level gestures. Device drivers and input method are different concerns. Raw touch timestamps, transform matrix, pointer IDs, cancel states and key codes must cross the bridge. `inputd` should not decide app actions directly; it routes events to compositor/focus manager. A user app only receives authorized event stream, not other apps' raw input. Accessibility activation should produce equivalent semantic events.

Test: pointer down-up, cancelled gesture, multi-touch pointer reset, rotation, surface resize, host pause, evdev disconnect, key repeat and event flood. Acceptance: no double tap action from duplicate Java/local+Rust handlers; focus only goes to one app; no raw input leakage between apps.

## K.8 `nilupd`, `nilrecovery`, `nilwdt`

`nilupd` should validate signed update metadata and coordinate staging; `nilrecovery` should expose safe menu/status, with dangerous write operations protected; `nilwdt` should monitor a defined set of required services and avoid feeding the watchdog indefinitely when system is irrecoverably stuck. Watchdog reset policy should not create repeated destructive loops. Boot counter and watchdog reasons should be durable yet resistant to corrupt-file reset. `nilrecovery` must not autoformat storage when recovery enters; diagnosis comes first. 

Test: update signature invalid, slot switch error, boot health fails after OTA, watchdog device absent, watchdog write failure, bad boot counter, recovery menu crash. Acceptance: rollback bounded; recovery doesn't silently wipe userdata; watchdog status truthful; failure reason logged.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী L: target-ভিত্তিক configuration এবং command consistency

প্রকল্পে একই target-কে `x86_64-generic`, `qemu-x86_64`, `aarch64-qemu`, `aarch64-generic`, `arm64-generic`, `oneplus-fajita` ইত্যাদি নামে reference করা যেতে পারে compatibility-র কারণে। কিন্তু alias policy না থাকলে `build.sh` one directory write, smoke runner another directory read, flasher third directory expect—এমন সমস্যা হয়। Canonical names external API হিসেবে স্থির করতে হবে এবং legacy aliases শুধু normalization stage-এ গ্রহণ করতে হবে।

## L.1 target spec schema

Proposed profile fields:

```toml
id = "qemu-aarch64"
kind = "qemu"
architecture = "aarch64"
rust_target = "aarch64-unknown-linux-musl"
output_dir = "out/qemu-aarch64"
kernel_config_id = "alpine-3.19-aarch64-netboot-pinned"
kernel_sha256 = "<verified hash>"
initramfs = "initramfs.cpio.gz"
data_image = "data.img"
data_fs = "ext4"
boot_validation = "required"
persistence_validation = "required"
flash_allowed = false
release_allowed = false
```

এই example illustrative; current repo output paths এ মুহূর্তে `out/aarch64-qemu`, তাই refactor করার আগে alias/path migration plan প্রয়োজন। Existing developer artifacts মুছে ফেলার বদলে compatibility mapping অথবা explicit clean/rebuild strategy লিখতে হবে। No source code should silently read old path once canonical path changes. 

## L.2 CLI design

প্রতিটি script help output-এ example, destructive behavior, default target ও output paths উল্লেখ করবে। Unknown target alias argument error দেবে। `--dry-run` build toolে resolved configuration print করবে; flash tool-এ zero device writes. `--force` কোথায় ব্যবহৃত হয় তা clear হবে: disk formatting force, unknown target force, checksum skip—এসব একই ambiguous `--force` option-এ থাকা উচিত নয়। `--skip-kernel-download` missing kernel হলে failure; checksum check skip নয়। Nonrelease fake-binary flag `--allow-host-binaries-for-tests` release profile-এ নিষিদ্ধ থাকবে।

## L.3 command plan manifest

Build orchestrator resolved target spec-কে JSON-এ লিখবে before build; artifact manifest build success-এ generated হবে। Command path/args/exit code record করা যেতে পারে, তবে sensitive env vars redact করতে হবে। Cache key target triple, Cargo lock, rustc version, build flags, kernel hash and profile hash-এর উপর নির্ভর করবে। Wrong cache restore হলে artifact checksum/ELF tests fail করবে। Reusing Cargo build cache acceptable; reusing unrelated output image without matching source revision নয়।

## L.4 acceptance criteria

- Single canonical target registry consumed by builder, smoke tests, disk builder and flasher.
- Legacy aliases covered by tests and deprecation notes.
- Resolved target manifest/plan available for each build.
- Conflicting profile/CLI args fail with explicit error.
- Destructive options separately named and covered by negative tests.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী M: app permission model এবং capability broker-এর end-to-end semantics

Permission broker শুধু settings screen-এ toggle দেখানোর ব্যবস্থা নয়। এটি app identity, publisher trust, requested permissions, user decision, runtime grant, lifetime, revocation এবং backend operation-এর enforcement point। `requested_permissions()` যদি package manifest থেকে permission নেয়, trusted app identity-এর সঙ্গে তা bind করতে হবে; malicious app নিজেকে system app হিসেবে declarative manifest-এ ঘোষণা করলেই elevated permission পাওয়া যাবে না।

## M.1 permission categories

Permissions explicit data access and device access group-এ ভাগ হতে পারে: `storage.read`, `storage.write`, `media.read`, `network`, `camera`, `microphone`, `location`, `telephony.call`, `telephony.sms`, `contacts.read`, `notifications.post`, `bluetooth.scan`, `bluetooth.connect`, `clipboard.read`, `background.execution`, `system.settings.modify`. Each has risk level, prompt text, persistent/session grant policy, revocation effect and backend support requirements. Fine-grained permissions future work হতে পারে; initial model-এ permission semantics clear থাকতে হবে।

## M.2 authorization flow

App requests capability → launcher/runtime authenticates app ID + signing publisher → broker reads requested permissions → policy checks user/system admin grants → user prompt where required → token/scoped handle issued → daemon validates caller credentials and token → backend performs operation → result callback/event → logs audit event without sensitive data. Calling service directly through a raw socket must not bypass broker. IPC peer credentials authenticate process, while scoped capability/permission state authorizes the operation. For Android hosted mode, both OnuronOS permission and Android runtime permission must be granted: host denial means no device access even if guest app granted permission internally.

## M.3 revocation and lifetime

A permission revoked while camera preview is active should close or stop the session. Microphone revocation stops capture. Network revocation may terminate current socket or prevent new requests according to policy. Background permission expires on app stop/idle if grant is temporary. Permission database schema is durable; atomic writes and corrupt-database quarantine are mentioned in completion checklist, but runtime revocation propagation and active-handle invalidation need tests. Future-dated timestamps/clock skew should not create indefinite grants. App reinstall/update or publisher key rotation should preserve/review grants according to signature identity, not app name alone.

## M.4 acceptance criteria

- Manifest request is not treated as user grant.
- Backend service checks authorization independently of UI.
- Android-host permissions and OnuronOS broker permissions are both enforced.
- Revocation cancels/restricts active handles.
- Permission store tampering/corruption does not grant all permissions by default.
- Audit logs expose app ID/capability/result but not secret payloads.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী N: release channel, versioning ও support policy

একটি public open-source OS-এর জন্য version number এবং release channel শুধুমাত্র marketing label নয়; কোন artifact কতটা পরীক্ষিত, কোন target supported এবং app/API compatibility কী—এসব বোঝায়। `0.x` development builds, `preview`, `nightly`, `beta`, `rc` এবং stable semantics থাকলে change policy নির্ধারণ করতে হবে। Preview ও stable artifact signing keys আলাদা রাখা যায়। Nightly build-এ experimental flag থাকতে পারে; official stable app publisher trust store-এ arbitrary nightly key automatically trusted হবে না।

## N.1 semantic versioning surfaces

System version, NilLang language version, bytecode version, Alap runtime API, `nilprotocol`, `.nilax` package manifest schema, device profile version, boot image tool version এবং OTA manifest version পৃথক versioned API হতে পারে। OS semantic version বদলালে সব API version একই সঙ্গে বদলাতে হবে এমন নয়। Breaking changes release note ও compatibility table-এ উল্লেখ হবে। User app binary/bytecode supported version range mismatch হলে launch/install clear error দেবে।

## N.2 release notes

প্রতিটি release note-এ supported target, exact tests passed, known failures, simulated subsystems, physical hardware verified list, security caveats, data migration, rollback procedure, signing key info এবং bug-report instructions থাকবে। “Camera support added” বদলে “Android-hosted Camera2 capture tested on API X/device Y; native `camerad` still planned” লেখা যথাযথ। “Verified boot” বদলে “signed integrity descriptor prototype; bootloader-enforced AVB not integrated” বলা উচিত যতক্ষণ full chain নেই।

## N.3 support lifecycle

Kernel LTS/firmware updates, dependency CVEs, device profile maintenance, app compatibility window এবং release artifact availability policy রাখতে হবে। Old test builds revoke করা লাগতে পারে যদি key compromised হয়। Device support list explicit: unsupported model-এ generic flash try করার পরামর্শ দেওয়া যাবে না। Rollback image/stock restore process link release artifact-এর সঙ্গে থাকবে। Maintainer capacity limited হলে supported device count intentionally one থাকবে; unsupported device broad claims থেকে avoid করতে হবে।

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী O: practical prioritization matrix

যখন দুই কাজের মধ্যে সিদ্ধান্ত নিতে হবে, শুধু code effort নয়, dependency leverage ও failure impact দিয়ে compare করুন।

| কাজ | User-facing visibility | Dependency leverage | Failure risk | অগ্রাধিকার |
|---|---:|---:|---:|---|
| ARM64 `nilhal` compile fix | কম | অত্যন্ত বেশি | উচ্চ | P0 |
| ARM64 boot/persistence CI | মাঝারি | অত্যন্ত বেশি | উচ্চ | P0 |
| Flash safety/target map | কম | অত্যন্ত বেশি | অত্যন্ত উচ্চ | P0 |
| More calculator/music UI polish | বেশি | কম | কম-মাঝারি | P3 |
| `nilrt-launch` actual end-to-end run | মাঝারি | অত্যন্ত বেশি | উচ্চ | P1 |
| Alap Button state-change path | বেশি | বেশি | মাঝারি | P1 |
| S25 battery/network telemetry contract | বেশি | বেশি | মাঝারি | P1 |
| Physical camera backend with real capture | বেশি | মাঝারি | মাঝারি | P1/P2, after bridge |
| Multi-phone support | বেশি | কম before first phone | অত্যন্ত উচ্চ | defer |
| OTA A/B physical rollout | medium | high but hardware-specific | very high | after verified boot and recovery |

এই matrix-এর “visibility” মানে কাজের গুরুত্ব কম-বেশি নয়। Visible UI change demo দেখাতে সাহায্য করে; critical infrastructure invisible হলেও subsequent work-এর ভিত্তি। P0/P1 must be unblocked first.

---

# শেষ verification note

এই পরিকল্পনায় সর্বশেষ অডিট-করা commit `be64e988a9a2a9658b300496f3fb601dd4aeb798` এবং workflow run `38024019829`-এর অবস্থাকে baseline হিসেবে ধরে লেখা হয়েছে। সর্বশেষ visible Linux/QEMU run-এ x86_64 job সফল, ARM64 job `runtime/nilhal/src/lib.rs`-এর C pointer type mismatch-এ ব্যর্থ। এই দলিলের build commands, code patterns, future profiles এবং acceptance schemas প্রস্তাব; এগুলো বর্তমান repository-তে সবই implemented আছে বলে দাবি করা হয়নি। নতুন commit এলে baseline ও priorities পুনরায় যাচাই করতে হবে।

# অতিরিক্ত বাস্তবায়ন-সংযোজনী P: issue-কে mergeable কাজের আকারে ভাঙা

এই backlog item-গুলোকে GitHub issue হিসেবে ব্যবহার করা যায়। প্রতিটির output ছোট রাখলে root cause isolate হয় এবং contributor নতুন subsystem-এর পুরো architecture না জেনেও একটি নির্দিষ্ট defect fix করতে পারেন। নিচের তালিকার issue titles proposed; এগুলো repository-তে ইতিমধ্যে open issue আছে বলে বোঝায় না।

## P.1 Build system issues

**BUILD-01 — Fix NilHAL AArch64 C ABI char-pointer mismatch.** Change: plugin ABI `name` field and `CStr` use. Tests: x86_64+ARM64 compile; invalid/null module name. Output: green AArch64 build.

**BUILD-02 — Add architecture-specific negative tests for rootfs.** Tests: host x86 ELF passed to AArch64 rootfs; missing `nilinit`; non-ELF script with invalid interpreter; wrong-architecture optional daemon. Output: builder refuses unsafe image.

**BUILD-03 — Add canonical target registry.** Change: target spec and alias mapping. Tests: all CLI aliases, output paths, conflicting args. Output: build tools agree on canonical target.

**BUILD-04 — Make release build reject stale artifacts.** Change: start revision/manifest binding and output clean policy. Tests: old manifest, stale kernel, missing image. Output: old artifacts cannot pass new build.

**BUILD-05 — Add separate `mkdisk --real` CI target requirement.** Tests: no e2fsprogs, synthetic mode, existing real image, wrong size. Output: required persistence gate always uses real filesystem.

**BUILD-06 — Generate artifact provenance manifest.** Include toolchains, revision, hashes and test IDs. Tests: schema validation, unknown revision, artifact hash mismatch. Output: each image has provenance.

**BUILD-07 — Fix target profile selection in `build/build.sh`.** Tests: `qemu-x86_64`, `qemu-aarch64`, `fajita`, `android-host-arm64`; assert exact target triple and output folder. Output: no accidental host target.

**BUILD-08 — Add `--dry-run` and safe path validation to image tools.** Test absolute path outside workspace, `..`, symlink outdir. Output: path operations cannot delete or overwrite arbitrary user files unintentionally.

## P.2 Boot/storage issues

**BOOT-01 — Replace mount-success unconditional log.** Tests: mock mount success/failure, QEMU missing `/dev`, required mount unavailable. Output: logs reflect syscall status.

**BOOT-02 — Introduce structured boot-status schema.** Fields: boot ID, state, required services, persistence, SELinux, cgroups, UI readiness. Tests schema version and failure states. Output: no string-only success signal.

**BOOT-03 — Implement service readiness response.** Every core daemon returns `READY` after actual initialization. Test delayed startup and process alive/not-ready. Output: init waits bounded interval.

**BOOT-04 — Prevent supervisor loop blocking on one service restart.** Replace sleep-based restart delay with scheduled deadline. Test simultaneous crash events. Output: unaffected services continue supervision.

**BOOT-05 — Implement real persistent disk reboot test for ARM64.** Test unique durable marker and filesystem health. Output: evidence says live runtime passed, not only superblock parsed.

**BOOT-06 — Make required-persistence mode fail on tmpfs fallback.** Test mount failure and normal developer mode. Output: release and QEMU persistence gate cannot report false success.

**BOOT-07 — Add storage corruption recovery policy.** Test corrupt volume, absent partition, read-only remount. Output: no autoformat of unknown data.

## P.3 Security/runtime issues

**SEC-01 — Atomic, locked UID registry.** Test concurrent process allocation, kill during rename, malformed registry and duplicate IDs. Output: registry not lost or collision-prone.

**SEC-02 — Enforce strict sandbox defaults in release.** Test `strict_permissions=false` configuration in release; critical `no_new_privs` failure. Output: launch rejects insecure config.

**SEC-03 — Verify active cgroup controllers.** Test no cgroup2 mount, controller missing, app assignment. Output: resource isolation status is truthful.

**SEC-04 — SELinux release fail-closed mode.** Compile policy, load/readback enforce, AVC test. Output: release boot fails/recovery on required policy failure.

**SEC-05 — Audit socket activation inheriting FD.** Service-side tests consume fd 3. Output: all configured sockets have verified consumers.

**SEC-06 — App package launch uses actual nilrt runner.** Change native app slice test; assert namespaces and UID isolation. Output: end-to-end launch uses runtime lifecycle.

**SEC-07 — Add package signature and update crash matrix.** Inject power/process kill at each transition. Output: install/upgrade/rollback recover transactionally.

**SEC-08 — Add release-only test-mode prohibitions.** Fake UID overrides, fake HAL, unverified signatures, host binaries and synthetic disk are explicitly blocked where required. Output: CI verifies no dev backdoor reaches release.

## P.4 UI/platform issues

**UI-01 — Decide Android-hosted rendering authority.** Document Java Canvas vs Rust frame pipeline. Output: no duplicate action path or ambiguous display claim.

**UI-02 — Implement actual sample app event loop.** Button tap changes state, re-renders and presents. Output: `.nil` interaction end-to-end.

**UI-03 — Create minimal Alap crate.** App lifecycle, state, components, navigation API. Output: tested public package; maturity remains experimental until integration gate.

**UI-04 — Add NilUI geometry and screenshot tests.** Fixed viewport, fonts and test scene. Output: regression detects layout/pixel bugs.

**UI-05 — Bound image/camera/audio queues.** Tests overflow and slow consumer. Output: memory does not grow indefinitely.

**UI-06 — Replace false network constants in AndroidHost HAL.** Host event carries live source/time; permission missing returns unavailable. Output: actual telemetry reaches status layer.

**UI-07 — Separate test camera JPEG from live capture.** Add frame metadata and test mode. Output: UI never treats synthetic image as photo.

**UI-08 — Bind audio daemon to actual AudioTrack/AudioRecord.** Integration tone/record test and lifecycle. Output: sound verified, unsupported status accurate.

**UI-09 — Correct telephony operation states.** Dialer launch distinct from call active; SMS queued/sent/delivered statuses. Output: no simulated VoLTE claims.

**UI-10 — Make shell terminal commands truthful.** `python` actual interpreter or unsupported; `ls/cat/cd` confines to allowed paths. Output: no fabricated command success or path escape.

## P.5 Native device issues

**PHONE-01 — Inventory exact OnePlus 6T variant and firmware.** Output: verified identity and profile draft.

**PHONE-02 — Collect and validate stock recovery bundle.** Test hash and restore procedure. Output: physical recovery ready.

**PHONE-03 — Build board-specific kernel and DTB.** Output: artefacts are device-specific and provenance recorded.

**PHONE-04 — Correct `mkbootimg --profile fajita` integration.** Tests with independent Android boot-image tooling. Output: header fields align with target profile.

**PHONE-05 — Discover exact partition map and active slot.** Output: no assumptions based on generic partition name.

**PHONE-06 — Disable flasher by default and verify no-write preflight.** Test mock fastboot responses. Output: invalid target never writes.

**PHONE-07 — Add physical serial boot report.** Output: kernel, initramfs and nilinit milestone evidence.

**PHONE-08 — Bring up storage, display and touch separately.** Output: per-subsystem hardware matrix records rather than one giant “phone supported” label.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী Q: end-to-end data flow diagram ও failure ownership

একটি component diagram শুধু system-এর সুন্দর গঠন দেখায়; failure কোথায় detect হবে এবং কে recover করবে সেটি না থাকলে debugging difficult হয়। নিচে প্রতিটি core flow-এর ownership দেওয়া হলো।

## Q.1 Boot flow

```text
Bootloader
  → target-specific Linux kernel + DTB/DTBO
  → initramfs unpacked, /init invoked
  → nilinit mounts required filesystems
  → persistent /data validated
  → SELinux/cgroup policy verified for target
  → services.toml schema validated
  → required daemons spawned
  → service readiness handshake
  → compositor and shell ready
  → boot marked successful / rollback counter updated
```

Owner boundaries: bootloader/image format error belongs to device profile/build tools; kernel panic belongs to kernel/DTB config; `/init` execution failure belongs to initramfs packaging and binary ABI; mount/storage failure belongs to early boot storage policy; service protocol failure belongs to daemon/readiness contract; frame presentation failure belongs to compositor/backend. `nilinit` should not hide underlying errors by printing generic “boot failed” alone; it should preserve structured cause and phase. Recovery action depends on phase: missing `nilinit` might require alternate initramfs; data corruption may boot read-only recovery; failed UI should still leave serial/recovery path.

## Q.2 Native application flow

```text
source.nil
  → nilc compile/type-check
  → bytecode + app manifest
  → nilpkg package/sign
  → trusted key/signature verify
  → atomic install
  → nilrt-launch validate app ID and permissions
  → allocate UID/GID and isolate namespaces
  → NilVM executes bytecode
  → Alap builds component graph
  → NilUI layout/raster
  → compositor presents frame
  → inputd routes user event
  → VM event handler updates state
```

A failure at each stage has a single primary owner and error contract. Compiler errors must not become runtime panics; signature failures must not continue to install; sandbox failure must not launch directly; render errors must not be reported as successful display; event handler failure should isolate the app rather than terminate PID 1. Integration tests exercise the flow, not just each isolated component. 

## Q.3 Hosted Android operation flow

```text
Onuron app UI requests capability
  → OnuronOS permission broker checks guest app grant
  → JNI protocol sends request ID + capability + args
  → Java host verifies Android runtime permission / OS policy
  → Android API starts asynchronous operation
  → host callback produces correlated result/event
  → Rust HAL maps host event to stable capability state
  → guest UI updates only after actual result
```

This flow has two independent authorization layers: Onuron guest permission and Android host permission. The host must not assume guest granted means Android has granted. The guest must not assume command enqueue means hardware action completed. Operation IDs are required to map callback responses to requests. If the Activity is gone but foreground service continues, event storage/re-delivery policy must be explicit. If host is destroyed, request becomes cancelled/unknown rather than silently completed.

## Q.4 physical phone update flow

```text
signed release bundle
  → manifest/key/target validation
  → connected device identity + mode validation
  → partition/slot compatibility preflight
  → backup/recovery gate
  → inactive slot write
  → read-back/checksum verification
  → boot-control switch
  → candidate boot and health check
  → mark slot successful or roll back
```

Each arrow is a possible failure. Host-side validation cannot force bootloader trust if device doesn't support it. Therefore target profile and actual bootloader capability are part of architecture, not a final installation detail. A generic fastboot flash script is insufficient to implement the whole flow.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী R: metric thresholds ও reliability dashboards

Metrics-এর numeric threshold target-specific experiment দিয়ে baseline করার পর নির্ধারণ করা উচিত। এখানে proposal হলো কোন metric collect করতে হবে এবং threshold policy কীভাবে বানাতে হবে। Random arbitrary valuesকে universal truth হিসেবে বসানো উচিত নয়।

## R.1 Build metrics

- Clean build duration by target and toolchain.
- Build cache hit rate.
- Artifact size per component and total initramfs.
- Reproducible-build pass rate.
- Failure class histogram: compile, link, checksum, package, boot, readiness, persistence.
- Time from a regression commit to detection/revert.

If build size suddenly increases, CI should retain section-size/binary-size reports and flag a percentage change threshold. A size regression may be justified by feature, but must be reviewed. Host and target binaries are measured separately. Build timing threshold should account for noisy hosted runners, while correctness never gets skipped because the test is slow.

## R.2 Boot and service metrics

- kernel entry to initramfs init, PID1 start, core ready, UI ready.
- service startup/ready latency and timeout count.
- crash/restart count, backoff state, crash loops.
- mount duration/errors, data mode persistent/volatile.
- boot recovery reason and boot counter.

Store the raw logs per CI test and summary metrics in JSON. If service readiness latency increases, test environment and QEMU settings are recorded to distinguish code regression from runner jitter. Required service failing to become ready is a hard failure; latency threshold only makes detection faster and more diagnosable.

## R.3 UI/runtime metrics

- compile/install/launch latency.
- bytecode execution instruction count and limit hits.
- VM memory footprint, event queue depth and dropped events.
- layout/raster/present latency, p95/p99 frame time, dropped frames.
- input-to-present latency and missed/cancelled gestures.
- accessibility tree generation and text shaping errors.

Metrics must not include content of user text fields, camera frames, SMS or audio. Performance reports include target and renderer backend. A fake renderer cannot benchmark hardware display throughput; it measures algorithmic path only.

## R.4 Hosted backend metrics

- JNI round-trip latency, queue depth, protocol mismatch count.
- Android permission denial count and service callback timeout.
- Surface create/destroy cycles and stale frame attempts.
- Audio buffer underrun/overrun, camera capture latency, network update age.

Keep counters local and user-controlled. No telemetry upload by default. If user opts to export debug report, preview/redaction and explicit destination are required. A crash report can contain app IDs/build IDs, not private capability payloads.

## R.5 hardware metrics

- temperature/thermal throttling, battery state/current if accessible, suspend/resume latency, Wi-Fi association stability, audio underrun and camera frame drops.
- Boot success over repeated cold/warm cycles.
- Recovery/rollback success after deliberate failure injection.

Thresholds need exact device model/firmware and ambient setup. A single successful boot is initial evidence; stable hardware support requires repeatability across defined runs and failure modes. If only one prototype device is available, support claim should describe exact known environment rather than general model guarantee.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী S: open-source maintenance এবং contributor onboarding

Open-source OS project-এর engineering quality contributor experience-এর উপর নির্ভরশীল। Build instructions একাধিক script-এ আলাদা হলে newcomer ভুল target build করে; stale binary বা hidden local environment থাকলে CI mismatch ঘটে। তাই contributor onboarding-কে roadmap-এর অংশ করতে হবে।

## S.1 one-page quickstart

Top-level `CONTRIBUTING.md`-এ development prerequisites, target build command, unit test, x86_64 QEMU launch, ARM64 QEMU launch, Android hosted build, artifact output location, clean process এবং known unsupported hardware claims এক পাতায় থাকবে। Full details reference pages-এ থাকবে। README-এর প্রথম screen-এ user project কী, currently tested targets, how to build and safe warnings থাকবে।

## S.2 labels and ownership

GitHub issues-এ `P0`, `P1`, `P2`, `P3`, `area:build`, `area:boot`, `area:runtime`, `area:ui`, `area:android-host`, `area:hardware`, `security`, `needs-evidence`, `simulated`, `good-first-issue` labels রাখা যায়। Maintainer unavailable হলে critical subsystem-এর backup owner বা clear escalation instructions দরকার। Contributor code review-এ exact test command ও result দিতে হবে।

## S.3 reproducible bug reports

Bug report-এ OS host, commit, target, toolchain, build commands, QEMU configuration, log excerpt, expected and actual result, existing artifact directory state থাকবে। “It doesn't boot” report-এ kernel booted but initramfs fail কিনা, `/init` run করেছে কিনা, PID1 output, disk attached কিনা, architecture match কিনা—এসব দরকার। `build/qemu-smoke.py` failure output এই data automatic collect করতে পারে।

## S.4 contributor security

Untrusted pull request from fork should not receive secrets used for signing or deployment. Test-only keys generated ephemerally; secrets never printed. Release signing job runs only on protected branch/tag with stricter review. CI artifacts from untrusted PR should not be able to overwrite official release bundle. Build scripts treat repo inputs as untrusted: arbitrary path/commands from config validated; not shell-eval string. Code review includes license compatibility.

## S.5 acceptance criteria

- Clean quickstart succeeds on documented host environments.
- Contributor can build/test without signing secrets.
- Issue template captures target and test log info.
- Release signing restricted to trusted workflow context.
- Hardware issues cannot be triaged from screenshots alone; logs and target profile requested.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী T: চূড়ান্ত release decision record

প্রতিটি milestone-এর শেষে একটি concise decision record রাখা উচিত। Record-এ `GO`, `GO WITH LIMITS` বা `NO-GO` outcome, evidence links, blockers, accepted risks, approver and next review date থাকবে। এই record roadmap ও actual development status-এর মধ্যে bridge তৈরি করে।

**GO** মানে নির্দিষ্ট target/scope-এর সব mandatory acceptance criteria পাস হয়েছে। **GO WITH LIMITS** মানে scope সীমিত—যেমন “QEMU AArch64 userspace booted, but display and native phone hardware not validated”—এবং release label/README limits clearly show করে। **NO-GO** মানে critical gate fail, যেমন ARM64 compile error, persistence test fail বা invalid flashing profile। No-go কোনও project failure নয়; এটি release engineering-এর safety mechanism।

A `GO WITH LIMITS` record-এ feature list নয়, tested combinations ও exclusions লেখা উচিত। উদাহরণ: “tested on x86_64 QEMU kernel X, initramfs SHA Y, `nilinit` commit Z; real ext4 marker survived three guest reboots; no physical phone boot performed; SELinux enforcing not yet release-verified; Android-host app only tested in emulator.” এ ধরনের বিবরণ পরবর্তী maintainer বুঝতে পারবেন কী নিশ্চিত এবং কী অনুমান।

এই রূপরেখার ভিত্তিতে সবচেয়ে উপযুক্ত প্রথম decision হবে ARM64 compilation fix PR-এর পরে পুনরায় CI চালানো। Build green হলেও QEMU boot/persistence gate না পাস করা পর্যন্ত decision `NO-GO for AArch64 boot claims`, একই সময়ে x86_64 QEMU scope-এর জন্য `GO WITH LIMITS` হতে পারে। Physical phone flashing remains `NO-GO` until device profile, recovery and real verified-boot story have independent evidence. This may sound cautious, but it prevents a working prototype from becoming a dangerous unsupported installer.

# অতিরিক্ত বাস্তবায়ন-সংযোজনী U: user data lifecycle, backup ও recovery contract

Mobile OS-এ persistent data শুধু filesystem mount হওয়ার বিষয় নয়; data creation, ownership, access, backup, upgrade, export, deletion এবং factory reset—সবকিছুর lifecycle থাকতে হবে। Notes app বা package manager দিয়ে প্রথম user data তৈরি হলে system-level policy স্পষ্ট হওয়া দরকার: app data কোন directory-তে থাকে, process UID/GID দিয়ে কীভাবে isolated, permission broker কীভাবে data access grant করে, encrypted state কীভাবে verify হয়, update-এ schema migration কীভাবে হয়, এবং user uninstall করলে data থাকে কি না।

## U.1 data classification

Data classification-এ অন্তত system configuration, app code, app-private data, user documents, contacts/SMS, authentication/cryptographic metadata, logs/crash reports এবং cache পৃথক হবে। App code immutable package directory-তে, app-private data scoped writable directory-তে; user documents user-mediated storage API-তে; system keys non-exportable key service-এর মাধ্যমে; transient cache cleanupযোগ্য path-এ থাকবে। “/data” একক directory হলেও ownership, SELinux labels, permissions, encryption policy এবং backup policy data class অনুযায়ী আলাদা হতে পারে।

## U.2 persistence guarantees

একটি app UI “Saved” দেখাবে কেবল filesystem/API layer durable commit confirmed হলে। `write()` success মানেই power loss-safe write নয়; গুরুত্বপূর্ণ metadata-তে file sync এবং containing directory sync দরকার। Atomic rename helper থাকলেও parent directory syncing, concurrent file write lock, permission ownership ও cross-platform semantics test করতে হবে। Crash-safe transaction journal এবং file-level atomic write আলাদা স্তর; একটি ব্যবহার করলেই অন্যটির সব property পাওয়া যায় না।

## U.3 backup & restore

Backup package-এ user data, app identity, schema version, encryption metadata এবং integrity checks থাকতে পারে; signing key/private key বা raw system trust root export default-এ নয়। Backup restore untrusted input; path traversal, archive bombs, app ID mismatch, version incompatibility এবং duplicate files validate করতে হবে। Restore আগে dry-run summary দেখাবে—কত files, কোন apps, কী conflicts; user consent ছাড়া data overwrite নয়। Restore failure partial transaction রেখে যাবে না।

## U.4 factory reset

Factory reset, app uninstall, storage format এবং recovery factory restoration চারটি আলাদা action. App uninstall app code সরাবে; data retention policy অনুযায়ী user data keep/delete choice; factory reset user profile/system data erase করে, কিন্তু immutable recovery/firmware partition স্বয়ংক্রিয়ভাবে বদলাবে না; low-level partition format সর্বোচ্চ-ঝুঁকির operation। System UI-তে destructive step-এর আগে authentication/confirmation এবং clear consequences থাকবে। Testing mode-এর ephemeral tmpfs data থাকলে user warning থাকা উচিত, কিন্তু সেটিকে factory reset বলা যাবে না।

## U.5 acceptance criteria

- Every user-facing save operation has documented durability semantics.
- Package upgrade and app-data migration are separately testable and rollback-safe.
- Backup/restore validates archive and provides preview before overwrite.
- Uninstall, factory reset, disk format and recovery restore have separate permissions and labels.
- Encryption indicator is grounded in actual device/filesystem state, not config flag.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী V: UI ও service-এর মধ্যে data ownership map

একই data screen state, Rust global variable, Java Activity field, daemon cache এবং disk database—একাধিক জায়গায় stored হলে sources diverge করতে পারে। প্রতিটি field-এর source of truth নির্ধারণ জরুরি।

| Data field | Source of truth | Cache allowed? | Update signal | Failure behavior |
|---|---|---|---|---|
| Battery percentage | `powerd` backend / Android BatteryManager | yes, timestamped | BatteryUpdate | unknown/unavailable with last-update time |
| Wi-Fi connection | `netd` / ConnectivityManager | yes, short-lived | NetworkUpdate | distinguish disconnected from unknown |
| Active call state | modem/Android telephony callback | yes, event-correlated | CallStateChanged | `Unknown`/`DialerPresented`, never optimistic Active |
| Captured camera frame | Camera2/V4L2 capture result | bounded frame cache | CameraFrameReady | error/unavailable; test-pattern marked simulated |
| Music playback | `audiod`/AudioTrack callback | yes, ephemeral | PlaybackState | failure/paused if backend cannot play |
| Notes list | persistent notes service/database | query cache only | NotesChanged | load/save error; no static seeded data except demo mode |
| Installed apps | verified package database | yes | PackageStateChanged | reconcile manifest/hash on load |
| App permissions | permission broker durable database | short-lived token cache | PermissionChanged | deny on corrupt/unavailable store, don't grant |
| SELinux mode | kernel active status | short cache | boot/security status | unknown or boot failure if mandatory |
| Storage encrypted state | real device storage policy | cache with source | StorageStateChanged | report unknown if cannot verify |
| Current refresh rate | actual display backend | dynamic | DisplayChanged | don't use 120Hz constant as live telemetry |

## V.1 event ordering

Events from Android callback, Rust worker and system daemon can arrive out of order. Each source should provide monotonic sequence number or timestamp. UI state reducer compares event generation and rejects older state updates. A `HostPause` arriving after `HostResume` due to queue delay must not keep screen paused. A camera `CaptureCompleted` for a cancelled request must not recreate a disposed screen. A network `onLost` event for an old network should not override a newer `onAvailable` network if callback includes network identity. Test event races deliberately.

## V.2 source fields and stale data

Every nontrivial telemetry snapshot should track `source`, `is_simulated`, `updated_at` and optionally `expires_at`/staleness. Cached data can be displayed as “last known” with age; stale data should not silently become current. If Android host service is offline, last battery value might remain for diagnostics, but status bar should indicate stale/unavailable, not fake a live icon. This also helps QEMU versus physical mode comparisons.

## V.3 state reducers

Prefer a reducer/state-machine for UI state updates over direct mutation from arbitrary threads. Android callbacks post typed events to bridge event loop; Rust HAL maps into platform-neutral event; state reducer updates single source. Local UI interactions can optimistically update purely visual states such as pressed feedback, but operation success (Wi-Fi connected, note saved, capture complete) waits for backend confirmation. For operations with long latency, display `Pending` and allow cancellation or timeout.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী W: repository hygiene ও generated content policy

Repository size-এ build artefact এবং caches accidentally commit হলে clone, review, CI and provenance complicate হয়. A previous commit removed `android-host/Onuron.apk` and tracked `.gradle` cache content, and updated `.gitignore`. That cleanup should become continuous policy, not a one-time cleanup. `.gitignore` is not enough to remove files already tracked; CI audit should inspect the Git index.

## W.1 generated file classes

Generated `.apk`, `.aab`, `build/`, `.gradle/`, `target/`, local SDK config, keystores, private keys, device-specific raw backups, user data, core dumps and generated large kernel/system images belong outside source unless the repository explicitly distributes a reviewed release artifact in a release asset rather than source tree. Device test logs may be committed if small, sanitized and reproducible; otherwise upload CI artefact. Large sample resources need licensing and review. `docs/Android Medium - 3.svg` is a large vector asset in the current tree; determine whether it is necessary to source and whether it can be optimized/externally hosted or retained with explanation. Do not blindly remove a document asset without impact check.

## W.2 secret scanning

CI scan should catch PEM private-key headers, `.jks`, `.keystore`, tokens, signing material, passwords, personal data and common credential patterns. False positives need narrow allowlist with rationale. Security scan should check history too if real secret was ever committed; deleting current file does not invalidate leaked keys. When actual secret leaked, rotate/revoke before history cleanup. Logs and screenshots are also secret-bearing files. Test keys must be clearly non-production and should not be confused with trusted release key.

## W.3 license and provenance

Kernel prebuilt, firmware files, copied docs, UI icons, fonts and third-party assets require compatible licensing. SBOM/license inventory should include Rust crates, Android Gradle dependencies, build tools and embedded firmware. Reproducibility isn't only code: generated `docs/master-plan.md` references, local file URLs in ADR index and outdated docs should be link-checked. Code and documentation must state when an external driver/firmware is planned but not yet integrated.

## W.4 acceptance criteria

- Repository has no tracked local build caches, generated APKs or secret material.
- CI scans tracked files and release artifacts for secrets and unwanted output.
- Large assets have source/license/reason recorded.
- All external dependencies and firmware have provenance/license entries.
- Current documents use relative links and pass link validation where possible.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী X: practical exit criteria for each target

## X.1 QEMU x86_64 exit criteria

`qemu-x86_64` target should be considered stable development reference only when a clean build generates architecture-correct kernel/initramfs; boot reaches structured `CoreServicesReady`; required service readiness protocols answer; ext4 `/data` mount is verified; persistence marker survives repeated reboot; fault injection creates truthful failed/degraded status; logs/manifests attach exact revision and hashes; and regression runs are repeatable. If UI renderer is disabled in headless smoke, its status should be `not tested` not implicitly successful. Once these pass, the target can anchor core system development.

## X.2 QEMU AArch64 exit criteria

AArch64 must independently pass all x86_64-style gates under ARM64 target compilation. It cannot inherit `x86_64` pass status because source is common. C FFI types, architecture-specific syscall numbers, seccomp filters, atomic support, kernel image format and cross-linking are target dependent. The current compiler failure means this target has not reached packaging or runtime gates on latest main. After it passes, do a fresh run and store new artifact provenance; do not promote old evidence files automatically. Then test resource/security features that may differ on ARM64.

## X.3 Android-hosted exit criteria

Hosted runtime is validated if APK build is reproducible; correct ABI `.so` packaged; protocol negotiation works; host Activity/Service lifecycle tested; permission grants/denials propagate; battery/network telemetry is real; screen rendering authority is known and frame test works if Rust frame pipeline is claimed; camera/audio commands are callback-completed and real/fake labels correct; errors and logs do not leak private data; and S25 physical test report lists exact tested features. Emulator pass alone permits “tested on emulator,” not “fully works on S25.”

## X.4 Native phone target exit criteria

`oneplus-fajita` target is ready for controlled boot testing only after exact hardware variant and recovery are verified; board-specific kernel, config and DTB/DTBO are built; boot image format independently validated; partition/slot map and flash commands match device; target manifest and verified signatures pass; no host/generic binary fallback; a no-write dry-run validates correct device; stock restore path has been physically exercised; and responsible operator has a backup. Full feature support then proceeds subsystem by subsystem. Until these pass, `flash_allowed=false` remains.

## X.5 OS-level release criteria

A public release should publish a capability matrix and target-specific evidence rather than a single “works on phones” label. For each release the maintainer signs a decision record with tested targets, known failures, data migration/rollback details, security limitations, simulated subsystems and instructions for reporting issues. Security-sensitive updates require signed metadata and key management, not only checksums. A release with no physical-device support can still be a useful QEMU/hosted developer preview, but its title and artifact names must clearly reflect that scope.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী Y: owner dashboard ও acceptance scorecard

Project dashboard-এ একসঙ্গে ৭০টি subsystem-কে সবুজ/লাল দেখালে প্রকৃত সংকট বোঝা যায় না। Dashboard-এ gate, target এবং evidence freshness-কে প্রধান axis করতে হবে। একই subsystem এক target-এ tested এবং অন্যটিতে unvalidated হতে পারে, তাই status row-তে target column থাকা জরুরি। একটি status scorecard নিম্নরূপ ব্যবহার করা যায়:

| Gate | বর্তমান মূল্যায়ন | প্রমাণ/পরবর্তী পদক্ষেপ | status পরিবর্তনের শর্ত |
|---|---|---|---|
| x86_64 compile/test | latest workflow-এ pass | [Linux/QEMU run](https://github.com/joysriramsarkar/onuronOS/actions/runs/38024019829) | regression এলে পুনরায় green build+tests |
| x86_64 QEMU boot | latest workflow-এ pass | serial boot smoke artifact | structured readiness ও fresh revision evidence বজায় রাখতে হবে |
| ARM64 compile | current main-এ fail | `runtime/nilhal/src/lib.rs:200` pointer mismatch | ARM64 workspace build pass |
| ARM64 initramfs/boot | latest run-এ skipped | compile gate green হওয়ার পরে live test | clean package, QEMU ready, persistent marker |
| ext4 persistence | x86_64 live harness pass; ARM64 latest evidence not fresh | real disk write/reboot/read test | target-specific current run artifact |
| package manager | functional prototype | trusted key, upgrade/rollback tests | app actually launched through sandbox |
| NilLang/UI vertical slice | prototype, integration gap | installed `.nilax` → `nilrt-launch` → scene/input | real event changes displayed state |
| S25 hosted build/runtime | implementation exists; end-to-end hardware proof incomplete | NDK/Gradle CI, lifecycle and permission tests | APK+native load+device test record |
| `fajita` native phone | selected candidate, physical subsystem matrix planned | exact device profile and recovery validation | board-specific serial boot plus subsystem evidence |
| AVB/flash safety | prototype controls, production trust chain incomplete | standard AVB and no-write tests | bootloader-enforced trust and tested recovery |

## Y.1 freshness of evidence

An evidence record is fresh only if its source revision and artifact hashes match the build being described. Old evidence may remain useful as historical reference, but should not be attached to a current milestone without explicit label. If a manifest says `ci_status=configured`, this is not same as `ci_status=passed`; labels should keep that distinction. GitHub green badges should be captured using the exact SHA so a later main commit changing code does not cause a false statement about prior behavior.

## Y.2 dashboard states

Use consistent labels: `blocked`, `not-run`, `failing`, `passing`, `passing-with-limits`, `experimental`, `simulated`, `physical-validation-required`, `release-gated`. “In progress” may describe work underway but should not replace test status. For a physical camera feature, `passing` on S25 does not mean `passing` on OnePlus. One project page can show a compact high-level status; the detailed matrix lives in `docs/maturity.toml` or a generated report. Manual duplication in README should be generated/checked to avoid drift.

## Y.3 maintainers’ weekly decision

At weekly review, ask four questions in order: (1) Did the current critical gate become pass/fail with evidence? (2) Did a regression invalidate a previously passing target? (3) Which single dependency most blocks the next vertical slice? (4) Is any documentation or UI still implying a capability that remains simulated? A week with fewer commits but one critical gate genuinely green may be more valuable than a week with many screen additions. The dashboard should reward the former.

---

# অতিরিক্ত বাস্তবায়ন-সংযোজনী Z: শেষ ৪০,০০০-শব্দের রূপরেখা কীভাবে কার্যকর করবেন

এই রূপরেখা একবার পড়ে রাখা ফাইল নয়; এটিকে sprint planning, code review এবং release decision-এর source of work হিসেবে ব্যবহার করা উচিত। শুরুতে সব অধ্যায় একই সঙ্গে implement করার দরকার নেই। বরং `P0` ticket backlog থেকে একটি issue বেছে নিয়ে তার acceptance criteria পূরণ করুন, evidence attach করুন, তারপর পরের issue শুরু করুন। তাতে document-এর বিস্তৃতি engineering focus-কে দুর্বল করবে না।

## Z.1 প্রথম কাজের পরিমিত scope

প্রথম PR-এ শুধু NilHAL C character pointer mismatch ঠিক করুন। C ABI declaration review করে type portability নিশ্চিত করুন, regression tests add করুন এবং exact latest ARM64 CI run-এর result attach করুন। তারপর build artifact stale state থেকে এসেছে কি না নিশ্চিত করতে output clean/manifest binding review করুন। এই PR-এর উদ্দেশ্য ARM64 kernel boot claim নয়; উদ্দেশ্য হলো ARM64 cross-compile gate খোলা।

দ্বিতীয় PR-এ ARM64 job package, disk, boot এবং persistence steps পর্যন্ত পৌঁছাবে। যদি QEMU kernel/bootargs বা initramfs-এ নতুন error আসে, সেটিকে দ্বিতীয় issue হিসেবে আলাদা করে fix করুন। Third PR-এ `nilinit` boot/readiness/storage status truthful করুন। এই তিনটি issue সম্পন্ন না হওয়া পর্যন্ত user-facing new app screen যোগ করা project progression-এর প্রধান মাপকাঠি হওয়া উচিত নয়।

## Z.2 second milestone-এর scope freeze

ARM64 QEMU green হলে পরবর্তী target হলো package-to-runtime-to-screen vertical slice। এই milestone-এ Alap-এর সব widget, App Store, telephony, camera, music streaming, native phone boot কিংবা OTA rollout চাইবেন না। একটিমাত্র `.nil` app compile, sign, install, sandbox launch, render, click এবং durable state update করে—এটিই যথেষ্ট। ছোট vertical slice architecture contracts পরীক্ষা করে; failing boundary দ্রুত খুঁজে পাওয়া যায়।

## Z.3 third milestone-এর scope freeze

একটি end-to-end NilLang app QEMU-তে চললে hosted S25 track-এ একই public app model integrate করুন। প্রথমে NDK/Gradle reproducibility ও JNI version; তারপর lifecycle and permission; তারপর battery/network; এরপর display; তারপর camera/audio. Calls/SMS, wide camera feature support, Bluetooth profile matrix এবং background service policy পরে। Each addition needs backend confirmation and negative test.

## Z.4 native phone milestone

OnePlus 6T target-এ সরাসরি “full OS image” বানানোর লক্ষ্য নয়; “serial boot and recovery-safe first userspace” প্রথম milestone। Exact kernel/DTS/DTBO, boot image, rootfs, partition mapping, stock backup and restore path verified না হওয়া পর্যন্ত flasher disabled। প্রথম physical boot সফল হলে storage/read-only diagnostics; তারপর display/touch; এরপর power/network; শেষে audio/modem/camera। একটি subsystem fail করলে অন্যটি tested বলে mark করা যাবে, কিন্তু overall phone support claim lower maturity-তেই থাকবে।

## Z.5 ৪০,০০০-শব্দের দলিলের কার্যকর ফল

এই দীর্ঘ রূপরেখা থেকে প্রকৃত ফল হবে তিনটি জিনিস: (১) পরিষ্কার priority/dependency graph, (২) tests ও evidence-এর মাধ্যমে যাচাইযোগ্য vertical slices, (৩) feature status সম্পর্কে honest communication. Source code-এ নতুন capabilities যোগ হওয়া অবশ্যই দরকার, কিন্তু capability-র সত্যতা প্রমাণ করা, failure mode সংজ্ঞায়িত করা, backup/recovery বজায় রাখা এবং hardware claims সীমার মধ্যে রাখা আরও গুরুত্বপূর্ণ। অনুরণ ওএসের জন্য প্রথম লক্ষ্য “সব ফোনে সব কাজ” নয়; প্রথম লক্ষ্য হলো একটি target-এ reliable boot, durable storage, secure app launch এবং real input/display flow-এর পুনরুৎপাদনযোগ্য প্রমাণ। এরপর সেই ভিত্তির উপর বাকি ecosystem গড়া উচিত।

# অতিরিক্ত বাস্তবায়ন-সংযোজনী AA: দ্রুত troubleshooting matrix

Build বা integration ব্যর্থ হলে একই সময় অনেক subsystem বদলানোর বদলে সর্বপ্রথম failure boundary isolate করুন। নীচের matrix root cause diagnosis-এর starting point; actual logs/code দেখে সিদ্ধান্ত নিতে হবে।

| লক্ষণ | সম্ভাব্য boundary | প্রথম যাচাই | যা করা উচিত নয় |
|---|---|---|---|
| ARM64 build-এ `E0308` pointer mismatch | Rust/C ABI portability | ABI struct field type, `c_char`, target-specific type, line-specific compiler context | compiler চুপ করাতে arbitrary pointer cast করে test বাদ দেওয়া |
| Initramfs তৈরি হয়েছে কিন্তু QEMU বলে `/init` চালানো যায় না | rootfs ELF/loader/path | machine type, executable mode, shebang/interpreter, copied target triple | host binary copy করে boot log success string যোগ করা |
| kernel checksum error | source integrity | pinned SHA, source URL, downloaded file size/hash, target architecture | checksum verification skip বা pinned value অন্ধভাবে বদলে দেওয়া |
| QEMU boot হয় কিন্তু `/data` volatile | disk formatting/mount | disk image actual ext4 কিনা, attached path, device node, mount error | `/data` directory আছে দেখে persistence pass বলা |
| `nilinit` success log কিন্তু service কাজ করে না | service readiness | process exit status, socket bind, protocol handshake, dependency init | `sleep` বাড়িয়ে indefinitely race mask করা |
| SELinux status UI-তে enforcing, log-এ policy file missing | state source / fail policy | kernel active enforcing state, policy load return and readback | UI text hard-code করে রেখে দেওয়া |
| Android screen দেখা যায় কিন্তু Rust frame update নেই | rendering authority / JNI | Java Canvas path, actual Surface object, JNI latest-frame data, frame IDs | Java mock UI-কে Rust compositor presentation বলা |
| Camera capture output আসে কিন্তু same test image | simulated backend fallback | frame `source`, request correlation, Camera2 ImageReader callback | sample JPEG-কে silent fallback হিসেবে real photo save করা |
| Wi-Fi UI connected দেখায়, actual network নেই | fake cache / backend event mapping | source field, connectivity callback, route/DNS validation | cached boolean mutate করে success দেখানো |
| Dialer opens but call status active | telephony state semantics | Android modem call-state callback, user action result | `ACTION_DIAL` launch-কে connected call হিসেবে গণ্য করা |
| APK builds locally but no native library on clean CI | NDK/Gradle integration | `.so` staging, ABI folder, clean build, native symbols | local staged `.so` repository-তে commit করে build issue আড়াল করা |
| Flasher validates product but wrong slot/partition | target profile | exact fastboot vars, slot, partition map, image header and signature | generic `fastboot flash boot/system` assumption ব্যবহার করা |
| `.nilax` install passes but app launches outside sandbox | integration test gap | actual `nilrt-launch`, namespace/UID, seccomp status | `NilVM::load_package()` unit test-কে sandbox launch test বলা |
| UI says note saved but state vanishes after reboot | durability contract | filesystem mounted, fsync, transaction commit, persistence marker | “file exists immediately” দেখে saved status দেখানো |
| CI evidence JSON says pass despite compile job failed earlier | stale/manual evidence | source revision, run ID, generated artifact timestamp, job status | manually `all_healthy=true` update করে result বাঁচানো |

## AA.1 root-cause workflow

প্রথমে failing test-এ exact command, source SHA, output path ও last-known-good revision capture করুন। দ্বিতীয়ত failure-কে compile, package, kernel boot, initramfs PID1, service readiness, storage, renderer, host permission বা hardware layer-এর একটি boundary-তে সীমাবদ্ধ করুন। তৃতীয়ত smallest reproducer তৈরি করুন। চতুর্থত fix-এর সঙ্গে regression test দিন, তারপর relevant wider matrix চালান। পঞ্চমত evidence files build result থেকে derive করুন এবং `docs/maturity.toml` update করুন। এই workflow-এর লক্ষ্য diagnostic loop দ্রুত করা, random edits নিষিদ্ধ করা নয়।

## AA.2 final target decision

যে কোনও সময় maintainers-এর কাছে যদি এই প্রশ্ন আসে—“এই সপ্তাহে আরও feature যোগ করব, নাকি build/boot issue ঠিক করব?”—তাহলে current P0 blocker থাকলে উত্তর হবে build/boot issue। User-facing app prototype প্রয়োজনীয়, কিন্তু তা তখনই long-term investment যখন package trust, runtime isolation, display event flow এবং backend truth contract আছে। এই project-এর সবচেয়ে ভাল next milestone হল এমন একটি ছোট, নির্ভরযোগ্য, পুনরুৎপাদনযোগ্য OnuronOS build, যেখানে boot status, storage, services এবং capability state সবই সত্যিকারের runtime evidence দিয়ে নির্ধারিত। সেটিই পরে phone port ও NilLang ecosystem-এর ভিত্তি হবে।

# অতিরিক্ত বাস্তবায়ন-সংযোজনী AB: milestone review meeting-এর নির্দিষ্ট agenda

একটি milestone review meeting যেন সাধারণ status conversation না হয়; প্রতিটি সভার শেষে সিদ্ধান্ত, gate result এবং পরবর্তী কাজ লিখিত থাকবে। Meeting-এর আগে contributor evidence links, test run, known limitation এবং unresolved issue শেয়ার করবেন। সভায় code review নতুন করে করা দরকার নেই; লক্ষ্য হবে project-level dependency ও readiness যাচাই।

## AB.1 ARM64 gate review

প্রথমে current main SHA এবং latest workflow run মিলিয়ে দেখুন। এরপর compile stage, artifact generation, real ext4 formatting, QEMU boot, readiness, persistence—প্রতিটি আলাদা status হিসেবে পেশ করুন। যদি compile pass কিন্তু boot fail হয়, সভার outcome হতে পারে “compile gate passed, boot gate failed”; পুরো ARM64 target pass বলা যাবে না। Evidence manifest-এর source revision current SHA-র সঙ্গে না মিললে evidence pending থাকবে। পরবর্তী task হবে failure-এর smallest reproducer, সংশ্লিষ্ট issue owner এবং next run-এর acceptance condition।

## AB.2 Runtime vertical-slice review

Review করুন source `.nil` থেকে actual package তৈরি হয়েছে কি না; signature যাচাই হয়েছে কি না; installed payload-এর hash মিলেছে কি না; runtime launcher বাস্তব app process তৈরি করেছে কি না; process UID/namespace/device visibility ঠিক কি না; NilUI frame actual render/present হয়েছে কি না; event handler visible state update করেছে কি না; app-private persistent state restart-এর পরে আছে কি না। কোনো ধাপ mock হলে তা স্পষ্টভাবে test mode হিসেবে record হবে। “The screen shows hello” একা acceptance নয়—সেই screen কীভাবে এসেছে, package ও sandbox lifecycle ব্যবহৃত হয়েছে কি না, তা review করুন।

## AB.3 S25 hosted review

NDK/Gradle artefact-এর ABI ও source revision মিলিয়ে দেখুন। Native library loaded কিনা, JNI protocol version negotiated হয়েছে কিনা এবং host capability operations live callback দিয়ে complete হয় কিনা যাচাই করুন। Permission denied, Activity pause/resume, Surface destroy/recreate, native library absent, host service restart এবং queue overflow-এর ফল review করুন। S25 physical device না থাকলে emulator test pass হতে পারে, কিন্তু meeting record-এ “physical not run” থাকবে এবং hardware-validation task খোলা থাকবে।

## AB.4 native phone review

এই review-এ device identity, bootloader state, exact image bundle, checksum/signature, partition map, current slot, recovery procedure, backup hash এবং no-write dry-run দেখা হবে। যে reviewer flash command authorize করছেন, তাকে ব্যাখ্যা করতে হবে failure হলে device ফেরত আনার path কী। Hardware matrix-এ কোন subsystem সত্যিই পরীক্ষা হয়েছে আর কোনটি planned—সেটা আলাদা করে পড়তে হবে। Recovery untested হলে physical write gate pass নয়।

## AB.5 decision record

Meeting শেষে `GO`, `GO WITH LIMITS` অথবা `NO-GO` নির্বাচন করুন। প্রতিটি decision-এর সঙ্গে source revision, workflow run IDs, evidence artifacts, accepted limitations, open stop-ship items, next milestone owner ও review date থাকবে। Feature marketing text, README maturity table এবং release notes এই decision record-এর সঙ্গে consistent হতে হবে। এই রীতি project-এর open-source nature-কে সীমাবদ্ধ করে না; বরং external contributors ও future maintainers-কে বর্তমান অবস্থা বুঝতে সাহায্য করে।

## AB.6 no-go মানে কী

`NO-GO` সিদ্ধান্তকে developer বা project-এর ব্যর্থতা হিসেবে দেখার দরকার নেই। এটি বলে যে নির্দিষ্ট claim-এর evidence এখনও পর্যাপ্ত নয়, অথবা safety condition পূরণ হয়নি। যেমন ARM64 compile ঠিক হলেও persistence test না চললে native-phone flash gate এখনও no-go; S25 emulator-এ camera permission flow ঠিক হলেও real S25 Camera2 capture not run থাকলে physical-camera claim no-go; signed custom metadata থাকলেও bootloader public-key enforcement না থাকলে verified-boot production claim no-go। একই project-এর অন্য target/scope-এ `GO WITH LIMITS` চলতে পারে—যেমন x86_64 QEMU development preview। এই precision-ই roadmap-কে বাস্তব engineering process-এ রূপান্তর করে।

## AB.7 পরবর্তী সভা পর্যন্ত action items

Review-এর শেষে তিনটির বেশি critical action item একসঙ্গে assign না করাই ভালো, যদি না team size ও bandwidth তা সমর্থন করে। প্রতিটি action item-এর owner, acceptance test, due review point এবং blocker explicit থাকবে। পরবর্তী সভার শুরুতে আগের action item-এর evidence দেখা হবে; শুধু “কাজ করেছি” status যথেষ্ট নয়। যদি কোন কাজের জন্য physical device দরকার হয় কিন্তু device পাওয়া না যায়, সেটিকে hardware-blocked হিসেবে চিহ্নিত করে parallel QEMU/hosted work করা যাবে। কিন্তু device-unavailable-কে test-passed হিসেবে গণ্য করা যাবে না।

এই acceptance policy-র সারকথা হলো: একটি capability তখনই উন্নীত হবে, যখন সংশ্লিষ্ট target-এ কাজটি বাস্তবভাবে করা হয়েছে, failure path পরীক্ষা করা হয়েছে, ফলাফল পুনরায় যাচাই করা সম্ভব, এবং user-facing status প্রমাণের সঙ্গে মেলে। Static screen, mocked callback বা architecture note প্রাথমিক ধাপকে দ্রুত করতে পারে; কিন্তু এগুলো live kernel, persistent disk, physical sensor কিংবা bootloader trust-এর প্রমাণ নয়। অনুরণ ওএসের দীর্ঘমেয়াদি credibility এই পার্থক্য বজায় রাখার উপর নির্ভর করে।

## Final maintainer pledge

প্রতিটি নতুন feature-এর সঙ্গে তিনটি জিনিস প্রকাশ করতে হবে: এটি কোন target-এ চলছে, কোন evidence দিয়ে তা যাচাই হয়েছে, এবং কোন সীমাবদ্ধতা এখনো বাকি। এই ছোট শৃঙ্খলা ভবিষ্যতের contributors-কে একই system model বুঝতে সাহায্য করবে, fake backend-এর উপর inadvertent dependency তৈরি হওয়া ঠেকাবে, এবং physical hardware-এর জন্য unsafe assumptions কমাবে। এই দলিলের সব milestone একসঙ্গে implement করার বাধ্যবাধকতা নেই; critical path-এর প্রথম item থেকেই শুরু করা যায়। তবে milestone-এর acceptance gate পূরণ না করে পরবর্তী status ঘোষণা করা উচিত নয়। এভাবেই prototype থেকে reliable system-এ যাওয়ার পথটি পরিষ্কার এবং মাপযোগ্য থাকবে।

পরবর্তী অডিটে একই GitHub `main` আবার পরীক্ষা করে নতুন commit, CI conclusion ও evidence freshness তুলনা করতে হবে। যদি compile error বদলায়, রূপরেখার priority list বাস্তব failure log অনুযায়ী পুনর্বিন্যাস করতে হবে; পুরোনো diagnosis-কে স্থায়ী সত্য ধরে নেওয়া যাবে না।
