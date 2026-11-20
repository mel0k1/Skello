// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Russian (`ru`).
class AppLocalizationsRu extends AppLocalizations {
  AppLocalizationsRu([String locale = 'ru']) : super(locale);

  @override
  String get appName => 'Skello';

  @override
  String get deviceNotConnectedTitle => 'Устройство не подключено';

  @override
  String get deviceNotConnectedBody =>
      'Skello в ранней сборке: здесь появится список устройств, когда Rust-ядро начнёт передавать данные в интерфейс. Подключите телефон по USB и включите отладку заранее.';
}
