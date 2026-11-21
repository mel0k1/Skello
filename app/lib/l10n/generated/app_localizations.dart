import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_ru.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'generated/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
      : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations? of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations);
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
    delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
  ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('ru')
  ];

  /// No description provided for @appName.
  ///
  /// In en, this message translates to:
  /// **'Skello'**
  String get appName;

  /// No description provided for @deviceNotConnectedTitle.
  ///
  /// In en, this message translates to:
  /// **'Device not connected'**
  String get deviceNotConnectedTitle;

  /// No description provided for @deviceNotConnectedBody.
  ///
  /// In en, this message translates to:
  /// **'No device shows up right now. Plug the phone in over USB, enable USB debugging, confirm the computer on the phone screen, or reboot the phone into fastboot mode. Then refresh.'**
  String get deviceNotConnectedBody;

  /// No description provided for @refreshTooltip.
  ///
  /// In en, this message translates to:
  /// **'Refresh'**
  String get refreshTooltip;

  /// No description provided for @loadingDevices.
  ///
  /// In en, this message translates to:
  /// **'Checking for devices…'**
  String get loadingDevices;

  /// No description provided for @sourcesUnavailableTitle.
  ///
  /// In en, this message translates to:
  /// **'Device sources unavailable'**
  String get sourcesUnavailableTitle;

  /// No description provided for @sourcesUnavailableBody.
  ///
  /// In en, this message translates to:
  /// **'Neither the Skello core nor the system adb and fastboot answered. Install Android platform-tools, make sure adb is on PATH, and refresh.'**
  String get sourcesUnavailableBody;

  /// No description provided for @retryLabel.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get retryLabel;

  /// No description provided for @rootTooltip.
  ///
  /// In en, this message translates to:
  /// **'Root tools'**
  String get rootTooltip;

  /// No description provided for @rootTitle.
  ///
  /// In en, this message translates to:
  /// **'Root'**
  String get rootTitle;

  /// No description provided for @rootIntro.
  ///
  /// In en, this message translates to:
  /// **'Pick the root method for your device, select the boot image of your exact firmware and check it before anything touches the phone.'**
  String get rootIntro;

  /// No description provided for @methodMagiskSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Patches the boot image with magiskinit. The official flow patches the image on the phone in the Magisk app.'**
  String get methodMagiskSubtitle;

  /// No description provided for @methodKernelSuSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Root via the kernel or a loadable module. Needs GKI 5.10+ or a module built for your exact kernel.'**
  String get methodKernelSuSubtitle;

  /// No description provided for @methodApatchSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Kernel patch with kpatch. Needs a kpatch build matched to the kernel version of your firmware.'**
  String get methodApatchSubtitle;

  /// No description provided for @bootSectionTitle.
  ///
  /// In en, this message translates to:
  /// **'Boot image'**
  String get bootSectionTitle;

  /// No description provided for @pickBootImage.
  ///
  /// In en, this message translates to:
  /// **'Select boot.img'**
  String get pickBootImage;

  /// No description provided for @bootImageTypeLabel.
  ///
  /// In en, this message translates to:
  /// **'Android boot image'**
  String get bootImageTypeLabel;

  /// No description provided for @validBootImage.
  ///
  /// In en, this message translates to:
  /// **'Valid Android boot image'**
  String get validBootImage;

  /// No description provided for @invalidBootImage.
  ///
  /// In en, this message translates to:
  /// **'This file is not a readable Android boot image.'**
  String get invalidBootImage;

  /// No description provided for @infoHeaderVersion.
  ///
  /// In en, this message translates to:
  /// **'Header version'**
  String get infoHeaderVersion;

  /// No description provided for @infoPageSize.
  ///
  /// In en, this message translates to:
  /// **'Page size'**
  String get infoPageSize;

  /// No description provided for @infoKernelSize.
  ///
  /// In en, this message translates to:
  /// **'Kernel size'**
  String get infoKernelSize;

  /// No description provided for @infoRamdiskSize.
  ///
  /// In en, this message translates to:
  /// **'Ramdisk size'**
  String get infoRamdiskSize;

  /// No description provided for @infoOsVersion.
  ///
  /// In en, this message translates to:
  /// **'Android version'**
  String get infoOsVersion;

  /// No description provided for @infoOsPatchLevel.
  ///
  /// In en, this message translates to:
  /// **'Security patch'**
  String get infoOsPatchLevel;

  /// No description provided for @infoFileSize.
  ///
  /// In en, this message translates to:
  /// **'File size'**
  String get infoFileSize;

  /// No description provided for @infoAvbFooter.
  ///
  /// In en, this message translates to:
  /// **'AVB footer'**
  String get infoAvbFooter;

  /// No description provided for @avbPresent.
  ///
  /// In en, this message translates to:
  /// **'present'**
  String get avbPresent;

  /// No description provided for @avbAbsent.
  ///
  /// In en, this message translates to:
  /// **'absent'**
  String get avbAbsent;

  /// No description provided for @checkErrorTitle.
  ///
  /// In en, this message translates to:
  /// **'Check failed'**
  String get checkErrorTitle;

  /// No description provided for @magiskGuide.
  ///
  /// In en, this message translates to:
  /// **'1. Unlock the bootloader first - Skello has an unlock wizard for that.\n2. Download Magisk only from the official repository.\n3. Copy your verified boot.img to the phone.\n4. In the Magisk app: Install → Select and Patch a File → pick boot.img.\n5. Copy the resulting magisk_patched-XXXXX.img back to the PC.\n6. Skello will flash it with fastboot flash boot.'**
  String get magiskGuide;

  /// No description provided for @kernelsuGuide.
  ///
  /// In en, this message translates to:
  /// **'1. Make sure the kernel supports GKI 5.10 or newer, or find an LKM module built for your exact kernel.\n2. KernelSU patches boot.img in its official app on the phone; modules for a specific kernel come from the official KernelSU builds.\n3. After patching, Skello will flash the image with fastboot flash boot.'**
  String get kernelsuGuide;

  /// No description provided for @apatchGuide.
  ///
  /// In en, this message translates to:
  /// **'1. Take kpatch exactly for the kernel version of your firmware.\n2. APatch patches boot.img in its official app on the phone.\n3. After patching, Skello will flash the image with fastboot flash boot.'**
  String get apatchGuide;

  /// No description provided for @warningTitle.
  ///
  /// In en, this message translates to:
  /// **'Before you flash'**
  String get warningTitle;

  /// No description provided for @warningBody.
  ///
  /// In en, this message translates to:
  /// **'Flashing a wrong image or a failed patch can leave the phone dead. A patched image requires an unlocked bootloader, and unlocking wipes the whole storage. Check the image against your exact model.'**
  String get warningBody;

  /// No description provided for @actionsTooltip.
  ///
  /// In en, this message translates to:
  /// **'Actions'**
  String get actionsTooltip;

  /// No description provided for @rebootMenuSystem.
  ///
  /// In en, this message translates to:
  /// **'Reboot'**
  String get rebootMenuSystem;

  /// No description provided for @rebootMenuBootloader.
  ///
  /// In en, this message translates to:
  /// **'Reboot to bootloader'**
  String get rebootMenuBootloader;

  /// No description provided for @rebootMenuRecovery.
  ///
  /// In en, this message translates to:
  /// **'Reboot to recovery'**
  String get rebootMenuRecovery;

  /// No description provided for @unlockMenu.
  ///
  /// In en, this message translates to:
  /// **'Unlock bootloader…'**
  String get unlockMenu;

  /// No description provided for @actionStarted.
  ///
  /// In en, this message translates to:
  /// **'Command sent'**
  String get actionStarted;

  /// No description provided for @actionFailed.
  ///
  /// In en, this message translates to:
  /// **'Command failed'**
  String get actionFailed;

  /// No description provided for @unlockTitle.
  ///
  /// In en, this message translates to:
  /// **'Unlock bootloader'**
  String get unlockTitle;

  /// No description provided for @unlockWarningTitle.
  ///
  /// In en, this message translates to:
  /// **'This wipes everything'**
  String get unlockWarningTitle;

  /// No description provided for @unlockWarningBody.
  ///
  /// In en, this message translates to:
  /// **'Unlocking the bootloader wipes the whole storage with no way back, voids the warranty and may trip anti-rollback on some vendors. Proceed only with your own risk assessment.'**
  String get unlockWarningBody;

  /// No description provided for @unlockDevice.
  ///
  /// In en, this message translates to:
  /// **'Device'**
  String get unlockDevice;

  /// No description provided for @unlockVendorNote.
  ///
  /// In en, this message translates to:
  /// **'The standard path works on Pixel and most Android One devices. Xiaomi needs the Mi Unlock tool, MediaTek and Unisoc have their own routes - see the chipset notes. The wizard runs the standard path only.'**
  String get unlockVendorNote;

  /// No description provided for @unlockChecking.
  ///
  /// In en, this message translates to:
  /// **'Checking bootloader state…'**
  String get unlockChecking;

  /// No description provided for @unlockAlready.
  ///
  /// In en, this message translates to:
  /// **'Bootloader is already unlocked'**
  String get unlockAlready;

  /// No description provided for @unlockLocked.
  ///
  /// In en, this message translates to:
  /// **'Bootloader is locked'**
  String get unlockLocked;

  /// No description provided for @unlockUnknown.
  ///
  /// In en, this message translates to:
  /// **'Could not read the bootloader state. Check that fastboot works and the device is in fastboot mode.'**
  String get unlockUnknown;

  /// No description provided for @unlockSteps.
  ///
  /// In en, this message translates to:
  /// **'1. Skello runs fastboot flashing unlock.\n2. The phone shows a confirmation screen - choose Unlock there.\n3. The storage is wiped and the phone reboots.'**
  String get unlockSteps;

  /// No description provided for @unlockConfirm.
  ///
  /// In en, this message translates to:
  /// **'I understand that all data will be erased'**
  String get unlockConfirm;

  /// No description provided for @unlockRun.
  ///
  /// In en, this message translates to:
  /// **'Unlock'**
  String get unlockRun;

  /// No description provided for @unlockWaiting.
  ///
  /// In en, this message translates to:
  /// **'Confirm on the phone screen…'**
  String get unlockWaiting;

  /// No description provided for @unlockFailed.
  ///
  /// In en, this message translates to:
  /// **'Unlock did not complete'**
  String get unlockFailed;

  /// No description provided for @flashSectionTitle.
  ///
  /// In en, this message translates to:
  /// **'Flash'**
  String get flashSectionTitle;

  /// No description provided for @flashDeviceLabel.
  ///
  /// In en, this message translates to:
  /// **'Device in fastboot'**
  String get flashDeviceLabel;

  /// No description provided for @flashNoDevice.
  ///
  /// In en, this message translates to:
  /// **'No device in fastboot mode. Reboot to bootloader first.'**
  String get flashNoDevice;

  /// No description provided for @flashPickFirst.
  ///
  /// In en, this message translates to:
  /// **'Select and verify a boot image first.'**
  String get flashPickFirst;

  /// No description provided for @flashLocked.
  ///
  /// In en, this message translates to:
  /// **'Bootloader is locked - unlock it first'**
  String get flashLocked;

  /// No description provided for @flashUnlocked.
  ///
  /// In en, this message translates to:
  /// **'Bootloader unlocked'**
  String get flashUnlocked;

  /// No description provided for @flashStateUnknown.
  ///
  /// In en, this message translates to:
  /// **'Bootloader state unknown'**
  String get flashStateUnknown;

  /// No description provided for @flashPartitionLabel.
  ///
  /// In en, this message translates to:
  /// **'Partition'**
  String get flashPartitionLabel;

  /// No description provided for @flashPartitionButton.
  ///
  /// In en, this message translates to:
  /// **'Flash {partition}'**
  String flashPartitionButton(String partition);

  /// No description provided for @flashConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Flash partition?'**
  String get flashConfirmTitle;

  /// No description provided for @flashConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'Skello will run fastboot flash {partition} on {serial}. Interrupting the flash can brick the device.'**
  String flashConfirmBody(String partition, String serial);

  /// No description provided for @flashRunning.
  ///
  /// In en, this message translates to:
  /// **'Flashing… do not unplug the device'**
  String get flashRunning;

  /// No description provided for @flashPhaseDownload.
  ///
  /// In en, this message translates to:
  /// **'Sending the image…'**
  String get flashPhaseDownload;

  /// No description provided for @flashPhaseFlash.
  ///
  /// In en, this message translates to:
  /// **'Writing the partition…'**
  String get flashPhaseFlash;

  /// No description provided for @flashBatteryError.
  ///
  /// In en, this message translates to:
  /// **'The device reports the battery is not ready to flash. Plug it in and wait for the charge.'**
  String get flashBatteryError;

  /// No description provided for @flashSizeError.
  ///
  /// In en, this message translates to:
  /// **'The image is larger than the device allows: max {max} B, image {image} B.'**
  String flashSizeError(String max, String image);

  /// No description provided for @flashDone.
  ///
  /// In en, this message translates to:
  /// **'image flashed'**
  String get flashDone;

  /// No description provided for @flashOutput.
  ///
  /// In en, this message translates to:
  /// **'fastboot output'**
  String get flashOutput;

  /// No description provided for @flashFailed.
  ///
  /// In en, this message translates to:
  /// **'Flash failed'**
  String get flashFailed;

  /// No description provided for @erasePartitionButton.
  ///
  /// In en, this message translates to:
  /// **'Erase {partition}'**
  String erasePartitionButton(String partition);

  /// No description provided for @eraseConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Erase partition?'**
  String get eraseConfirmTitle;

  /// No description provided for @eraseConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'Skello will run fastboot erase {partition} on {serial}. All data on the partition will be destroyed.'**
  String eraseConfirmBody(String partition, String serial);

  /// No description provided for @eraseDone.
  ///
  /// In en, this message translates to:
  /// **'partition erased'**
  String get eraseDone;

  /// No description provided for @rebootCheckButton.
  ///
  /// In en, this message translates to:
  /// **'Reboot and wait for the device'**
  String get rebootCheckButton;

  /// No description provided for @rebootWaiting.
  ///
  /// In en, this message translates to:
  /// **'Rebooting… waiting for the device to come back'**
  String get rebootWaiting;

  /// No description provided for @rebootBack.
  ///
  /// In en, this message translates to:
  /// **'The device is back in the system'**
  String get rebootBack;

  /// No description provided for @rebootTimeout.
  ///
  /// In en, this message translates to:
  /// **'The device did not come back in time — check it manually'**
  String get rebootTimeout;

  /// No description provided for @shellMenu.
  ///
  /// In en, this message translates to:
  /// **'Shell'**
  String get shellMenu;

  /// No description provided for @shellTitle.
  ///
  /// In en, this message translates to:
  /// **'Shell'**
  String get shellTitle;

  /// No description provided for @shellHint.
  ///
  /// In en, this message translates to:
  /// **'adb shell command'**
  String get shellHint;

  /// No description provided for @shellSend.
  ///
  /// In en, this message translates to:
  /// **'Run'**
  String get shellSend;

  /// No description provided for @shellFailed.
  ///
  /// In en, this message translates to:
  /// **'Command failed'**
  String get shellFailed;

  /// No description provided for @mediatekMenu.
  ///
  /// In en, this message translates to:
  /// **'MediaTek seccfg'**
  String get mediatekMenu;

  /// No description provided for @seccfgTitle.
  ///
  /// In en, this message translates to:
  /// **'seccfg'**
  String get seccfgTitle;

  /// No description provided for @seccfgStateLabel.
  ///
  /// In en, this message translates to:
  /// **'Lock state'**
  String get seccfgStateLabel;

  /// No description provided for @seccfgVersionLabel.
  ///
  /// In en, this message translates to:
  /// **'Format'**
  String get seccfgVersionLabel;

  /// No description provided for @seccfgDetailLabel.
  ///
  /// In en, this message translates to:
  /// **'Detail'**
  String get seccfgDetailLabel;

  /// No description provided for @seccfgUnlockButton.
  ///
  /// In en, this message translates to:
  /// **'Unlock (seccfg)'**
  String get seccfgUnlockButton;

  /// No description provided for @seccfgUnlockConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Patch seccfg?'**
  String get seccfgUnlockConfirmTitle;

  /// No description provided for @seccfgUnlockConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'Skello will read seccfg on {serial}, patch the lock flag to unlocked and write it back. Root access is required. A bad write can break the boot.'**
  String seccfgUnlockConfirmBody(String serial);

  /// No description provided for @seccfgUnlockDone.
  ///
  /// In en, this message translates to:
  /// **'seccfg patched to unlocked'**
  String get seccfgUnlockDone;

  /// No description provided for @seccfgRootHint.
  ///
  /// In en, this message translates to:
  /// **'Root access is required to read the seccfg block device.'**
  String get seccfgRootHint;

  /// No description provided for @seccfgNeedsSejHint.
  ///
  /// In en, this message translates to:
  /// **'Only V3 partitions with the software SEJ key are patched locally; hardware-keyed partitions need the Download Agent stage, not implemented yet.'**
  String get seccfgNeedsSejHint;

  /// No description provided for @seccfgFailed.
  ///
  /// In en, this message translates to:
  /// **'seccfg failed'**
  String get seccfgFailed;

  /// No description provided for @chipsetsTitle.
  ///
  /// In en, this message translates to:
  /// **'Chipsets'**
  String get chipsetsTitle;

  /// No description provided for @chipsetsTooltip.
  ///
  /// In en, this message translates to:
  /// **'Vendor modes'**
  String get chipsetsTooltip;

  /// No description provided for @chipsetsLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not list the USB ports'**
  String get chipsetsLoadFailed;

  /// No description provided for @chipsetsPortsNone.
  ///
  /// In en, this message translates to:
  /// **'No port detected'**
  String get chipsetsPortsNone;

  /// No description provided for @chipsetsPickType.
  ///
  /// In en, this message translates to:
  /// **'Firmware files'**
  String get chipsetsPickType;

  /// No description provided for @chipsetsConfirmGo.
  ///
  /// In en, this message translates to:
  /// **'Run it'**
  String get chipsetsConfirmGo;

  /// No description provided for @chipsetsAddrInvalid.
  ///
  /// In en, this message translates to:
  /// **'Addresses must be hex numbers, for example 0x00090000'**
  String get chipsetsAddrInvalid;

  /// No description provided for @chipsetsMtkTitle.
  ///
  /// In en, this message translates to:
  /// **'MediaTek: BROM + Download Agent'**
  String get chipsetsMtkTitle;

  /// No description provided for @chipsetsMtkHint.
  ///
  /// In en, this message translates to:
  /// **'Power the phone off, hold BROM or let the battery drain; the BROM/Preloader port shows up when the cable is plugged in.'**
  String get chipsetsMtkHint;

  /// No description provided for @chipsetsMtkLoaderNone.
  ///
  /// In en, this message translates to:
  /// **'Stock Download Agent loader not chosen'**
  String get chipsetsMtkLoaderNone;

  /// No description provided for @chipsetsMtkLoaderPick.
  ///
  /// In en, this message translates to:
  /// **'Choose DA loader'**
  String get chipsetsMtkLoaderPick;

  /// No description provided for @chipsetsMtkUnlock.
  ///
  /// In en, this message translates to:
  /// **'Unlock bootloader (no root)'**
  String get chipsetsMtkUnlock;

  /// No description provided for @chipsetsMtkUnlockConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Unlock via Download Agent?'**
  String get chipsetsMtkUnlockConfirmTitle;

  /// No description provided for @chipsetsMtkUnlockConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'Skello boots the stock loader over BROM, reads seccfg, patches the lock flag to unlocked and writes it back. Root is not needed, but the loader file of your model is. A bad write can break the boot.'**
  String get chipsetsMtkUnlockConfirmBody;

  /// No description provided for @chipsetsMtkDone.
  ///
  /// In en, this message translates to:
  /// **'seccfg patched over the Download Agent'**
  String get chipsetsMtkDone;

  /// No description provided for @chipsetsMtkV6Boot.
  ///
  /// In en, this message translates to:
  /// **'Boot DA V6 (XML)'**
  String get chipsetsMtkV6Boot;

  /// No description provided for @chipsetsMtkV6Fastboot.
  ///
  /// In en, this message translates to:
  /// **'Fastboot'**
  String get chipsetsMtkV6Fastboot;

  /// No description provided for @chipsetsMtkV6Meta.
  ///
  /// In en, this message translates to:
  /// **'Meta'**
  String get chipsetsMtkV6Meta;

  /// No description provided for @chipsetsMtkV6Done.
  ///
  /// In en, this message translates to:
  /// **'DA V6 booted'**
  String get chipsetsMtkV6Done;

  /// No description provided for @chipsetsUnisocTitle.
  ///
  /// In en, this message translates to:
  /// **'Unisoc: FDL'**
  String get chipsetsUnisocTitle;

  /// No description provided for @chipsetsUnisocHint.
  ///
  /// In en, this message translates to:
  /// **'Choose FDL1 and FDL2 from the factory firmware of your model; the addresses below are the common defaults.'**
  String get chipsetsUnisocHint;

  /// No description provided for @chipsetsUnisocPickFdl1.
  ///
  /// In en, this message translates to:
  /// **'Choose FDL1'**
  String get chipsetsUnisocPickFdl1;

  /// No description provided for @chipsetsUnisocPickFdl2.
  ///
  /// In en, this message translates to:
  /// **'Choose FDL2'**
  String get chipsetsUnisocPickFdl2;

  /// No description provided for @chipsetsUnisocAddr1Label.
  ///
  /// In en, this message translates to:
  /// **'FDL1 address'**
  String get chipsetsUnisocAddr1Label;

  /// No description provided for @chipsetsUnisocAddr2Label.
  ///
  /// In en, this message translates to:
  /// **'FDL2 address'**
  String get chipsetsUnisocAddr2Label;

  /// No description provided for @chipsetsUnisocAddrHint.
  ///
  /// In en, this message translates to:
  /// **'Hex, for example 0x00090000'**
  String get chipsetsUnisocAddrHint;

  /// No description provided for @chipsetsUnisocPartitionLabel.
  ///
  /// In en, this message translates to:
  /// **'Partition name'**
  String get chipsetsUnisocPartitionLabel;

  /// No description provided for @chipsetsUnisocImageNone.
  ///
  /// In en, this message translates to:
  /// **'Image not chosen'**
  String get chipsetsUnisocImageNone;

  /// No description provided for @chipsetsUnisocImagePick.
  ///
  /// In en, this message translates to:
  /// **'Choose image'**
  String get chipsetsUnisocImagePick;

  /// No description provided for @chipsetsUnisocFlash.
  ///
  /// In en, this message translates to:
  /// **'Flash partition'**
  String get chipsetsUnisocFlash;

  /// No description provided for @chipsetsUnisocConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Flash over FDL?'**
  String get chipsetsUnisocConfirmTitle;

  /// No description provided for @chipsetsUnisocConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'Skello boots the factory loaders and writes {image} into {partition}. Wrong addresses or a foreign image can break the boot.'**
  String chipsetsUnisocConfirmBody(String image, String partition);

  /// No description provided for @chipsetsUnisocDone.
  ///
  /// In en, this message translates to:
  /// **'Partition written over FDL2'**
  String get chipsetsUnisocDone;

  /// No description provided for @chipsetsUnisocTable.
  ///
  /// In en, this message translates to:
  /// **'Read partition table'**
  String get chipsetsUnisocTable;

  /// No description provided for @chipsetsUnisocTableDone.
  ///
  /// In en, this message translates to:
  /// **'Partition table'**
  String get chipsetsUnisocTableDone;

  /// No description provided for @chipsetsUnisocSectors.
  ///
  /// In en, this message translates to:
  /// **'{count} sectors'**
  String chipsetsUnisocSectors(int count);

  /// No description provided for @chipsetsQualcommTitle.
  ///
  /// In en, this message translates to:
  /// **'Qualcomm: EDL (Sahara + Firehose)'**
  String get chipsetsQualcommTitle;

  /// No description provided for @chipsetsQualcommHint.
  ///
  /// In en, this message translates to:
  /// **'The EDL port appears when the phone is plugged in dead mode; the programmer and rawprogram XML come from your model\'s firmware package.'**
  String get chipsetsQualcommHint;

  /// No description provided for @chipsetsQualcommProgrammerNone.
  ///
  /// In en, this message translates to:
  /// **'Firehose programmer not chosen'**
  String get chipsetsQualcommProgrammerNone;

  /// No description provided for @chipsetsQualcommProgrammerPick.
  ///
  /// In en, this message translates to:
  /// **'Choose programmer'**
  String get chipsetsQualcommProgrammerPick;

  /// No description provided for @chipsetsQualcommXmlPick.
  ///
  /// In en, this message translates to:
  /// **'Choose rawprogram XML'**
  String get chipsetsQualcommXmlPick;

  /// No description provided for @chipsetsQualcommParse.
  ///
  /// In en, this message translates to:
  /// **'Parse XML'**
  String get chipsetsQualcommParse;

  /// No description provided for @chipsetsQualcommEntriesLabel.
  ///
  /// In en, this message translates to:
  /// **'{count} entries in the XML'**
  String chipsetsQualcommEntriesLabel(int count);

  /// No description provided for @chipsetsQualcommEntryLine.
  ///
  /// In en, this message translates to:
  /// **'{label}: {file} @ sector {sector} ({sectors})'**
  String chipsetsQualcommEntryLine(
      String label, String file, String sector, String sectors);

  /// No description provided for @chipsetsQualcommFlash.
  ///
  /// In en, this message translates to:
  /// **'Flash all entries'**
  String get chipsetsQualcommFlash;

  /// No description provided for @chipsetsQualcommConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Flash over EDL?'**
  String get chipsetsQualcommConfirmTitle;

  /// No description provided for @chipsetsQualcommConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'Skello serves the programmer over Sahara and writes every rawprogram entry. Partitions are rewritten without a confirmation on the phone.'**
  String get chipsetsQualcommConfirmBody;

  /// No description provided for @chipsetsQualcommDone.
  ///
  /// In en, this message translates to:
  /// **'rawprogram finished'**
  String get chipsetsQualcommDone;

  /// No description provided for @chipsetsQualcommSerial.
  ///
  /// In en, this message translates to:
  /// **'EDL over serial (UART)'**
  String get chipsetsQualcommSerial;

  /// No description provided for @chipsetsQualcommSerialLoad.
  ///
  /// In en, this message translates to:
  /// **'Scan serial ports'**
  String get chipsetsQualcommSerialLoad;

  /// No description provided for @chipsetsQualcommSerialNone.
  ///
  /// In en, this message translates to:
  /// **'No serial port found'**
  String get chipsetsQualcommSerialNone;

  /// No description provided for @chipsetsQualcommSerialFlash.
  ///
  /// In en, this message translates to:
  /// **'Flash over serial'**
  String get chipsetsQualcommSerialFlash;

  /// No description provided for @chipsetsQualcommSerialDone.
  ///
  /// In en, this message translates to:
  /// **'rawprogram finished over serial'**
  String get chipsetsQualcommSerialDone;

  /// No description provided for @chipsetsExploitTitle.
  ///
  /// In en, this message translates to:
  /// **'Kamakiri2 exploits'**
  String get chipsetsExploitTitle;

  /// No description provided for @chipsetsExploitHint.
  ///
  /// In en, this message translates to:
  /// **'Vulnerable chips only (MT6755..MT6785): BootROM dump, DA-auth bypass and the Carbonara patched stage 2.'**
  String get chipsetsExploitHint;

  /// No description provided for @chipsetsExploitDump.
  ///
  /// In en, this message translates to:
  /// **'Dump BootROM'**
  String get chipsetsExploitDump;

  /// No description provided for @chipsetsExploitKamakiri.
  ///
  /// In en, this message translates to:
  /// **'Kamakiri2 unlock'**
  String get chipsetsExploitKamakiri;

  /// No description provided for @chipsetsExploitCarbonara.
  ///
  /// In en, this message translates to:
  /// **'Carbonara unlock'**
  String get chipsetsExploitCarbonara;

  /// No description provided for @chipsetsExploitDone.
  ///
  /// In en, this message translates to:
  /// **'Exploit flow finished'**
  String get chipsetsExploitDone;

  /// No description provided for @chipsetsExploitConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Run the exploit?'**
  String get chipsetsExploitConfirmTitle;

  /// No description provided for @chipsetsExploitConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'The exploit talks to the bootrom in BROM mode. Continue only with the device connected in BROM.'**
  String get chipsetsExploitConfirmBody;

  /// No description provided for @chipsetsRpmbRead.
  ///
  /// In en, this message translates to:
  /// **'Read RPMB block'**
  String get chipsetsRpmbRead;

  /// No description provided for @chipsetsRpmbDone.
  ///
  /// In en, this message translates to:
  /// **'RPMB block read'**
  String get chipsetsRpmbDone;

  /// No description provided for @chipsetsUnisocUnlock.
  ///
  /// In en, this message translates to:
  /// **'Unlock bootloader (T618)'**
  String get chipsetsUnisocUnlock;

  /// No description provided for @chipsetsUnisocUnlockHint.
  ///
  /// In en, this message translates to:
  /// **'Signs the identifier token with the community T618 RSA key and runs flashing unlock_bootloader.'**
  String get chipsetsUnisocUnlockHint;

  /// No description provided for @chipsetsUnisocUnlockDone.
  ///
  /// In en, this message translates to:
  /// **'Unlock flow finished'**
  String get chipsetsUnisocUnlockDone;

  /// No description provided for @cancelLabel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancelLabel;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'ru'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'ru':
      return AppLocalizationsRu();
  }

  throw FlutterError(
      'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
      'an issue with the localizations generation tool. Please file an issue '
      'on GitHub with a reproducible sample app and the gen-l10n configuration '
      'that was used.');
}
