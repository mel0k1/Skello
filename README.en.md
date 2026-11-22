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
- crates/skello-chipsets: the MediaTek seccfg partition format (V3/V4) after mtkclient: lock state reading, a local unlock patch for partitions with the software SEJ key (AES-256-CBC), hardware keys are honestly declined until the DA stage
- app: the Flutter-to-Rust bridge over flutter_rust_bridge (the app/rust crate builds with cargokit alongside the app); DeviceRepository calls the core and switches to system adb and fastboot when FFI is unavailable
- app: a live device list straight over USB (adb and fastboot) with state cards, a reboot menu (system / bootloader / recovery) and the unlock wizard entry; a Material 3 Expressive theme, English and Russian localization
- app: the bootloader unlock wizard: a wipe warning with mandatory confirmation, flashing unlock over USB (or through the binary) waiting for the on-phone confirmation and a repeated check
- app: the Root screen: choosing Magisk / KernelSU / APatch, picking boot.img, header verification through the Rust core, flashing boot / recovery / vendor_boot over USB with live progress and partition erase after the unlocked, battery-soc-ok and max-download-size checks; after flashing - a reboot waiting for the device in adb for up to 90 seconds
- app: the Root screen flashes images larger than max-download-size: Android sparse is split along the chunks into a series of download+flash, a raw image without sparse layout is honestly refused
- app: an adb shell terminal over USB on the device card and a MediaTek seccfg screen: lock state reading and unlock through root (dd + base64 over adb shell)
- app: stage 1 vendor transports (skello-chipsets): MediaTek BROM/DA V5 with root-free seccfg unlock, Unisoc FDL with transcode escaping and partition writes, Qualcomm Sahara/Firehose with the rawprogram XML; chipset USB ports are visible at the API level
- app: a Chipsets screen for the stage 1 paths: BROM / FDL / EDL ports with serials, MediaTek unlock through the stock Download Agent (no root), Unisoc partition flash through FDL1/FDL2 with hex addresses, Qualcomm EDL with a Firehose programmer and rawprogram XML parsing; every operation needs a confirmation and lands in the journal
- app: stage 2 chipsets: SEJ hardware through DA V5 - the HACC engine driven over the DA custom registers, a hardware-keyed V4 seccfg unlocks without root; DA V6 (XML) - a session with BOOT-TO stage 2 and a fastboot or meta exit; Qualcomm EDL over serial (115200, Sahara with the silent-HELLO workaround + Firehose); Unisoc - the partition table over FDL2; all four live on the Chipsets screen
- app: exploits and unlocks: Kamakiri2 (linecode) - BootROM dump and DA-auth bypass for vulnerable chips (MT6755..MT6785, mtkclient payload blobs embedded), Carbonara - DA1 hash re-mark and patched stage 2 boot; the RPMB block reads and writes through the DA (parttype 3); Unisoc T618 - bootloader unlock with a signed identifier token (RSA-4096, community key); the fastboot transport accepts the non-standard Unisoc SN answers
- app: whole-firmware flashing: Unisoc - the .pac parser (header and directory per unpac, CRC16 0xA001 over header and data, the FDL pair boots from the package itself, every flash file goes to its named partition, then a reset); MediaTek - FORMAT through DA V5 (erase a partition by its GPT name, RPMB refused by design), the scatter file (platform, addresses, boot areas) with the images flashed from the scatter directory
- app: partition ops through DA V6 and EDL: MediaTek - the XML DA reads and writes partitions in 1 MiB chunks; after a disconnect the session rebuilds and the operation continues at the exact offset (tested against a mock with a dead link), partition erase by its GPT name; Qualcomm - the Firehose `<erase>` over a partition's LBA range from the GPT, patch0.xml application (DISK entries only, as qdlrs does), the GPT partition list
- app: a command and device response journal in skello-commands.log (the app support directory) - written by both the Dart layer and the Rust core inside transactions
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

## Roadmap

Priority from the alpha.15 code audit and what is still missing for a full SP Flash Tool + QPST grade combine:

Done in alpha.16: cancel for long operations (a shared flag and a stop button), seccfg/boot/vbmeta backups through DA and firehose, image checks against the actual GPT before writing.

1. A/B slots: getvar current-slot, set_active, flashing the inactive slot and switching after
2. Dynamic partitions: super, resizing through LP metadata
3. Qualcomm: the full rawprogram + patch0.xml pipeline (today separate operations), reads from LUNs other than zero
4. OTA sideload through the adb transport
5. New vendors: Samsung (download mode / Loke), Allwinner
6. Log export to a file and in-app driver hints: detect missing udev rules / WinUSB and point at the install
7. Auto-update and localization beyond ru/en

## Safety

Unlocking the bootloader wipes the whole device storage. Flashing a wrong image or a mid-way failure can leave the phone dead. Skello shows what is about to happen before every dangerous step, but the person pressing the button owns the operation. Check that the image is built for your exact model.

## Ports

The core opens the bootloader ports directly over USB (bulk endpoints), so without preparation only root has access.

Linux - udev rules ship in the release archive (`udev/40-skello.rules`):

    sudo cp 40-skello.rules /etc/udev/rules.d/
    sudo udevadm control --reload && sudo udevadm trigger

The rules give access to MediaTek BROM/preloader/DA (0e8d), EDL 9008 (05c6), Unisoc BROM (1782) and fastboot/adb (18d1), and bind qcserial to 9008 for the serial tools. Unplug and plug the phone back in after installing.

Windows - the WinUSB driver package (`windows/skello_winusb.inf`, `windows/install_winusb.cmd`): run the script as administrator or `pnputil /add-driver skello_winusb.inf /install`. The transport goes through WinUSB, so without the package Windows keeps Skello away from the ports. Once WinUSB binds, the old COM-port drivers (usbser for the MediaTek CDC ports, QDLoader for 9008) stop claiming these devices - Skello never uses COM ports; serial tools do, roll back through Device Manager for them.

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
