import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:skello/l10n/generated/app_localizations.dart';
import 'package:skello/services/device_output.dart';
import 'package:skello/ui/chipsets_screen.dart';
import 'package:skello/ui/home_screen.dart';

import 'fake_repository.dart';

class FakeSelector extends FileSelectorPlatform {
  FakeSelector(this.path);

  final String path;


  @override
  Future<FileSaveLocation?> getSaveLocation({
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    return FileSaveLocation(path);
  }

  @override
  Future<XFile?> openFile({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    return XFile(path);
  }
}

Widget _wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: child,
  );
}

void main() {
  Future<void> pumpChipsets(WidgetTester tester, FakeRepository repository) async {
    tester.view.physicalSize = const Size(1200, 4000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(_wrap(ChipsetsScreen(repository: repository)));
    await tester.pumpAndSettle();
  }

  testWidgets('chipsets screen lists the ports per family', (tester) async {
    final repository = FakeRepository();
    repository.mtkPorts = ['BROM_123'];
    repository.qualcommPorts = ['9008'];
    await pumpChipsets(tester, repository);

    expect(find.text('MediaTek: BROM + Download Agent'), findsOneWidget);
    expect(find.text('Unisoc: FDL'), findsOneWidget);
    expect(find.text('Qualcomm: EDL (Sahara + Firehose)'), findsOneWidget);
    expect(find.text('BROM_123'), findsOneWidget);
    expect(find.text('9008'), findsOneWidget);
    expect(find.text('No port detected'), findsOneWidget);
    expect(repository.calls, contains('chipset-ports'));
  });

  testWidgets('mtk unlock runs after the loader pick and confirmation', (
    tester,
  ) async {
    FileSelectorPlatform.instance = FakeSelector('/fw/DA.bin');
    final repository = FakeRepository();
    repository.mtkPorts = ['BROM_123'];
    await pumpChipsets(tester, repository);

    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose DA loader'),
    );
    await tester.pumpAndSettle();
    expect(find.text('DA.bin'), findsOneWidget);

    await tester.tap(
      find.widgetWithText(FilledButton, 'Unlock bootloader (no root)'),
    );
    await tester.pumpAndSettle();
    expect(find.text('Unlock via Download Agent?'), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, 'Run it'));
    await tester.pumpAndSettle();

    expect(repository.calls, contains('mtk-da-unlock /fw/DA.bin'));
    expect(find.text('seccfg patched over the Download Agent'), findsOneWidget);
    expect(find.text('seccfg patched over the DA'), findsOneWidget);
  });

  testWidgets('unisoc flash rejects non-hex addresses', (tester) async {
    FileSelectorPlatform.instance = FakeSelector('/fw/fdl.bin');
    final repository = FakeRepository();
    repository.unisocPorts = ['SPD'];
    await pumpChipsets(tester, repository);

    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose FDL1'),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose FDL2'),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose image'),
    );
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField).at(2), 'boot');
    await tester.enterText(find.byType(TextField).at(0), 'zz');
    await tester.pumpAndSettle();

    await tester.tap(find.widgetWithText(FilledButton, 'Flash partition'));
    await tester.pumpAndSettle();

    expect(
      find.text('Addresses must be hex numbers, for example 0x00090000'),
      findsOneWidget,
    );
    expect(
      repository.calls.where((c) => c.startsWith('unisoc-flash')),
      isEmpty,
    );
  });

  testWidgets('unisoc flash runs with valid addresses', (tester) async {
    FileSelectorPlatform.instance = FakeSelector('/fw/fdl.bin');
    final repository = FakeRepository();
    repository.unisocPorts = ['SPD'];
    await pumpChipsets(tester, repository);

    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose FDL1'),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose FDL2'),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose image'),
    );
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField).at(2), 'boot');
    await tester.pumpAndSettle();

    await tester.tap(find.widgetWithText(FilledButton, 'Flash partition'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Run it'));
    await tester.pumpAndSettle();

    expect(
      repository.calls.where((c) => c.startsWith('unisoc-flash')),
      ['unisoc-flash boot 589824 1342177280'],
    );
    expect(find.text('Partition written over FDL2'), findsOneWidget);
  });

  testWidgets('qualcomm parse lists entries and flash runs over edl', (
    tester,
  ) async {
    FileSelectorPlatform.instance = FakeSelector('/fw/rawprogram.xml');
    final repository = FakeRepository();
    repository.qualcommPorts = ['9008'];
    repository.parsedEntries = [
      ChipsetProgramEntry(
        label: 'boot',
        startSector: '1024',
        numSectors: BigInt.from(100),
        physPart: 0,
        filePath: '/fw/boot.img',
      ),
    ];
    await pumpChipsets(tester, repository);

    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose programmer'),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose rawprogram XML'),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(OutlinedButton, 'Parse XML'));
    await tester.pumpAndSettle();

    expect(find.text('1 entries in the XML'), findsOneWidget);
    expect(find.text('boot: boot.img @ sector 1024 (100)'), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, 'Flash all entries'));
    await tester.pumpAndSettle();
    expect(find.text('Flash over EDL?'), findsOneWidget);
    await tester.tap(find.widgetWithText(FilledButton, 'Run it'));
    await tester.pumpAndSettle();

    expect(
      repository.calls.where((c) => c.startsWith('edl-flash')),
      ['edl-flash 1 via /fw/rawprogram.xml'],
    );
    expect(find.text('rawprogram finished'), findsOneWidget);
  });

  testWidgets('home screen opens the chipsets screen', (tester) async {
    tester.view.physicalSize = const Size(1200, 2200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final repository = FakeRepository();
    repository.mtkPorts = ['BROM_123'];
    await tester.pumpWidget(_wrap(HomeScreen(repository: repository)));
    await tester.pumpAndSettle();

    await tester.tap(find.byTooltip('Vendor modes'));
    await tester.pumpAndSettle();

    expect(find.text('MediaTek: BROM + Download Agent'), findsOneWidget);
    expect(repository.calls.where((c) => c == 'chipset-ports'), hasLength(1));
  });

  testWidgets('mtk v6 boot runs for fastboot and meta', (tester) async {
    FileSelectorPlatform.instance = FakeSelector('/fw/DA.bin');
    final repository = FakeRepository();
    repository.mtkPorts = ['BROM_123'];
    await pumpChipsets(tester, repository);

    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose DA loader'),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(OutlinedButton, 'Fastboot'));
    await tester.pumpAndSettle();

    expect(
      repository.calls.where((c) => c.startsWith('v6-boot')),
      ['v6-boot fastboot /fw/DA.bin'],
    );
    expect(find.text('DA V6 booted'), findsOneWidget);
    expect(find.text('DA V6 booted, device set to fastboot'), findsOneWidget);
  });

  testWidgets('unisoc partition table shows the rows', (tester) async {
    FileSelectorPlatform.instance = FakeSelector('/fw/fdl.bin');
    final repository = FakeRepository();
    repository.unisocPorts = ['SPD'];
    repository.unisocTable = [
      UnisocPartitionRow('prod', BigInt.from(4194304)),
      UnisocPartitionRow('misc', BigInt.from(2048)),
    ];
    await pumpChipsets(tester, repository);

    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose FDL1'),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose FDL2'),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(OutlinedButton, 'Read partition table'));
    await tester.pumpAndSettle();

    expect(
      repository.calls.where((c) => c.startsWith('unisoc-table')),
      ['unisoc-table 589824 1342177280'],
    );
    expect(find.text('Partition table'), findsOneWidget);
    expect(find.text('prod - 4194304 sectors'), findsOneWidget);
    expect(find.text('misc - 2048 sectors'), findsOneWidget);
  });

  testWidgets('edl serial flash runs over the picked port', (tester) async {
    FileSelectorPlatform.instance = FakeSelector('/fw/rawprogram.xml');
    final repository = FakeRepository();
    repository.qualcommPorts = ['9008'];
    repository.serialPorts = ['/dev/ttyUSB0'];
    repository.parsedEntries = [
      ChipsetProgramEntry(
        label: 'boot',
        startSector: '1024',
        numSectors: BigInt.from(100),
        physPart: 0,
        filePath: '/fw/boot.img',
      ),
    ];
    await pumpChipsets(tester, repository);

    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose programmer'),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(OutlinedButton, 'Choose rawprogram XML'),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(OutlinedButton, 'Parse XML'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(OutlinedButton, 'Scan serial ports'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(ChoiceChip, '/dev/ttyUSB0'));
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, 'Flash over serial'),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Run it'));
    await tester.pumpAndSettle();

    expect(
      repository.calls.where((c) => c.startsWith('edl-serial-flash')),
      ['edl-serial-flash 1 on /dev/ttyUSB0'],
    );
    expect(find.text('rawprogram finished over serial'), findsOneWidget);
    expect(find.text('flash OKAY over serial'), findsOneWidget);
  });

  testWidgets('brom dump saves through the save dialog', (tester) async {
    FileSelectorPlatform.instance = FakeSelector('/fw/DA.bin');
    final repository = FakeRepository();
    repository.mtkPorts = ['BROM_123'];
    await pumpChipsets(tester, repository);

    await tester.tap(find.widgetWithText(OutlinedButton, 'Dump BootROM'));
    await tester.pumpAndSettle();

    expect(
      repository.calls.where((c) => c.startsWith('exploit-dump')),
      ['exploit-dump /fw/DA.bin'],
    );
    expect(find.text('Exploit flow finished'), findsOneWidget);
    expect(find.text('bootrom dumped (MT6768, 131072 bytes)'), findsOneWidget);
  });

  testWidgets('kamakiri2 unlock runs after the confirmation', (tester) async {
    FileSelectorPlatform.instance = FakeSelector('/fw/DA.bin');
    final repository = FakeRepository();
    repository.mtkPorts = ['BROM_123'];
    await pumpChipsets(tester, repository);

    await tester.tap(find.widgetWithText(OutlinedButton, 'Choose DA loader'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(OutlinedButton, 'Kamakiri2 unlock'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Run it'));
    await tester.pumpAndSettle();

    expect(
      repository.calls.where((c) => c.startsWith('exploit-unlock')),
      ['exploit-unlock /fw/DA.bin'],
    );
    expect(find.text('Exploit flow finished'), findsOneWidget);
    expect(find.text('seccfg patched via exploit'), findsOneWidget);
  });

  testWidgets('rpmb read shows the hex preview', (tester) async {
    FileSelectorPlatform.instance = FakeSelector('/fw/DA.bin');
    final repository = FakeRepository();
    repository.mtkPorts = ['BROM_123'];
    await pumpChipsets(tester, repository);

    await tester.tap(find.widgetWithText(OutlinedButton, 'Choose DA loader'));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(OutlinedButton, 'Read RPMB block'));
    await tester.pumpAndSettle();

    expect(
      repository.calls.where((c) => c.startsWith('rpmb-read')),
      ['rpmb-read 0 256'],
    );
    expect(find.text('RPMB block read'), findsOneWidget);
    // the first 32 bytes rendered as hex
    expect(find.textContaining('00 01 02'), findsOneWidget);
  });

  testWidgets('unisoc unlock drives the fastboot flow', (tester) async {
    FileSelectorPlatform.instance = FakeSelector('/fw/fdl.bin');
    final repository = FakeRepository();
    repository.unisocPorts = ['SPD'];
    await pumpChipsets(tester, repository);

    await tester.tap(
      find.widgetWithText(FilledButton, 'Unlock bootloader (T618)'),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Run it'));
    await tester.pumpAndSettle();

    expect(repository.calls, contains('unisoc-unlock'));
    expect(find.text('Unlock flow finished'), findsOneWidget);
    expect(
      find.text('unlock sent; confirm on the device screen'),
      findsOneWidget,
    );
  });
}
