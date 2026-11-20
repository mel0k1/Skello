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
      '1. Unlock the bootloader first. The unlock wizard comes next in the MVP plan.\n2. Download Magisk only from the official repository.\n3. Copy your verified boot.img to the phone.\n4. In the Magisk app: Install → Select and Patch a File → pick boot.img.\n5. Copy the resulting magisk_patched-XXXXX.img back to the PC.\n6. Skello will flash it with fastboot flash boot.';

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
}
