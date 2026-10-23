# OnuronOS UI — measured design specification

Source of truth: `অনুরণ ওএস — অ্যাপ UI ডিজাইন-png/` (35 screens × dark/light),
780×1592 px each. The phone canvas used by `nilsim` is **390×796**, exactly half,
so every metric below is *design px ÷ 2* and can be read straight against the art.

This file records what was measured from the reference PNGs so the numbers do not
have to be re-derived by eye.

## Palette (sampled, exact)

| Token | Dark | Light |
|---|---|---|
| background | `#1E1236` | `#F5F0FF` |
| card / surface | `#322748` | `#FDFCFF` |
| leading icon tile | `#4C3876` | `#E5D8FF` |
| hairline border | `#534A65` | `#E1D9F0` |
| primary text | `#FFFFFF` | `#1E1236` |
| secondary text | `#CBBDE6` | `#5E4B86` |
| tertiary text | `#9588AE` | `#7F6F9F` |
| icon stroke | `#D9C6FF` | `#5B2FC0` |
| accent fill (both) | `#9B6BFF` | `#9B6BFF` |
| on-accent text | `#14082B` | `#14082B` |
| slider track | `#423856` | `#E6E0F3` |
| battery fill | `#2FBF71` | `#2FBF71` |
| priority (উচ্চ/মাঝারি) | `#DA9E65` | `#A5540F` |

Android launcher tiles: YouTube `#D92D2D`, Telegram `#0B6EA8`,
Wikipedia `#1B2A41`, Maps `#15803D`, Instagram `#C2377E`,
Call of Duty `#A5540F`, Spotify `#15803D`, 2048 `#B45309`.

## Global metrics (390×796 canvas)

- Card margin **16**, card width **358**, card radius **16**, border **1px**.
- Leading tile **40×40**, radius **12**, tile sits **19** in from the card edge.
- Row pitch **72** (settings list measured 72.5), row text starts **85**.
- Icon-tile glyph ≈ **22** wide, icon stroke ≈ **1.8**.
- Screen title **30 bold** at `x=24, y=60`; with back button `x=72`, circle
  **35** at `(20, 59)`.
- Primary button: `x=24`, `w=342`, `h=56`, radius **28**, label 16 bold,
  on-accent colour; bottom button at `y=708`.
- Switch **44×24**, thumb 20, on = accent.
- Slider card: `h=70`, track at `y+46`, `6` tall, inset 18.
- Chip: `h=34`, radius 17, horizontal padding 16, label 13.5.
- Status bar: clock `x=22, y=8`, 15 bold; right cluster (pin, BT, Wi-Fi,
  signal, battery) ends at `x≈366`, centred at `y=19`.
- Page dots: active pill **32×10**, inactive dot 10.

## Per screen

| Screen | Key geometry |
|---|---|
| Home | brand 12 @ (24,41); clock 58 centred @ y68; date 15 bold @ y137; status pill `17,173 356×39`; app grid 59×59 tiles at x `26/118/211/304`, rows y `235/339/443`, label at tile+66; section label y556; Android tiles y `584/676` |
| Lock | clock 58 @ y104; date @ y172; notification card `21,229 348×74`; "পিন দিন" y340; 4 PIN dots y366 (22 pitch); keypad top 409 |
| Notification panel | date 16 @ (24,32); 8 tiles 61×61 at x `24+85c`, y `66+92r`, label +68; brightness card `258 h78` (bar y306); "নোটিফিকেশন" y352; notification cards from y378, h74, gap 8; "সব মুছুন" y708 |
| Recents | cards 268×470 at y90, x60 step 286; "সব বন্ধ করুন" y708 |
| Power menu | pills `25,y 340×58` radius 29 at y `279 + 66i` |
| OOBE 1 | logo `130,171 129×130` r34; "স্বাগতম" 29 @ y336; sub @ y388; dots y429; button y708 |
| OOBE 2 | title 27 @ (24,60); card `130 h144`; 2 rows of 72; dots y290; button y708 |
| OOBE 3 | title @ 60; hint @ 118; 4 dots y160; keypad top 200; page dots y470; button y708 |
| Settings | card from y130; 9 rows pitch 72.5 starting 146 |
| Wi-Fi / Bluetooth | master switch card y104; list card y170 (72/row); Bluetooth action button y708 |
| Display | 3 theme cards 111×130 at y104; brightness y250; text size y330; info card y410 h116 |
| Sound | 3 slider cards y `104/190/276`; haptics switch y366; ringtone row y430 |
| Security | list card y104 (3 rows); switches y336, y400 |
| Battery | hero card y104 h170 (52 bold %); 3 usage rows from 290; saver switch 536 |
| Storage | usage card y104 h76; list card y196 (3 rows) |
| Backup | info card y104 h136; auto switch y252; button y708 |
| Language | list card y104 (3 rows); switches y336, y400 |
| Accessibility | text slider y104; switches y190/254/318 |
| VPN | switch y104; profile row y170; button y708 |
| Update | logo `155,118 80×80`; info card y218 h136; status y380; button y708 |
| About | logo y120; name y218; card y258 h300 (5 rows, 58 pitch) |
| SoftBus | device card y104 h76; peer list y196; clipboard switch; FAB |
| nilpkg | search y104; chips y160; list y206; publisher row after list |
| Android | capability switch y104; app list y170; FAB "APK ইনস্টল" |
| Phone | chips y104; list card y152 h200; number 21 bold y370; keypad top 404; green call disc 60 at y700 |
| Messages | search y104; list y160 h360 (5 rows); FAB "নতুন বার্তা" |
| Files | path y112; mount chips y140; list y196 |
| Notes | search y104; chips y158; note cards from y208, h74, gap 10; FAB |
| Calculator | expression 15 right y88; result 44 right y118; function pills y210; keypad 80×58 from y292 |
| Music | cover `24,100 342×250`; title y372; path y400; progress y432; transport y490; list y560 |
| Browser | search y104; 4 shortcuts y160; "ইতিহাস" y250; history card y276; toolbar y712 h56 |
| Camera | viewfinder `16,40 358×548`; zoom y566; ছবি/ভিডিও y610; shutter 66 at y690 |
| Terminal (out of scope) | unchanged legacy renderer |
| Memories (out of scope) | unchanged legacy renderer |

## Out of scope by maintainer instruction

`টার্মিনাল` (Terminal) and `স্মৃতি` (Memories) keep their existing renderers,
status bar and navigation bar; the 1.7 redesign does not touch them.
