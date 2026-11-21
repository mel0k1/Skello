import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:skello/main.dart';
import 'package:skello/services/device_output.dart';

import 'fake_repository.dart';

void main() {
  testWidgets('home shows a connected device card', (tester) async {
    await tester.pumpWidget(SkelloApp(
      repository: FakeRepository(
        adb: const [AdbDevice('R58N60ABCD', 'device')],
      ),
    ));
    await tester.pumpAndSettle();

    expect(find.text('R58N60ABCD'), findsOneWidget);
    expect(find.text('device'), findsOneWidget);
    expect(find.text('adb'), findsOneWidget);
  });

  testWidgets('home shows the empty state without devices', (tester) async {
    await tester.pumpWidget(SkelloApp(repository: FakeRepository()));
    await tester.pumpAndSettle();

    expect(find.text('Device not connected'), findsOneWidget);
  });

  testWidgets('home opens the root screen from the app bar', (tester) async {
    tester.view.physicalSize = const Size(1200, 2200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(SkelloApp(repository: FakeRepository()));
    await tester.pumpAndSettle();

    await tester.tap(find.byTooltip('Root tools'));
    await tester.pumpAndSettle();

    expect(find.text('Magisk'), findsNWidgets(2));
    expect(find.text('KernelSU'), findsOneWidget);
    expect(find.text('APatch'), findsOneWidget);
    expect(find.text('Select boot.img'), findsOneWidget);
  });

  testWidgets('card menu reboots an adb device to bootloader', (tester) async {
    tester.view.physicalSize = const Size(1200, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final repository = FakeRepository(
      adb: const [AdbDevice('R58N60ABCD', 'device')],
    );
    await tester.pumpWidget(SkelloApp(repository: repository));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.more_vert));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Reboot to bootloader'));
    await tester.pumpAndSettle();

    expect(repository.calls, contains('adb-reboot R58N60ABCD bootloader'));
    expect(find.text('Command sent'), findsOneWidget);

    await tester.pump(const Duration(seconds: 3));
    await tester.pumpAndSettle();
  });

  testWidgets('card menu opens the unlock wizard for a fastboot device',
      (tester) async {
    tester.view.physicalSize = const Size(1200, 2200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final repository = FakeRepository(
      fastboot: const [FastbootDevice('ABC123', 'fastboot')],
    );
    await tester.pumpWidget(SkelloApp(repository: repository));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.more_vert));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Unlock bootloader…'));
    await tester.pumpAndSettle();

    expect(find.text('This wipes everything'), findsOneWidget);
    expect(find.text('Bootloader is locked'), findsOneWidget);
    expect(find.textContaining('fastboot flashing unlock'), findsOneWidget);
  });
}
