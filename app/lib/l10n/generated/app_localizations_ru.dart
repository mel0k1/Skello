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
      '1. Сначала разблокируйте загрузчик — в Skello для этого есть мастер.\n2. Скачивайте Magisk только из официального репозитория.\n3. Перенесите проверенный boot.img на телефон.\n4. В приложении Magisk: Установка → Выбрать и пропатчить файл → укажите boot.img.\n5. Верните полученный magisk_patched-XXXXX.img на ПК.\n6. Skello прошьёт его командой fastboot flash boot.';

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

  @override
  String get actionsTooltip => 'Действия';

  @override
  String get rebootMenuSystem => 'Перезагрузить';

  @override
  String get rebootMenuBootloader => 'Перезагрузить в bootloader';

  @override
  String get rebootMenuRecovery => 'Перезагрузить в recovery';

  @override
  String get unlockMenu => 'Разблокировать загрузчик…';

  @override
  String get actionStarted => 'Команда отправлена';

  @override
  String get actionFailed => 'Команда не выполнена';

  @override
  String get unlockTitle => 'Разблокировка загрузчика';

  @override
  String get unlockWarningTitle => 'Это сотрёт всё';

  @override
  String get unlockWarningBody =>
      'Разблокировка стирает всё хранилище без возможности восстановления, лишает гарантии и на части вендоров включает анти-откат прошивки. Действуйте, только если понимаете риски.';

  @override
  String get unlockDevice => 'Устройство';

  @override
  String get unlockVendorNote =>
      'Стандартный путь работает на Pixel и большинстве Android One. Xiaomi требует утилиту Mi Unlock, у MediaTek и Unisoc свои пути — см. заметки по чипсетам. Мастер выполняет только стандартный путь.';

  @override
  String get unlockChecking => 'Проверяю состояние загрузчика…';

  @override
  String get unlockAlready => 'Загрузчик уже разблокирован';

  @override
  String get unlockLocked => 'Загрузчик заблокирован';

  @override
  String get unlockUnknown =>
      'Не удалось узнать состояние загрузчика. Проверьте, что fastboot работает, а устройство находится в режиме fastboot.';

  @override
  String get unlockSteps =>
      '1. Skello запустит fastboot flashing unlock.\n2. Телефон покажет экран подтверждения — выберите там Unlock.\n3. Хранилище будет стёрто, телефон перезагрузится.';

  @override
  String get unlockConfirm => 'Я понимаю, что все данные будут стёрты';

  @override
  String get unlockRun => 'Разблокировать';

  @override
  String get unlockWaiting => 'Подтвердите на экране телефона…';

  @override
  String get unlockFailed => 'Разблокировка не завершилась';

  @override
  String get flashSectionTitle => 'Прошивка';

  @override
  String get flashDeviceLabel => 'Устройство в fastboot';

  @override
  String get flashNoDevice =>
      'Нет устройства в режиме fastboot. Сначала перезагрузите его в bootloader.';

  @override
  String get flashPickFirst => 'Сначала выберите и проверьте boot-образ.';

  @override
  String get flashLocked => 'Загрузчик заблокирован — сначала разблокируйте';

  @override
  String get flashUnlocked => 'Загрузчик разблокирован';

  @override
  String get flashStateUnknown => 'Состояние загрузчика неизвестно';

  @override
  String get flashPartitionLabel => 'Раздел';

  @override
  String flashPartitionButton(String partition) {
    return 'Прошить $partition';
  }

  @override
  String get flashConfirmTitle => 'Прошить раздел?';

  @override
  String flashConfirmBody(String partition, String serial) {
    return 'Skello выполнит fastboot flash $partition на $serial. Прерывание прошивки может окирпичить устройство.';
  }

  @override
  String get flashRunning => 'Прошиваю… не отключайте устройство';

  @override
  String get flashPhaseDownload => 'Отправка образа…';

  @override
  String get flashPhaseFlash => 'Запись раздела…';

  @override
  String get flashBatteryError =>
      'Устройство сообщает, что батарея не готова к прошивке. Подключите зарядку и дождитесь заряда.';

  @override
  String flashSizeError(String max, String image) {
    return 'Образ больше, чем разрешает устройство: максимум $max Б, образ $image Б.';
  }

  @override
  String get flashDone => 'образ прошит';

  @override
  String get flashOutput => 'вывод fastboot';

  @override
  String get flashFailed => 'Прошивка не удалась';

  @override
  String erasePartitionButton(String partition) {
    return 'Стереть $partition';
  }

  @override
  String get eraseConfirmTitle => 'Стереть раздел?';

  @override
  String eraseConfirmBody(String partition, String serial) {
    return 'Skello выполнит fastboot erase $partition на $serial. Все данные раздела будут уничтожены.';
  }

  @override
  String get eraseDone => 'раздел стёрт';

  @override
  String get rebootCheckButton => 'Перезагрузить и дождаться устройства';

  @override
  String get rebootWaiting => 'Перезагрузка… жду, когда устройство вернётся';

  @override
  String get rebootBack => 'Устройство вернулось в систему';

  @override
  String get rebootTimeout =>
      'Устройство не вернулось за отведённое время — проверьте вручную';

  @override
  String get shellMenu => 'Терминал';

  @override
  String get shellTitle => 'Терминал';

  @override
  String get shellHint => 'команда adb shell';

  @override
  String get shellSend => 'Выполнить';

  @override
  String get shellFailed => 'Команда не удалась';

  @override
  String get mediatekMenu => 'MediaTek seccfg';

  @override
  String get seccfgTitle => 'seccfg';

  @override
  String get seccfgStateLabel => 'Состояние блокировки';

  @override
  String get seccfgVersionLabel => 'Формат';

  @override
  String get seccfgDetailLabel => 'Детали';

  @override
  String get seccfgUnlockButton => 'Разблокировать (seccfg)';

  @override
  String get seccfgUnlockConfirmTitle => 'Пропатчить seccfg?';

  @override
  String seccfgUnlockConfirmBody(String serial) {
    return 'Skello прочитает seccfg на $serial, пропатчит флаг блокировки на unlocked и запишет обратно. Нужен root. Ошибка записи может испортить загрузку.';
  }

  @override
  String get seccfgUnlockDone => 'seccfg пропатчен на unlocked';

  @override
  String get seccfgRootHint =>
      'Для чтения блочного устройства seccfg нужен root.';

  @override
  String get seccfgNeedsSejHint =>
      'Локально патчатся только разделы V3 с программным ключом SEJ; разделы с аппаратным ключом требуют этап Download Agent, пока не реализован.';

  @override
  String get seccfgFailed => 'Не удалось';

  @override
  String get chipsetsTitle => 'Чипсеты';

  @override
  String get chipsetsTooltip => 'Вендорские режимы';

  @override
  String get chipsetsLoadFailed => 'Не удалось получить список USB-портов';

  @override
  String get chipsetsPortsNone => 'Порт не обнаружен';

  @override
  String get chipsetsPickType => 'Файлы прошивки';

  @override
  String get chipsetsConfirmGo => 'Запустить';

  @override
  String get chipsetsAddrInvalid =>
      'Адреса должны быть шестнадцатеричными числами, например 0x00090000';

  @override
  String get chipsetsMtkTitle => 'MediaTek: BROM + Download Agent';

  @override
  String get chipsetsMtkHint =>
      'Выключите телефон, зажмите BROM или дайте батарее сесть; порт BROM/Preloader появляется при подключении кабеля.';

  @override
  String get chipsetsMtkLoaderNone =>
      'Заводской загрузчик Download Agent не выбран';

  @override
  String get chipsetsMtkLoaderPick => 'Выбрать загрузчик DA';

  @override
  String get chipsetsMtkUnlock => 'Разблокировать загрузчик (без root)';

  @override
  String get chipsetsMtkUnlockConfirmTitle =>
      'Разблокировать через Download Agent?';

  @override
  String get chipsetsMtkUnlockConfirmBody =>
      'Skello поднимет заводской загрузчик по BROM, прочитает seccfg, перепишет флаг блокировки на unlocked и запишет раздел обратно. Root не нужен, но нужен файл загрузчика вашей модели. Ошибка записи может сломать загрузку.';

  @override
  String get chipsetsMtkDone => 'seccfg пропатчен через Download Agent';

  @override
  String get chipsetsMtkV6Boot => 'Загрузить DA V6 (XML)';

  @override
  String get chipsetsMtkV6Fastboot => 'Fastboot';

  @override
  String get chipsetsMtkV6Meta => 'Meta';

  @override
  String get chipsetsMtkV6Done => 'DA V6 загружен';

  @override
  String get chipsetsUnisocTitle => 'Unisoc: FDL';

  @override
  String get chipsetsUnisocHint =>
      'Выберите FDL1 и FDL2 из заводской прошивки вашей модели; адреса ниже - обычные значения по умолчанию.';

  @override
  String get chipsetsUnisocPickFdl1 => 'Выбрать FDL1';

  @override
  String get chipsetsUnisocPickFdl2 => 'Выбрать FDL2';

  @override
  String get chipsetsUnisocAddr1Label => 'Адрес FDL1';

  @override
  String get chipsetsUnisocAddr2Label => 'Адрес FDL2';

  @override
  String get chipsetsUnisocAddrHint =>
      'В шестнадцатеричном виде, например 0x00090000';

  @override
  String get chipsetsUnisocPartitionLabel => 'Имя раздела';

  @override
  String get chipsetsUnisocImageNone => 'Образ не выбран';

  @override
  String get chipsetsUnisocImagePick => 'Выбрать образ';

  @override
  String get chipsetsUnisocFlash => 'Прошить раздел';

  @override
  String get chipsetsUnisocConfirmTitle => 'Прошить раздел по FDL?';

  @override
  String chipsetsUnisocConfirmBody(String image, String partition) {
    return 'Skello поднимет заводские загрузчики и запишет $image в $partition. Неверные адреса или чужой образ могут сломать загрузку.';
  }

  @override
  String get chipsetsUnisocDone => 'Раздел записан через FDL2';

  @override
  String get chipsetsUnisocTable => 'Прочитать таблицу разделов';

  @override
  String get chipsetsUnisocTableDone => 'Таблица разделов';

  @override
  String chipsetsUnisocSectors(int count) {
    return 'Секторов: $count';
  }

  @override
  String get chipsetsQualcommTitle => 'Qualcomm: EDL (Sahara + Firehose)';

  @override
  String get chipsetsQualcommHint =>
      'Порт EDL появляется при подключении телефона в dead mode; программатор и rawprogram XML берутся из пакета прошивки вашей модели.';

  @override
  String get chipsetsQualcommProgrammerNone =>
      'Программатор Firehose не выбран';

  @override
  String get chipsetsQualcommProgrammerPick => 'Выбрать программатор';

  @override
  String get chipsetsQualcommXmlPick => 'Выбрать rawprogram XML';

  @override
  String get chipsetsQualcommParse => 'Разобрать XML';

  @override
  String chipsetsQualcommEntriesLabel(int count) {
    return 'Записей в XML: $count';
  }

  @override
  String chipsetsQualcommEntryLine(
      String label, String file, String sector, String sectors) {
    return '$label: $file @ сектор $sector ($sectors)';
  }

  @override
  String get chipsetsQualcommFlash => 'Прошить все записи';

  @override
  String get chipsetsQualcommConfirmTitle => 'Прошивать по EDL?';

  @override
  String get chipsetsQualcommConfirmBody =>
      'Skello подаст программатор по Sahara и запишет каждую запись rawprogram. Разделы перезаписываются без подтверждения на телефоне.';

  @override
  String get chipsetsQualcommDone => 'rawprogram завершён';

  @override
  String get chipsetsQualcommSerial => 'EDL по serial (UART)';

  @override
  String get chipsetsQualcommSerialLoad => 'Найти serial-порты';

  @override
  String get chipsetsQualcommSerialNone => 'Serial-порты не найдены';

  @override
  String get chipsetsQualcommSerialFlash => 'Прошить по serial';

  @override
  String get chipsetsQualcommSerialDone => 'rawprogram завершён по serial';

  @override
  String get chipsetsExploitTitle => 'Эксплойты Kamakiri2';

  @override
  String get chipsetsExploitHint =>
      'Только уязвимые чипы (MT6755..MT6785): дамп BootROM, обход авторизации DA и Carbonara с патченым stage 2.';

  @override
  String get chipsetsExploitDump => 'Сдампить BootROM';

  @override
  String get chipsetsExploitKamakiri => 'Kamakiri2 разблокировка';

  @override
  String get chipsetsExploitCarbonara => 'Carbonara разблокировка';

  @override
  String get chipsetsExploitDone => 'Эксплойт отработал';

  @override
  String get chipsetsExploitConfirmTitle => 'Запустить эксплойт?';

  @override
  String get chipsetsExploitConfirmBody =>
      'Эксплойт работает с BootROM в режиме BROM. Продолжайте только при устройстве в BROM.';

  @override
  String get chipsetsRpmbRead => 'Прочитать RPMB';

  @override
  String get chipsetsRpmbDone => 'RPMB прочитан';

  @override
  String get chipsetsUnisocUnlock => 'Разблокировать загрузчик (T618)';

  @override
  String get chipsetsUnisocUnlockHint =>
      'Подписывает identifier token известным ключом T618 и запускает flashing unlock_bootloader.';

  @override
  String get chipsetsUnisocUnlockDone => 'Разблокировка завершена';

  @override
  String get chipsetsScatterTitle => 'Прошивка по scatter';

  @override
  String get chipsetsScatterHint =>
      'Прошиваются образы из scatter-файла; записи boot-области пишутся в hardware boot1.';

  @override
  String get chipsetsScatterFlash => 'Прошить по scatter';

  @override
  String get chipsetsScatterDone => 'Scatter-прошивка завершена';

  @override
  String get chipsetsEraseHint =>
      'Стереть раздел по имени из GPT: одна команда FORMAT на всю область. RPMB стирать нельзя.';

  @override
  String get chipsetsEraseName => 'Имя раздела';

  @override
  String get chipsetsEraseBtn => 'Стереть раздел';

  @override
  String get chipsetsEraseDone => 'Раздел стёрт';

  @override
  String get chipsetsPacTitle => 'Полная прошивка (.pac)';

  @override
  String get chipsetsPacHint =>
      'Пакет несёт свою пару FDL: устройство бутится, пишутся все файлы прошивки, потом перезагрузка. Сначала проверяются CRC.';

  @override
  String get chipsetsPacPick => 'Выбрать прошивку .pac';

  @override
  String get chipsetsPacFlash => 'Прошить прошивку';

  @override
  String get chipsetsPacDone => 'Прошивка .pac записана';

  @override
  String chipsetsPacConfirm(String info) {
    return 'Прошить эту прошивку? $info';
  }

  @override
  String get cancelLabel => 'Отмена';
}
