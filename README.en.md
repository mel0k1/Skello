<img src="assets/logo.png" width="96" align="right" alt="">

# Skello

[![CI](https://github.com/mel0k1/Skello/actions/workflows/ci.yml/badge.svg)](https://github.com/mel0k1/Skello/actions/workflows/ci.yml)
[![Release](https://github.com/mel0k1/Skello/actions/workflows/release.yml/badge.svg)](https://github.com/mel0k1/Skello/actions/workflows/release.yml)

Skello is a GUI tool for flashing Android devices and getting root access. The Rust core talks to the phone directly over adb, fastboot, BROM/DA (MediaTek), FDL (Unisoc) and Sahara/Firehose (Qualcomm) protocols, and the Flutter interface is built on Material 3 Expressive.

Документация на русском: [README.md](README.md).

The project is in an early stage: the protocol crates already work with real message formats, while the interface and flashing wizards are in development.

## Why

adb and fastboot live in a terminal: you have to remember commands, keep the terminal open, and double-check yourself at every step. A wrong partition name or someone else's image easily turns a phone into a brick. Skello walks you through it: it shows the device state, explains what is about to happen before a dangerous operation, and checks the conditions (charge level, unlocked bootloader, image size) before sending a command.

## What is here

- crates/skello-adb: the adb protocol fully in Rust: the 24-byte amessage header, a client for the adb server on port 5037 and a server-less USB transport (the ff/42/01 interface): the CNXN/AUTH handshake with an RSA key in the android pubkey format, shell and reboot services through A_OPEN, device / unauthorized / offline states
- crates/skello-fastboot: fastboot command strings (getvar, download, flash, erase, set_active, reboot-*), response parsing for OKAY / FAIL / INFO / TEXT / DATA, a max-download-size parser and a USB transport on nusb (ff/42/03 device enumeration, getvar, reboot, unlock, erase, download with progress)
- crates/skello-bootimg: Android boot image header parsing (v0-v4): kernel and ramdisk sizes, page size, Android version and security patch, AVB footer presence
- app: the Flutter-to-Rust bridge over flutter_rust_bridge (the app/rust crate builds with cargokit alongside the app); DeviceRepository calls the core and switches to system adb and fastboot when FFI is unavailable
- app: a live device list straight over USB (adb and fastboot) with state cards, a reboot menu (system / bootloader / recovery) and the unlock wizard entry; a Material 3 Expressive theme, English and Russian localization
- app: the bootloader unlock wizard: a wipe warning with mandatory confirmation, flashing unlock over USB (or through the binary) waiting for the on-phone confirmation and a repeated check
- app: the Root screen: choosing Magisk / KernelSU / APatch, picking boot.img, header verification through the Rust core, flashing boot / recovery / vendor_boot over USB with live progress and partition erase after the unlocked, battery-soc-ok and max-download-size checks; after flashing - a reboot waiting for the device in adb for up to 90 seconds
- app: a command and device response journal in skello-commands.log (the app support directory)
- CI on GitHub Actions: fmt, clippy, Rust tests on Linux and Windows, Flutter analyze and tests; release builds for Windows and Linux on tags

Protocol notes and the AOSP source breakdown live in [ARCHITECTURE.en.md](ARCHITECTURE.en.md), chipset work in [CHIPSETS.en.md](CHIPSETS.en.md).

## MVP plan

1. Device list from the adb server (host:devices) on the main screen
2. Device card: model, Android version, charge, bootloader state over getvar
3. Reboot to bootloader, recovery and fastboot from the device menu
4. Bootloader unlock wizard with a full-wipe warning; MediaTek and Unisoc have their own paths from CHIPSETS.en.md
5. Image flashing over fastboot; Qualcomm over EDL (Sahara + Firehose)
6. Root of your choice: Magisk, KernelSU or APatch - patch boot.img on the computer side and flash the patched image

The order can change along the way.

## Safety

Unlocking the bootloader wipes the whole device storage. Flashing a wrong image or a mid-way failure can leave the phone dead. Skello shows what is about to happen before every dangerous step, but the person pressing the button owns the operation. Check that the image is built for your exact model.

## Development

You need Rust stable and Flutter stable (verified on Flutter 3.50).

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

cd app
flutter pub get
flutter analyze
flutter test
```

Code and commit rules are in [CONTRIBUTING.en.md](CONTRIBUTING.en.md).

## License

Dual licensed: [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), your choice.
