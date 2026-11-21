<img src="assets/logo.png" width="96" align="right" alt="">

# Skello

[![CI](https://github.com/mel0k1/Skello/actions/workflows/ci.yml/badge.svg)](https://github.com/mel0k1/Skello/actions/workflows/ci.yml)
[![Release](https://github.com/mel0k1/Skello/actions/workflows/release.yml/badge.svg)](https://github.com/mel0k1/Skello/actions/workflows/release.yml)

Skello - инструмент для прошивки Android-устройств и получения root-прав с графическим интерфейсом. Ядро на Rust общается с телефоном по протоколам adb, fastboot, BROM/DA (MediaTek), FDL (Unisoc) и Sahara/Firehose (Qualcomm) напрямую, интерфейс на Flutter собран на Material 3 Expressive.

Документация на английском: [README.en.md](README.en.md).

Проект в ранней стадии: протокольные крейты уже работают с реальными форматами сообщений, интерфейс и мастера прошивки в разработке.

## Зачем

adb и fastboot работают из терминала: нужно помнить команды, держать терминал открытым и на каждом шаге проверять себя вручную. Ошибка в имени раздела или прошивка чужого образа легко превращает телефон в кирпич. Skello ведёт по шагам: показывает состояние устройства, перед опасной операцией объясняет, что произойдёт, и проверяет условия (заряд, разблокированный загрузчик, размер образа) до того, как отправить команду.

## Что уже есть

- crates/skello-adb: протокол adb целиком на Rust: заголовок amessage на 24 байта, клиент adb-сервера на порту 5037 и USB-транспорт без сервера (интерфейс ff/42/01): CNXN/AUTH-рукопожатие с RSA-ключом в формате android pubkey, сервисы shell и reboot через A_OPEN, статусы device / unauthorized / offline
- crates/skello-fastboot: строки команд fastboot (getvar, download, flash, erase, set_active, reboot-*), разбор ответов OKAY / FAIL / INFO / TEXT / DATA, парсер max-download-size и USB-транспорт на nusb (перечисление устройств ff/42/03, getvar, reboot, unlock, erase, download с прогрессом)
- crates/skello-bootimg: разбор заголовка Android boot image (v0-v4): размеры ядра и ramdisk, размер страницы, версия Android и патч безопасности, наличие AVB-футера
- crates/skello-chipsets: формат раздела seccfg MediaTek (V3/V4) по mtkclient: чтение состояния блокировки, локальный патч unlock для разделов с программным ключом SEJ (AES-256-CBC), аппаратные ключи честно отклоняются до этапа DA
- app: мост Flutter и Rust через flutter_rust_bridge (крейт app/rust собирается cargokit-ом вместе с приложением), DeviceRepository зовёт ядро и при недоступности FFI переключается на системные adb и fastboot
- app: живой список устройств напрямую по USB (adb и fastboot) с карточками состояний, меню перезагрузки (system / bootloader / recovery) и запуском мастера разблокировки; тема Material 3 Expressive, i18n на русский и английский
- app: мастер разблокировки загрузчика: предупреждение о стирании с обязательным подтверждением, flashing unlock по USB (или через бинарник) с ожиданием подтверждения на телефоне и повторной проверкой
- app: Root-экран: выбор Magisk / KernelSU / APatch, выбор boot.img, проверка заголовка через Rust-ядро, прошивка boot / recovery / vendor_boot по USB с живым прогрессом и стирание разделов после проверок unlocked, battery-soc-ok и max-download-size; после прошивки - перезагрузка с ожиданием устройства в adb до 90 секунд
- app: Root-экран прошивает образы больше max-download-size: Android sparse разбивается по чанкам на серию download+flash, сырой образ без sparse-разметки честно отклоняется
- app: терминал adb shell по USB на карточке устройства и экран MediaTek seccfg: чтение состояния блокировки и разблокировка через root (dd + base64 через adb shell)
- app: вендорские транспорты этапа 1 (skello-chipsets): MediaTek BROM/DA V5 с разблокировкой seccfg без root, Unisoc FDL с transcode-экранированием и записью разделов, Qualcomm Sahara/Firehose с rawprogram XML; USB-порты чипсетов видны на уровне API
- app: экран «Чипсеты» для путей этапа 1: порты BROM / FDL / EDL с серийниками, разблокировка MediaTek через заводской Download Agent (без root), прошивка раздела Unisoc через FDL1/FDL2 с hex-адресами, Qualcomm EDL с программатором Firehose и разбором rawprogram XML; перед каждой операцией - подтверждение, всё пишется в журнал
- app: этап 2 чипсетов: SEJ-железо через DA V5 - HACC-движок по кастомным регистрам DA, seccfg V4 с аппаратным ключом разблокируется без root; DA V6 (XML) - сессия с BOOT-TO stage 2 и выходом в fastboot или meta; Qualcomm EDL по serial (115200, Sahara с обходом молчащего HELLO + Firehose); Unisoc - таблица разделов через FDL2; всё доступно на экране «Чипсеты»
- app: эксплойты и разблокировки: Kamakiri2 (linecode) - дамп BootROM и обход авторизации DA для уязвимых чипов (MT6755..MT6785, payload-бины mtkclient встроены), Carbonara - перезапись хеша DA1 и загрузка патченого stage 2; RPMB-блок читается и пишется через DA (parttype 3); Unisoc T618 - разблокировка загрузчика подписанным identifier token (RSA-4096, ключ сообщества); fastboot-транспорт принял нестандартные SN-ответы Unisoc
- app: прошивка прошивок целиком: Unisoc - парсер .pac (заголовок и каталог по unpac, CRC16 0xA001 header и данных, FDL-пара бутится из самого пакета, все flash-файлы пишутся по именам разделов, потом перезагрузка); MediaTek - FORMAT через DA V5 (стирание раздела по имени из GPT, RPMB отказом защищён), scatter-файл (платформа, адреса, boot-области) с прошивкой образов из каталога scatter
- app: журнал команд и ответов устройства в skello-commands.log (каталог поддержки приложения) - пишут и Dart, и Rust-ядро внутри транзакций
- CI на GitHub Actions: fmt, clippy, тесты Rust на Linux и Windows, analyze и тесты Flutter; релизные сборки Windows и Linux по тегу

Разбор протоколов и исходников AOSP - в [ARCHITECTURE.md](ARCHITECTURE.md), работа с чипсетами - в [CHIPSETS.md](CHIPSETS.md).

## План MVP

1. Список устройств из adb-сервера (host:devices) на главном экране
2. Карточка устройства: модель, версия Android, заряд, состояние загрузчика через getvar
3. Перезагрузка в bootloader, recovery и fastboot из меню устройства
4. Мастер разблокировки загрузчика с предупреждением о полном стирании данных; для MediaTek и Unisoc - свои пути из CHIPSETS.md
5. Прошивка образов через fastboot; Qualcomm - через EDL (Sahara + Firehose)
6. Root на выбор: Magisk, KernelSU или APatch - патч boot.img на стороне компьютера и прошивка патченного образа

Порядок может поменяться по ходу дела.

## Безопасность

Разблокировка загрузчика стирает всё хранилище устройства. Прошивка неподходящего образа или сбой на середине пути могут оставить телефон неработающим. Skello показывает, что произойдёт перед каждым опасным шагом, но ответственность за операцию остаётся на том, кто нажал кнопку. Проверяйте, что образ собран именно для вашей модели.

## Разработка

Нужны Rust stable и Flutter stable (проверено на Flutter 3.50).

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

cd app
flutter pub get
flutter analyze
flutter test
```

Правила для кода и коммитов - в [CONTRIBUTING.md](CONTRIBUTING.md).

## Лицензия

Двойная: [MIT](LICENSE-MIT) или [Apache-2.0](LICENSE-APACHE), на выбор.
