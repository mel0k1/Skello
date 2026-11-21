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
- app: мост Flutter и Rust через flutter_rust_bridge (крейт app/rust собирается cargokit-ом вместе с приложением), DeviceRepository зовёт ядро и при недоступности FFI переключается на системные adb и fastboot
- app: живой список устройств напрямую по USB (adb и fastboot) с карточками состояний, меню перезагрузки (system / bootloader / recovery) и запуском мастера разблокировки; тема Material 3 Expressive, i18n на русский и английский
- app: мастер разблокировки загрузчика: предупреждение о стирании с обязательным подтверждением, flashing unlock по USB (или через бинарник) с ожиданием подтверждения на телефоне и повторной проверкой
- app: Root-экран: выбор Magisk / KernelSU / APatch, выбор boot.img, проверка заголовка через Rust-ядро, прошивка boot / recovery / vendor_boot по USB с живым прогрессом и стирание разделов после проверок unlocked, battery-soc-ok и max-download-size; после прошивки - перезагрузка с ожиданием устройства в adb до 90 секунд
- app: журнал команд и ответов устройства в skello-commands.log (каталог поддержки приложения)
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
