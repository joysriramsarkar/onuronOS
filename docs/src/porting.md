# 📱 NilOS Android Phone Installation & Porting Guide

This guide explains how to run and install NilOS on Android hardware.

---

## 🎯 ২ টি প্রধান পদ্ধতিতে চালানো যায়:

### পদ্ধতি ১: ফোনে কোনো রিস্ক ছাড়া টেস্ট করা (Container / Termux-Wayland)
যদি ফোন ফ্ল্যাশ বা রুট না করে সরাসরি NilOS-এর UI, অ্যাপ ও SoftBus পরীক্ষা করতে চান:
1. ফোনে **Termux** এবং **Termux:X11 / Wayland** ইনস্টল করুন।
2. NilOS-এর ARM64 বাইনারিগুলো পুশ করুন:
   ```bash
   adb push out/aarch64-generic/rootfs /data/local/tmp/nilos
   ```
3. Termux-এ Wayland শেল চালু করে NilOS অ্যাপস রান করুন:
   ```bash
   export WAYLAND_DISPLAY=wayland-1
   ./usr/bin/hello
   ```

---

## পদ্ধতি ২: ফোনে সম্পূর্ণ ওএস ফ্ল্যাশ করা (Bare-Metal Fastboot Flash)

### ধাপ ১: ARM64 বিল্ড তৈরি করা
আপনার পিসিতে NilOS-এর ARM64 ইমেজ বিল্ড করুন:
```bash
./build/build.sh aarch64-generic
```

### ধাপ ২: ফোনের বুটলোডার আনলক করা
1. ফোনে **Developer Options** চালু করে **OEM Unlocking** এবং **USB Debugging** অন করুন।
2. ফোনটিকে Fastboot মোডে রিবুট করুন:
   ```bash
   adb reboot bootloader
   ```
3. বুটলোডার আনলক করুন:
   ```bash
   fastboot flashing unlock
   # অথবা পুরানো ফোনে: fastboot oem unlock
   ```

### ধাপ ৩: NilOS সিস্টেম ফ্ল্যাশ করা
ফোনে স্বয়ংক্রিয়ভাবে ফ্ল্যাশ করতে আমাদের ফ্লাশার টুলটি চালান:
```bash
./build/flash-device.sh aarch64-generic
```

অথবা ম্যানুয়ালি:
```bash
# ১. কার্নেল ফ্ল্যাশ
fastboot flash boot out/aarch64-generic/boot.img

# ২. সিস্টেম ইমেজ ফ্ল্যাশ (Treble GSI স্টাইল)
fastboot flash system out/aarch64-generic/system_a.img

# ৩. ভেরিফায়েড বুট ডিজেবল ও ইউজারডাটা ফরম্যাট
fastboot flash vbmeta --disable-verity --disable-verification out/aarch64-generic/vbmeta_a.img
fastboot erase userdata

# ৪. রিবুট
fastboot reboot
```

---

## 🔌 হার্ডওয়্যার ড্রাইভার ও ভেন্ডর ব্লব (Vendor Blobs)
NilOS-এর **HAL আর্কিটেকচার** Android Treble দর্শন অনুসরণ করে:
- ফোনের অরিজিনাল `/vendor` পার্টিশন অপরিবর্তিত থাকে।
- NilOS-এর [runtime/nilhal](file:///c:/Users/joysr/Documents/OS/nilos/runtime/nilhal) লোডার **libhybris** ব্যবহার করে ভেন্ডর ড্রাইভারের (Adreno GPU, Camera HAL, Audio, Modem) সাথে কমিউনিকেট করে।
