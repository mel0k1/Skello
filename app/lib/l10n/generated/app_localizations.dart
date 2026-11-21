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

  /// No description provided for @flashBootButton.
  ///
  /// In en, this message translates to:
  /// **'Flash boot'**
  String get flashBootButton;

  /// No description provided for @flashConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Flash boot image?'**
  String get flashConfirmTitle;

  /// No description provided for @flashConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'Skello will run fastboot flash boot on {serial}. Interrupting the flash can brick the device.'**
  String flashConfirmBody(String serial);

  /// No description provided for @flashRunning.
  ///
  /// In en, this message translates to:
  /// **'Flashing… do not unplug the device'**
  String get flashRunning;

  /// No description provided for @flashDone.
  ///
  /// In en, this message translates to:
  /// **'boot flashed'**
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
