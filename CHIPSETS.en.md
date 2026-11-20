# Chipsets

Skello targets three platforms: MediaTek, Unisoc and Qualcomm. Samsung is out of scope. This file covers how each family enters flashing mode, what protocol the host speaks, and what Skello takes from existing tools.

The rule for reusing other projects: code from permissively licensed projects (public domain, BSD, MIT, Apache) is ported with attribution in NOTICE; from AGPL projects and unlicensed ones we take ideas, formats and step sequences only, and write our own code.

## MediaTek

The device enters BROM (boot ROM) - hold Volume Up and Volume Down on a powered-off phone while plugging USB - or loads its Preloader. The host talks to BROM, uploads a Download Agent (DA), and the DA performs partition read, write and erase.

What the references provide:

- penumbra (AGPL-3.0): partition flashing and readback, both DA generations - V5 (XFlash) and V6 (XML), bootloader unlocking on vulnerable devices, RPMB operations, arbitrary memory read and write, scatter-file flashing
- mtkclient-rs (Apache-2.0): pure Rust with no runtime dependencies, DA session reuse and resume of interrupted reads and writes, bootloader unlock through the seccfg partition with HACC encryption (V3/V4), vbmeta patching to disable AVB, runtime chip table (sdata.json), automatic WinUSB driver installation on Windows

Skello plan: stage 1 - BROM and Preloader transport, DA V5/V6 upload, GPT and partition reading; stage 2 - seccfg unlock on vulnerable chips, linecode (Kamakiri2) and Carbonara exploits, RPMB.

## Unisoc

BROM starts via a boot key (model specific) or a boot cable, the device shows up as USB 1782:4d00. The host loads FDL1 into SRAM at the address from the firmware XML, FDL1 initializes external memory and loads FDL2 into DRAM, and partition commands follow. The protocol has "transcode": bytes 0x7d and 0x7e get prefixed with 0x7d, and some FDLs turn that off by command. Bootloader unlock runs from fastboot: the Identifier Token is signed with an RSA key (known for chips like the T618) and sent to the device.

What the references provide:

- spreadtrum_flash (public domain): FDL upload, keep_charge, disable_transcode, partition listing, partition read and write, repartitioning, power off after work; FDL binaries come from the model-specific firmware (.pac), no universal set exists
- unisoc-unlock (no license): the unlock step sequence over fastboot and token signing; tested on the T618

Skello plan: stage 1 - BROM exchange, FDL upload, GPT reading; stage 2 - partition writing, token-signature unlock for chips with a known key.

## Qualcomm

Emergency Download Mode (EDL, "9008") is started by the boot ROM itself, the device shows up as USB 05C6:9008 or a COM port / ttyUSB. The host uploads the programmer over Sahara (prog_firehose_ddr.elf, or xbl_s_devprg_ns.melf on newer platforms), then Firehose XML commands follow: read, write, erase by LBA, GPT handling, rawprogram and patch XML application, SHA256 verification. UFS requires an explicit LUN - the index of a physical storage partition.

What the references provide:

- qdlrs (BSD-3): Rust implementation of Sahara and Firehose, USB and serial backends, flasher with rawprogram, dump, write, erase, print-gpt, LUN selection, workaround for a missing HELLO packet
- qedl-rs (MIT): a modular crate stack (core, transport, Sahara, Firehose, storage, images, jobs), Android sparse image expansion, SHA256 checks, automatic DIAG to EDL switch, rawprogram.xml conversion

Skello plan: stage 1 - Sahara over serial transport, programmer upload, GPT reading; stage 2 - Firehose write and erase, rawprogram.xml, sparse images.

## Access level on Linux

Without udev rules only root can reach USB devices. Skello will offer to install rules for the vendors: 1782 (Unisoc), 0e8d (MediaTek), 05c6 (Qualcomm) - the same approach spreadtrum_flash documents.
