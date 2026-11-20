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
      'Сейчас устройств не видно. Подключите телефон по USB, включите отладку по USB, подтвердите доверие компьютеру на экране телефона — или перезагрузите его в fastboot. Затем обновите список.';

  @override
  String get refreshTooltip => 'Обновить';

  @override
  String get loadingDevices => 'Проверяю устройства…';

  @override
  String get sourcesUnavailableTitle => 'Источники устройств недоступны';

  @override
  String get sourcesUnavailableBody =>
      'Ни ядро Skello, ни системные adb и fastboot не ответили. Установите Android platform-tools, проверьте, что adb есть в PATH, и обновите список.';

  @override
  String get retryLabel => 'Повторить';

  @override
  String get rootTooltip => 'Root-инструменты';

  @override
  String get rootTitle => 'Root';

  @override
  String get rootIntro =>
      'Выберите root-метод для своего устройства, укажите boot-образ именно своей прошивки и проверьте его, прежде чем что-то тронет телефон.';

  @override
  String get methodMagiskSubtitle =>
      'Пропатчит boot-образ через magiskinit. Официальный путь — патч на телефоне в приложении Magisk.';

  @override
  String get methodKernelSuSubtitle =>
      'Root через ядро или подгружаемый модуль. Нужны GKI 5.10+ либо модуль, собранный под точное ядро прошивки.';

  @override
  String get methodApatchSubtitle =>
      'Патч ядра через kpatch. Нужна сборка kpatch под версию ядра вашей прошивки.';

  @override
  String get bootSectionTitle => 'Boot-образ';

  @override
  String get pickBootImage => 'Выбрать boot.img';

  @override
  String get bootImageTypeLabel => 'Android boot-образ';

  @override
  String get validBootImage => 'Корректный Android boot-образ';

  @override
  String get invalidBootImage =>
      'Этот файл не читается как Android boot-образ.';

  @override
  String get infoHeaderVersion => 'Версия заголовка';

  @override
  String get infoPageSize => 'Размер страницы';

  @override
  String get infoKernelSize => 'Размер ядра';

  @override
  String get infoRamdiskSize => 'Размер ramdisk';

  @override
  String get infoOsVersion => 'Версия Android';

  @override
  String get infoOsPatchLevel => 'Патч безопасности';

  @override
  String get infoFileSize => 'Размер файла';

  @override
  String get infoAvbFooter => 'AVB-футер';

  @override
  String get avbPresent => 'есть';

  @override
  String get avbAbsent => 'нет';

  @override
  String get checkErrorTitle => 'Проверка не удалась';

  @override
  String get magiskGuide =>
      '1. Сначала разблокируйте загрузчик. Мастер разблокировки — следующий шаг плана MVP.\n2. Скачивайте Magisk только из официального репозитория.\n3. Перенесите проверенный boot.img на телефон.\n4. В приложении Magisk: Установка → Выбрать и пропатчить файл → укажите boot.img.\n5. Верните полученный magisk_patched-XXXXX.img на ПК.\n6. Skello прошьёт его командой fastboot flash boot.';

  @override
  String get kernelsuGuide =>
      '1. Убедитесь, что ядро поддерживает GKI 5.10 или новее, либо найдите LKM-модуль, собранный точно под ваше ядро.\n2. KernelSU патчит boot.img в официальном приложении на телефоне; модули под конкретное ядро берутся из официальных сборок KernelSU.\n3. После патча Skello прошьёт образ командой fastboot flash boot.';

  @override
  String get apatchGuide =>
      '1. Возьмите kpatch строго под версию ядра вашей прошивки.\n2. APatch патчит boot.img в официальном приложении на телефоне.\n3. После патча Skello прошьёт образ командой fastboot flash boot.';

  @override
  String get warningTitle => 'Перед прошивкой';

  @override
  String get warningBody =>
      'Прошивка неподходящего образа или неудачный патч могут окирпичить телефон. Пропатченный образ требует разблокированного загрузчика, а разблокировка стирает всё хранилище. Сверяйте образ со своей моделью.';
}
