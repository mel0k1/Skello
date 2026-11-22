# Chipsets

Skello targets three platforms: MediaTek, Unisoc and Qualcomm. Samsung is out of scope. This file covers how each family enters flashing mode, what protocol the host speaks, and what Skello takes from existing tools.

The rule for reusing other projects: code from permissively licensed projects (public domain, BSD, MIT, Apache) is ported with attribution in NOTICE; from AGPL projects and unlicensed ones we take ideas, formats and step sequences only, and write our own code.

## MediaTek

The device enters BROM (boot ROM) - hold Volume Up and Volume Down on a powered-off phone while plugging USB - or loads its Preloader. The host talks to BROM, uploads a Download Agent (DA), and the DA performs partition read, write and erase.

What the references provide:

- penumbra (AGPL-3.0): partition flashing and readback, both DA generations - V5 (XFlash) and V6 (XML), bootloader unlocking on vulnerable devices, RPMB operations, arbitrary memory read and write, scatter-file flashing
- mtkclient-rs (Apache-2.0): pure Rust with no runtime dependencies, DA session reuse and resume of interrupted reads and writes, bootloader unlock through the seccfg partition with HACC encryption (V3/V4), vbmeta patching to disable AVB, supported chip table in code (exploit.rs and the loader table), WinUSB package in the release (windows/skello_winusb.inf)

Skello plan: stage 1 - BROM and Preloader transport, DA V5 upload, GPT and partition reading (implemented in crates/skello-chipsets/src/mediatek: the BROM handshake and SEND_DA/JUMP_DA, booting the stock DA from a loader file, the XFlash framing and readflash/writeflash, root-free seccfg unlock); stage 2 - DA V6 (XML), seccfg unlock on vulnerable chips, linecode (Kamakiri2) and Carbonara exploits, RPMB (implemented: the seccfg V4 hardware-key unlock - the HACC engine over the DA V5 custom registers in sej.rs, the V3/V4-legacy modes in patch_v4; DA V6 - the xml.rs XML session; Kamakiri2 - the linecode exploit in exploit.rs with a BootROM dump, a bypass and payload-blob injection for ten chips; RPMB - readflash/writeflash with parttype 3; Carbonara - the anti-patch detection, the hash position search (md5/sha1/sha256), the DA2 patches and the stage 2 boot over BOOT-TO, carried over from the reference and waiting for hardware validation; FORMAT (0x010003) - erasing a partition by its GPT name with CONTINUE/COMPLETE statuses; the scatter file - parser and image flashing, boot areas go into hardware boot1).

## Unisoc

BROM starts via a boot key (model specific) or a boot cable, the device shows up as USB 1782:4d00. The host loads FDL1 into SRAM at the address from the firmware XML, FDL1 initializes external memory and loads FDL2 into DRAM, and partition commands follow. The protocol has "transcode": bytes 0x7d and 0x7e get prefixed with 0x7d, and some FDLs turn that off by command. Bootloader unlock runs from fastboot: the Identifier Token is signed with an RSA key (known for chips like the T618) and sent to the device.

What the references provide:

- spreadtrum_flash (public domain): FDL upload, keep_charge, disable_transcode, partition listing, partition read and write, repartitioning, power off after work; FDL binaries come from the model-specific firmware (.pac), no universal set exists
- unisoc-unlock (no license): the unlock step sequence over fastboot and token signing; tested on the T618

Skello plan: stage 1 - BROM exchange, FDL upload, partition writing (implemented in crates/skello-chipsets/src/unisoc: the BSL framing with CRC16 and the sum checksum, transcode escaping and its shutdown, FDL1/FDL2 boot from the model firmware, partition read and write by name); stage 2 - GPT reading, token-signature unlock for chips with a known key (implemented: the partition table through READ_PARTITION/0x2D with 0xBA answers, 0x4C records with UTF-16 names and sector sizes; the token unlock for the T618 class - the identifier token from fastboot over the non-standard SN packets, the RSA-4096 PKCS#1 v1.5 SHA-256 signature with the community key, download + flashing unlock_bootloader; .pac flashing - the 2124-byte header and 3080-byte directory entries per unpac (Unlicense), CRC16 0xA001 over header and data, the FDL pair from the package, every flash file by its partition name, then a reset).

## Qualcomm

Emergency Download Mode (EDL, "9008") is started by the boot ROM itself, the device shows up as USB 05C6:9008 or a COM port / ttyUSB. The host uploads the programmer over Sahara (prog_firehose_ddr.elf, or xbl_s_devprg_ns.melf on newer platforms), then Firehose XML commands follow: read, write, erase by LBA, GPT handling, rawprogram and patch XML application, SHA256 verification. UFS requires an explicit LUN - the index of a physical storage partition.

What the references provide:

- qdlrs (BSD-3): Rust implementation of Sahara and Firehose, USB and serial backends, flasher with rawprogram, dump, write, erase, print-gpt, LUN selection, workaround for a missing HELLO packet
- qedl-rs (MIT): a modular crate stack (core, transport, Sahara, Firehose, storage, images, jobs), Android sparse image expansion, SHA256 checks, automatic DIAG to EDL switch, rawprogram.xml conversion

Skello plan: stage 1 - Sahara over USB, programmer upload, command mode (implemented in crates/skello-chipsets/src/qualcomm together with the Firehose XML commands, the rawprogram XML and sparse-to-raw expansion); stage 2 - the serial transport, GPT reading, Firehose erase and patch (implemented: Sahara over USB and serial - the 115200 8N1 serial link through serial2, the blind HELLO_RESP after a pause when the device stays silent, Firehose configure/program/read/reboot, the rawprogram XML and sparse-to-raw expansion; rawprogram flashing over serial and USB is available on the Chipsets screen). Implemented after stage 2: erase by LBA (`<erase first_lba last_lba PHYSICAL_PARTITION_NUM>`, the partition name through the GPT list read over LBAs 0..34), the patch XML (`<patch>` entries with filename=DISK - byte_offset, size_in_bytes, start_sector, value - host-side ones skipped, as qdlrs does), DA V6 - READ-FLASH/WRITE-FLASH/ERASE-FLASH/READ-PARTITION/WRITE-PARTITION/ERASE-PARTITION through UPLOAD-FILE/DOWNLOAD-FILE/PROGRESS-REPORT/FILE-SYS-OPERATION (xml.rs, session per mtkclient-rs), operation resume after a disconnect - read_flash_resumable/write_flash_resumable with session rebuilding (erase only before the first chunk) and a test against a dead link.

## Access level on Linux

Without udev rules only root can reach USB devices. Implemented: `udev/40-skello.rules` ships in the release archive (raw USB 0666 for 0e8d/05c6:9008/1782/18d1, tty permissions for the serial tools, qcserial binding to 9008 guarded by TEST). On Windows the transport runs through nusb and requires WinUSB: `windows/skello_winusb.inf` (MTK 0003/2000/2001, EDL 9008, Unisoc 4D00, Google fastboot d00d and adb 4ee0) installs through `windows/install_winusb.cmd` (pnputil with self-elevation); once WinUSB binds, the COM-port drivers stop claiming the ports - Skello never uses COM.
