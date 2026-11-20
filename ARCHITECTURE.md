# Архитектура Skello

Документ собирает то, что выписано из исходников AOSP и референсных проектов, и план того, как Skello собирает это в один инструмент. Работа с чипсетами MediaTek, Unisoc и Qualcomm разобрана отдельно в [CHIPSETS.md](CHIPSETS.md).

## Слои

- crates/skello-adb и crates/skello-fastboot: протокольные крейты, без привязки к GUI
- app: Flutter-приложение, Material 3 Expressive (светлая и тёмная темы из одного seed-цвета), i18n через ARB-файлы в lib/l10n
- Связь слоёв: flutter_rust_bridge - крейты собираются в библиотеку и живут в одном процессе с Flutter, типы генерируются в Dart. На первом этапе поверх этого работает фолбэк: системные adb и fastboot вызываются через std::process (cmd/PowerShell на Windows, bash на Linux), чтобы мастера взлетели раньше собственного USB-транспорта
- Собственный USB-транспорт: крейт nusb (libusb на Linux, WinUSB на Windows), поверх него - реализации транспортов из этого документа и из CHIPSETS.md

## Как adb устроен (по исходникам packages/modules/adb)

- Клиент-сервер: adb-клиенты не ходят в устройство напрямую, они подключаются к adb-серверу на 127.0.0.1:5037. Формат «smart socket»: длина сообщения в 4 шестнадцатеричных цифрах, потом тело; сервер отвечает строкой OKAY или FAIL, затем снова длина и тело (smart_socket_enqueue в sockets.cpp)
- Сервер-устройство (adbd): бинарный протокол. Каждый пакет: 24-байтовый заголовок amessage (command, arg0, arg1, data_length, data_check, magic = command ^ 0xffffffff) и полезная нагрузка до 1 МБ (MAX_PAYLOAD в adb.h). Наш крейт воспроизводит этот заголовок байт в байт
- Команды: A_CNXN (рукопожатие, версия протокола и max payload), A_OPEN / A_OKAY / A_WRTE / A_CLSE (стримы-сокеты), A_AUTH (авторизация), A_STLS (переход в TLS для беспроводной отладки)
- AUTH: устройство присылает 20-байтовый токен (TOKEN_SIZE), хост подписывает его приватным RSA-ключом; отпечаток ключа пользователь подтверждает на экране телефона
- Версия протокола 0x01000001 и новее разрешает не считать контрольную сумму полезной нагрузки; Skello всё равно её считает, чтобы дружить со старыми adbd
- Файлы (push/pull) идут отдельным SYNC-протоколом внутри сокета: ID_SEND / ID_RECV / ID_STAT / ID_LIST / ID_DATA / ID_DONE, блоки данных до 64 КБ (SYNC_DATA_MAX), во второй версии протокола добавились сжатие (Brotli, LZ4, Zstd) и полная статистика файлов (file_sync_protocol.h)
- USB: интерфейс с class 0xFF, subclass 0x42, protocol 0x01 - это adb (ADB_CLASS / ADB_SUBCLASS / ADB_PROTOCOL в adb.h)
- Состояния транспорта: device, bootloader, recovery, sideload, rescue (ConnectionState в adb.h) - по ним интерфейс поймёт, в каком режиме телефон

## Как fastboot устроен (по исходникам system/core/fastboot)

- USB: тот же class 0xFF и subclass 0x42, но protocol 0x03 - по этой тройке хост отличает загрузчик от adbd (match_fastboot_with_serial в fastboot.cpp)
- Команда - ASCII-строка: getvar:version, download:00010000, flash:boot, erase:userdata, set_active:a, reboot-bootloader, oem ..., fetch:boot, snapshot-update:..., flashing:unlock / flashing:lock (в constants.h и fastboot_driver.cpp)
- Ответ - строка до 256 байт с 4-символьным префиксом: OKAY (успех), FAIL (ошибка с сообщением), INFO (строка прогресса, их бывает много подряд), TEXT (диагностика), DATA (шестнадцатеричный размер, после него устройство принимает сырые байты)
- Полезную нагрузку драйвер пишет в транспорт кусками по 1024 байта (TRANSPORT_CHUNK_SIZE), таймаут ответа - 30 секунд (RESP_TIMEOUT)
- Переменные устройства через getvar: unlocked, secure, current-slot, slot-count, has-slot, max-download-size, battery-soc-ok, battery-voltage, partition-size:<раздел>, partition-type:<раздел>. На них строятся все проверки безопасности перед прошивкой
- A/B-устройства: два слота разделов, set_active переключает, snapshot-update отменяет виртуальные A/B-обновления

## Сценарии Skello

### Разблокировка загрузчика

1. Перезагрузка в bootloader: через adb (reboot bootloader) или кнопками
2. getvar:unlocked - если уже unlocked, шаг пропускается
3. Стандартный путь - flashing:unlock с подтверждением на экране телефона
4. MediaTek и Unisoc имеют свои пути (seccfg с HACC, подпись Identifier Token) - в [CHIPSETS.md](CHIPSETS.md)
5. Предупреждение в интерфейсе: хранилище стирается полностью, без вариантов

### Прошивка образа

1. Проверки перед отправкой: battery-soc-ok даёт "yes", загрузчик unlocked, размер образа не больше max-download-size
2. download:<размер в hex>, затем образ кусками по 1024 байта
3. flash:<раздел>; между шагами собираем INFO-строки и показываем прогресс
4. Перезагрузка и проверка, что устройство вернулось

### Root

Все три метода доступны на выбор в интерфейсе, общий каркас один: на стороне компьютера патчится boot.img, патченный образ прошивается через fastboot (или через DA/BROM на MediaTek и Unisoc с заблокированным загрузчиком).

- Magisk: в ramdisk вставляется Magisk init, настройка через приложение Magisk
- KernelSU: патч ядра или GKI-образ, управление через приложение
- APatch: патч boot.img с ядром, управление через приложение

## Правила безопасности, которые строим в код

- Никакой команды прошивки без явного подтверждения и показа целевого раздела
- Проверка заряда и состояния загрузчика до download/flash
- Отказ шлить образ больше max-download-size без разбиения на части
- Лог каждой команды и ответа устройства: то, что случилось с телефоном, должно быть восстановимо по логу
