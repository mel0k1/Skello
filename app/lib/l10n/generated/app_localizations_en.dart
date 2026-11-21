// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appName => 'Skello';

  @override
  String get deviceNotConnectedTitle => 'Device not connected';

  @override
  String get deviceNotConnectedBody =>
      'No device shows up right now. Plug the phone in over USB, enable USB debugging, confirm the computer on the phone screen, or reboot the phone into fastboot mode. Then refresh.';

  @override
  String get refreshTooltip => 'Refresh';

  @override
  String get loadingDevices => 'Checking for devices…';

  @override
  String get sourcesUnavailableTitle => 'Device sources unavailable';

  @override
  String get sourcesUnavailableBody =>
      'Neither the Skello core nor the system adb and fastboot answered. Install Android platform-tools, make sure adb is on PATH, and refresh.';

  @override
  String get retryLabel => 'Retry';

  @override
  String get rootTooltip => 'Root tools';

  @override
  String get rootTitle => 'Root';

  @override
  String get rootIntro =>
      'Pick the root method for your device, select the boot image of your exact firmware and check it before anything touches the phone.';

  @override
  String get methodMagiskSubtitle =>
      'Patches the boot image with magiskinit. The official flow patches the image on the phone in the Magisk app.';

  @override
  String get methodKernelSuSubtitle =>
      'Root via the kernel or a loadable module. Needs GKI 5.10+ or a module built for your exact kernel.';

  @override
  String get methodApatchSubtitle =>
      'Kernel patch with kpatch. Needs a kpatch build matched to the kernel version of your firmware.';

  @override
  String get bootSectionTitle => 'Boot image';

  @override
  String get pickBootImage => 'Select boot.img';

  @override
  String get bootImageTypeLabel => 'Android boot image';

  @override
  String get validBootImage => 'Valid Android boot image';

  @override
  String get invalidBootImage =>
      'This file is not a readable Android boot image.';

  @override
  String get infoHeaderVersion => 'Header version';

  @override
  String get infoPageSize => 'Page size';

  @override
  String get infoKernelSize => 'Kernel size';

  @override
  String get infoRamdiskSize => 'Ramdisk size';

  @override
  String get infoOsVersion => 'Android version';

  @override
  String get infoOsPatchLevel => 'Security patch';

  @override
  String get infoFileSize => 'File size';

  @override
  String get infoAvbFooter => 'AVB footer';

  @override
  String get avbPresent => 'present';

  @override
  String get avbAbsent => 'absent';

  @override
  String get checkErrorTitle => 'Check failed';

  @override
  String get magiskGuide =>
      '1. Unlock the bootloader first - Skello has an unlock wizard for that.\n2. Download Magisk only from the official repository.\n3. Copy your verified boot.img to the phone.\n4. In the Magisk app: Install → Select and Patch a File → pick boot.img.\n5. Copy the resulting magisk_patched-XXXXX.img back to the PC.\n6. Skello will flash it with fastboot flash boot.';

  @override
  String get kernelsuGuide =>
      '1. Make sure the kernel supports GKI 5.10 or newer, or find an LKM module built for your exact kernel.\n2. KernelSU patches boot.img in its official app on the phone; modules for a specific kernel come from the official KernelSU builds.\n3. After patching, Skello will flash the image with fastboot flash boot.';

  @override
  String get apatchGuide =>
      '1. Take kpatch exactly for the kernel version of your firmware.\n2. APatch patches boot.img in its official app on the phone.\n3. After patching, Skello will flash the image with fastboot flash boot.';

  @override
  String get warningTitle => 'Before you flash';

  @override
  String get warningBody =>
      'Flashing a wrong image or a failed patch can leave the phone dead. A patched image requires an unlocked bootloader, and unlocking wipes the whole storage. Check the image against your exact model.';

  @override
  String get actionsTooltip => 'Actions';

  @override
  String get rebootMenuSystem => 'Reboot';

  @override
  String get rebootMenuBootloader => 'Reboot to bootloader';

  @override
  String get rebootMenuRecovery => 'Reboot to recovery';

  @override
  String get unlockMenu => 'Unlock bootloader…';

  @override
  String get actionStarted => 'Command sent';

  @override
  String get actionFailed => 'Command failed';

  @override
  String get unlockTitle => 'Unlock bootloader';

  @override
  String get unlockWarningTitle => 'This wipes everything';

  @override
  String get unlockWarningBody =>
      'Unlocking the bootloader wipes the whole storage with no way back, voids the warranty and may trip anti-rollback on some vendors. Proceed only with your own risk assessment.';

  @override
  String get unlockDevice => 'Device';

  @override
  String get unlockVendorNote =>
      'The standard path works on Pixel and most Android One devices. Xiaomi needs the Mi Unlock tool, MediaTek and Unisoc have their own routes - see the chipset notes. The wizard runs the standard path only.';

  @override
  String get unlockChecking => 'Checking bootloader state…';

  @override
  String get unlockAlready => 'Bootloader is already unlocked';

  @override
  String get unlockLocked => 'Bootloader is locked';

  @override
  String get unlockUnknown =>
      'Could not read the bootloader state. Check that fastboot works and the device is in fastboot mode.';

  @override
  String get unlockSteps =>
      '1. Skello runs fastboot flashing unlock.\n2. The phone shows a confirmation screen - choose Unlock there.\n3. The storage is wiped and the phone reboots.';

  @override
  String get unlockConfirm => 'I understand that all data will be erased';

  @override
  String get unlockRun => 'Unlock';

  @override
  String get unlockWaiting => 'Confirm on the phone screen…';

  @override
  String get unlockFailed => 'Unlock did not complete';

  @override
  String get flashSectionTitle => 'Flash';

  @override
  String get flashDeviceLabel => 'Device in fastboot';

  @override
  String get flashNoDevice =>
      'No device in fastboot mode. Reboot to bootloader first.';

  @override
  String get flashPickFirst => 'Select and verify a boot image first.';

  @override
  String get flashLocked => 'Bootloader is locked - unlock it first';

  @override
  String get flashUnlocked => 'Bootloader unlocked';

  @override
  String get flashStateUnknown => 'Bootloader state unknown';

  @override
  String get flashPartitionLabel => 'Partition';

  @override
  String flashPartitionButton(String partition) {
    return 'Flash $partition';
  }

  @override
  String get flashConfirmTitle => 'Flash partition?';

  @override
  String flashConfirmBody(String partition, String serial) {
    return 'Skello will run fastboot flash $partition on $serial. Interrupting the flash can brick the device.';
  }

  @override
  String get flashRunning => 'Flashing… do not unplug the device';

  @override
  String get flashPhaseDownload => 'Sending the image…';

  @override
  String get flashPhaseFlash => 'Writing the partition…';

  @override
  String get flashBatteryError =>
      'The device reports the battery is not ready to flash. Plug it in and wait for the charge.';

  @override
  String flashSizeError(String max, String image) {
    return 'The image is larger than the device allows: max $max B, image $image B.';
  }

  @override
  String get flashDone => 'image flashed';

  @override
  String get flashOutput => 'fastboot output';

  @override
  String get flashFailed => 'Flash failed';

  @override
  String erasePartitionButton(String partition) {
    return 'Erase $partition';
  }

  @override
  String get eraseConfirmTitle => 'Erase partition?';

  @override
  String eraseConfirmBody(String partition, String serial) {
    return 'Skello will run fastboot erase $partition on $serial. All data on the partition will be destroyed.';
  }

  @override
  String get eraseDone => 'partition erased';

  @override
  String get rebootCheckButton => 'Reboot and wait for the device';

  @override
  String get rebootWaiting => 'Rebooting… waiting for the device to come back';

  @override
  String get rebootBack => 'The device is back in the system';

  @override
  String get rebootTimeout =>
      'The device did not come back in time — check it manually';

  @override
  String get shellMenu => 'Shell';

  @override
  String get shellTitle => 'Shell';

  @override
  String get shellHint => 'adb shell command';

  @override
  String get shellSend => 'Run';

  @override
  String get shellFailed => 'Command failed';

  @override
  String get mediatekMenu => 'MediaTek seccfg';

  @override
  String get seccfgTitle => 'seccfg';

  @override
  String get seccfgStateLabel => 'Lock state';

  @override
  String get seccfgVersionLabel => 'Format';

  @override
  String get seccfgDetailLabel => 'Detail';

  @override
  String get seccfgUnlockButton => 'Unlock (seccfg)';

  @override
  String get seccfgUnlockConfirmTitle => 'Patch seccfg?';

  @override
  String seccfgUnlockConfirmBody(String serial) {
    return 'Skello will read seccfg on $serial, patch the lock flag to unlocked and write it back. Root access is required. A bad write can break the boot.';
  }

  @override
  String get seccfgUnlockDone => 'seccfg patched to unlocked';

  @override
  String get seccfgRootHint =>
      'Root access is required to read the seccfg block device.';

  @override
  String get seccfgNeedsSejHint =>
      'Only V3 partitions with the software SEJ key are patched locally; hardware-keyed partitions need the Download Agent stage, not implemented yet.';

  @override
  String get seccfgFailed => 'seccfg failed';

  @override
  String get cancelLabel => 'Cancel';
}
