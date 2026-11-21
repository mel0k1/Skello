# Skello architecture

This file collects what came out of the AOSP and reference-project sources, and the plan for how Skello turns it into one tool. Chipset work for MediaTek, Unisoc and Qualcomm is covered separately in [CHIPSETS.en.md](CHIPSETS.en.md).

## Layers

- crates/skello-adb, crates/skello-fastboot and crates/skello-bootimg: protocol crates, no GUI coupling
- app: the Flutter app, Material 3 Expressive (light and dark themes from one seed color), i18n through ARB files in lib/l10n
- Layer bridge: flutter_rust_bridge - the crates build into a library living in the same process as Flutter, types generated into Dart. On top of that, the first stage runs a fallback: system adb and fastboot invoked through std::process (cmd/PowerShell on Windows, bash on Linux), so the wizards work before the own USB transport exists
- Own USB transport: the nusb crate (libusb on Linux, WinUSB on Windows), with the transports from this file and CHIPSETS.en.md implemented on top

## How adb works (from packages/modules/adb sources)

- Client-server: adb clients never reach the device directly, they connect to the adb server on 127.0.0.1:5037. The "smart socket" format: message length in 4 hex digits, then the body; the server answers with an OKAY or FAIL line, then length and body again (smart_socket_enqueue in sockets.cpp)
- Server-device (adbd): a binary protocol. Every packet: a 24-byte amessage header (command, arg0, arg1, data_length, data_check, magic = command ^ 0xffffffff) and a payload up to 1 MB (MAX_PAYLOAD in adb.h). Our crate reproduces this header byte for byte
- Commands: A_CNXN (handshake, protocol version and max payload), A_OPEN / A_OKAY / A_WRTE / A_CLSE (streams), A_AUTH (authorization), A_STLS (TLS upgrade for wireless debugging)
- AUTH: the device sends a 20-byte token (TOKEN_SIZE), the host signs it with a private RSA key; the user confirms the key fingerprint on the phone screen
- Protocol version 0x01000001 and newer allows skipping the payload checksum; Skello still computes it, to stay compatible with older adbd
- Files (push/pull) travel in a separate SYNC protocol inside the socket: ID_SEND / ID_RECV / ID_STAT / ID_LIST / ID_DATA / ID_DONE, data blocks up to 64 KB (SYNC_DATA_MAX), the second protocol version added compression (Brotli, LZ4, Zstd) and full file stats (file_sync_protocol.h)
- USB: an interface with class 0xFF, subclass 0x42, protocol 0x01 is adb (ADB_CLASS / ADB_SUBCLASS / ADB_PROTOCOL in adb.h)
- Transport states: device, bootloader, recovery, sideload, rescue (ConnectionState in adb.h) - the interface reads the phone mode from them

## How fastboot works (from system/core/fastboot sources)

- USB: same class 0xFF and subclass 0x42, but protocol 0x03 - this triple tells the bootloader apart from adbd (match_fastboot_with_serial in fastboot.cpp)
- A command is an ASCII string: getvar:version, download:00010000, flash:boot, erase:userdata, set_active:a, reboot-bootloader, oem ..., fetch:boot, snapshot-update:..., flashing:unlock / flashing:lock (constants.h and fastboot_driver.cpp)
- A response is a string up to 256 bytes with a 4-character prefix: OKAY (success), FAIL (error with message), INFO (progress lines, often many in a row), TEXT (diagnostics), DATA (hex size, the device then accepts raw bytes)
- The driver writes payload to the transport in 1024-byte chunks (TRANSPORT_CHUNK_SIZE), response timeout is 30 seconds (RESP_TIMEOUT)
- Device variables over getvar: unlocked, secure, current-slot, slot-count, has-slot, max-download-size, battery-soc-ok, battery-voltage, partition-size:<partition>, partition-type:<partition>. Every safety check before flashing builds on them
- A/B devices: two partition slots, set_active switches, snapshot-update cancels virtual A/B updates

## Boot image header (the skello-bootimg crate)

- The format is described in system/tools/mkbootimg/include/bootimg/bootimg.h. The file starts with the ANDROID! magic, the header version sits at byte offset 40 and defines the layout of the rest
- v0-v2: kernel_size at offset 8, ramdisk_size at 16, page_size at 36; v1 adds recovery_dtbo, v2 adds dtb. The header fits entirely into the first page of the image
- v3-v4: a compact 158-byte header (header_size at offset 20), the page size is fixed at 4096, the ramdisk moves into a separate vendor_boot image
- os_version packs the Android version and the security patch into one field: the top 21 bits hold major.minor.patch, the low 11 hold the year (from 2000) and the month. Zero means unset
- The end of the file may carry an AVB footer with the AVBf magic in the last 64 bytes - its presence means the image is vbmeta-signed
- Header verification is the first anti-brick barrier: Skello refuses to work with a file that does not read as a boot image of a known version

## Skello scenarios

### Bootloader unlock

Implemented in the wizard (screen opened from the fastboot device card menu): a wipe warning with a mandatory checkbox, getvar:unlocked check, fastboot flashing unlock with a 120 second timeout and confirmation on the phone screen, a repeated getvar check afterwards. The rest is the plan.

1. Reboot to bootloader: over adb (reboot bootloader) - already in the card menu; or by buttons
2. MediaTek and Unisoc have their own paths (seccfg with HACC, Identifier Token signing) - see [CHIPSETS.en.md](CHIPSETS.en.md), not implemented yet
3. Xiaomi requires Mi Unlock with a queue wait - outside the standard path

### Image flashing

Flashing goes through the own USB transport (nusb: ff/42/03 interface, bulk endpoints, 64-byte frames): the image streams via download with live progress over a flutter_rust_bridge stream, then flash:<partition>. If USB is unavailable, Skello falls back to the system fastboot without progress. The button is active only after the image header check, getvar:unlocked = yes and a dialog confirmation.

1. Checks before sending: battery-soc-ok reports "yes" (when the variable exists), image size within max-download-size - both via getvar before download
2. download:<size in hex>, then the image in 16 KiB chunks with progress
3. flash:<partition>; INFO lines are collected and shown with the output
4. Partitions to choose: boot, recovery, vendor_boot; the same place has partition erase with a confirmation and the bootloader check
5. Reboot and verify the device came back: after flashing a button sends reboot over USB and waits for the device in adb for up to 90 seconds, showing the result

### adb over USB

The adb work goes directly over USB without a server on port 5037: the ff/42/01 interface, the same bulk endpoints as fastboot. The codec is the 24-byte amessage with a checksum (byte sum) and magic; the CNXN handshake, on an AUTH token - a SHA1withRSA signature with the own key (the adbkey file, PKCS#1 PEM, in the app support directory), on rejection - publishing the key in the android pubkey format (len, n0inv, modulus, R², exponent - all little-endian) and waiting for the on-phone confirmation. Services open through A_OPEN: shell:<command> collects the output until CLSE acknowledging every WRTE, reboot[,target] reboots the phone. The device list is a USB enumeration with handshakes in threads: CNXN received - device, key not accepted - unauthorized, connection failed - offline. When USB is unavailable Skello falls back to the adb server and the system binaries.

### Root

All three methods are available on choice in the interface, the frame is the same: boot.img is patched on the computer side, the patched image is flashed over fastboot (or through DA/BROM on MediaTek and Unisoc with a locked bootloader).

- Magisk: Magisk init goes into the ramdisk, configuration through the Magisk app
- KernelSU: kernel or GKI image patch, control through the app
- APatch: boot.img patch with the kernel, control through the app

## Safety rules we build into the code

- No flashing command without an explicit confirmation showing the target partition
- Charge and bootloader state checked before download/flash - implemented: battery-soc-ok, unlocked, max-download-size
- Refusal to flash an image larger than max-download-size (splitting is on the plan)
- A log of every command and device response - implemented: skello-commands.log in the app support directory; what happened to the phone is reconstructable from the log
