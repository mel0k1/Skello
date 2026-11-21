# Чипсеты

Skello работает с тремя платформами: MediaTek, Unisoc и Qualcomm. Samsung в планы не входит. Здесь собрано, как каждое семейство попадает в режим прошивки, по какому протоколу идёт обмен и что Skello берёт у существующих инструментов.

Правило «что берём»: код проектов с permissive-лицензиями (public domain, BSD, MIT, Apache) переносится с атрибуцией в NOTICE; у проектов под AGPL и без лицензии берём только идеи, форматы и порядок шагов, код пишем свой.

## MediaTek

Устройство попадает в BROM (boot ROM) - зажать Volume Up и Volume Down на выключенном телефоне при подключении USB, - либо грузится Preloader. Хост общается с BROM, загружает Download Agent (DA), и уже DA выполняет чтение, запись и стирание разделов.

Что дают референсы:

- penumbra (AGPL-3.0): прошивка и чтение разделов, протоколы DA двух поколений - V5 (XFlash) и V6 (XML), разблокировка загрузчика на уязвимых устройствах, операции с RPMB, произвольное чтение и запись памяти, прошивка по scatter-файлам
- mtkclient-rs (Apache-2.0): чистый Rust без рантайм-зависимостей, повторное использование DA-сессии и продолжение чтения или записи после обрыва, разблокировка через раздел seccfg с HACC-шифрованием (V3/V4), патч vbmeta для отключения AVB, таблица чипов в рантайме (sdata.json), автоматическая установка WinUSB-драйвера на Windows

План Skello: этап 1 - транспорт BROM и Preloader, загрузка DA V5, чтение GPT и разделов (реализовано в crates/skello-chipsets/src/mediatek: BROM-хендшейк и SEND_DA/JUMP_DA, загрузка стокового DA из файла загрузчика, XFlash-фрейминг и readflash/writeflash, seccfg-разблокировка без root); этап 2 - DA V6 (XML), разблокировка через seccfg на уязвимых чипах, эксплойты linecode (Kamakiri2) и Carbonara, RPMB (реализовано: seccfg V4 с аппаратным ключом - HACC-движок по кастомным регистрам DA V5 в sej.rs, режимы V3/V4-legacy в patch_v4; DA V6 - XML-сессия xml.rs; Kamakiri2 - linecode-эксплойт exploit.rs с дампом BootROM, bypass и инъекцией payload-бинов для десяти чипов, RPMB - readflash/writeflash с parttype 3; Carbonara - детект анти-патчей, поиск позиции хеша (md5/sha1/sha256), патчи DA2 и загрузка stage 2 через BOOT-TO - последовательность перенесена по референсу и ждёт проверки на железе; FORMAT (0x010003) - стирание раздела по имени из GPT со статусами CONTINUE/COMPLETE; scatter-файл - парсер и прошивка образов, boot-области пишутся в hardware boot1).

## Unisoc

BROM включается boot-кнопкой (у каждой модели своя) или boot-кабелем, устройство видно как USB 1782:4d00. Хост загружает FDL1 в SRAM по адресу из XML прошивки, FDL1 поднимает внешнюю память и грузит FDL2 в DRAM, дальше идут команды работы с разделами. В протоколе есть «transcode»: байты 0x7d и 0x7e экранируются байтом 0x7d, часть FDL отключает это командой. Разблокировка загрузчика делается из fastboot: Identifier Token подписывается RSA-ключом (для чипов вроде T618 ключ уже известен) и отправляется устройству.

Что дают референсы:

- spreadtrum_flash (public domain): загрузка FDL, keep_charge, disable_transcode, список разделов, чтение и запись разделов, переразбивка, выключение после работы; FDL-бинари берутся из прошивки конкретной модели (.pac), общий их набор невозможен
- unisoc-unlock (без лицензии): порядок шагов разблокировки через fastboot и подпись токена; тестировалось на T618

План Skello: этап 1 - обмен с BROM, загрузка FDL, запись разделов (реализовано в crates/skello-chipsets/src/unisoc: BSL-фрейминг с CRC16 и суммой, transcode-экранирование и его отключение, загрузка FDL1/FDL2 из прошивки модели, запись и чтение разделов по имени); этап 2 - чтение GPT, разблокировка подписью токена для чипов с известным ключом (таблица разделов реализована: READ_PARTITION/0x2D с ответом 0xBA, записи по 0x4C - имя UTF-16 и размер в секторах; разблокировка токеном реализована для T618-класса: identifier token из fastboot (нестандартные SN-пакеты), подпись RSA-4096 PKCS#1 v1.5 SHA-256 ключом сообщества, download + flashing unlock_bootloader; прошивка .pac реализована: заголовок 2124 байта и каталог с записями по 3080 (unpac, Unlicense), CRC16 0xA001 по заголовку и данным, FDL-пара из пакета, все flash-файлы по именам разделов через FDL2, перезагрузка).

## Qualcomm

Emergency Download Mode (EDL, «9008») поднимается самим boot ROM, устройство видно как USB 05C6:9008 или как COM-порт / ttyUSB. Хост по протоколу Sahara передаёт программатор (prog_firehose_ddr.elf, на новых платформах xbl_s_devprg_ns.melf), дальше Firehose XML-команды: чтение, запись, стирание по LBA, работа с GPT, применение rawprogram и patch XML, проверка SHA256. Для UFS важно указывать LUN - индекс физического раздела памяти.

Что дают референсы:

- qdlrs (BSD-3): Rust-реализация Sahara и Firehose, бекенды USB и serial, flasher с rawprogram, dump, write, erase, print-gpt, выбор LUN, обход отсутствия HELLO-пакета
- qedl-rs (MIT): модульный стек крейтов (ядро, транспорт, Sahara, Firehose, хранилище, образы, задачи), распаковка Android sparse-образов, проверка SHA256, автоматический переход DIAG в EDL, перекачка rawprogram.xml в свои команды

План Skello: этап 1 - Sahara с serial-транспортом и USB, загрузка программиста, этап 2 - Firehose XML и sparse-образы (реализовано в crates/skello-chipsets/src/qualcomm: Sahara по USB и serial - serial-линк 115200 8N1 через serial2, HELLO_RESP вслепую после паузы, если устройство молчит, Firehose configure/program/read/reboot, rawprogram-XML и развёртка sparse в raw; прошивка rawprogram по serial и USB доступна на экране «Чипсеты»). Реализовано после этапа 2: erase по LBA (`<erase first_lba last_lba PHYSICAL_PARTITION_NUM>`, имя раздела через GPT-список по read LBA 0..34), patch-XML (записи `<patch>` с filename=DISK - byte_offset, size_in_bytes, start_sector, value - host-side пропускаются, как в qdlrs), DA V6 - READ-FLASH/WRITE-FLASH/ERASE-FLASH/READ-PARTITION/WRITE-PARTITION/ERASE-PARTITION через UPLOAD-FILE/DOWNLOAD-FILE/PROGRESS-REPORT/FILE-SYS-OPERATION (xml.rs, по сессии mtkclient-rs), продолжение операций после обрыва - read_flash_resumable/write_flash_resumable с пересборкой сессии (erase только перед первым куском) и тестом на «умершем» линке.

## Уровень доступа на Linux

Без udev-правил доступ к USB есть только у root. Реализовано: `udev/40-skello.rules` идёт в архиве релиза (raw USB 0666 для 0e8d/05c6:9008/1782/18d1, tty-права для серийных инструментов, привязка qcserial к 9008 с guard через TEST). На Windows транспорт работает через nusb и требует WinUSB: `windows/skello_winusb.inf` (MTK 0003/2000/2001, EDL 9008, Unisoc 4D00, Google fastboot d00d и adb 4ee0) ставится через `windows/install_winusb.cmd` (pnputil с само-элевацией); после привязки COM-драйверы перестают занимать порты - Skello COM не использует.
