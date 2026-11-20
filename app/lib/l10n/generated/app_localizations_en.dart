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
      'Skello is an early build: the device list shows up here when the Rust core starts feeding data to the UI. Plug the phone in over USB and turn on debugging ahead of time.';
}
