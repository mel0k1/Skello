import 'dart:ui' show Size;

import 'package:flutter_test/flutter_test.dart';

import 'package:skello/main.dart';
import 'package:skello/services/device_output.dart';
import 'package:skello/services/device_repository.dart';

class FakeRepository implements DeviceRepository {
  const FakeRepository(this.adb, this.fastboot);

  final List<AdbDevice> adb;
  final List<FastbootDevice> fastboot;

  @override
  Future<List<AdbDevice>> adbDevices() async => adb;

  @override
  Future<List<FastbootDevice>> fastbootDevices() async => fastboot;
}

void main() {
  testWidgets('home shows a connected device card', (tester) async {
    await tester.pumpWidget(SkelloApp(
      repository: FakeRepository(
        const [AdbDevice('R58N60ABCD', 'device')],
        const [],
      ),
    ));
    await tester.pumpAndSettle();

    expect(find.text('R58N60ABCD'), findsOneWidget);
    expect(find.text('device'), findsOneWidget);
    expect(find.text('adb'), findsOneWidget);
  });

  testWidgets('home shows the empty state without devices', (tester) async {
    await tester.pumpWidget(const SkelloApp(
      repository: FakeRepository([], []),
    ));
    await tester.pumpAndSettle();

    expect(find.text('Device not connected'), findsOneWidget);
  });

  testWidgets('home opens the root screen from the app bar', (tester) async {
    tester.view.physicalSize = const Size(1200, 2200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(const SkelloApp(
      repository: FakeRepository([], []),
    ));
    await tester.pumpAndSettle();

    await tester.tap(find.byTooltip('Root tools'));
    await tester.pumpAndSettle();

    expect(find.text('Magisk'), findsNWidgets(2));
    expect(find.text('KernelSU'), findsOneWidget);
    expect(find.text('APatch'), findsOneWidget);
    expect(find.text('Select boot.img'), findsOneWidget);
  });
}
