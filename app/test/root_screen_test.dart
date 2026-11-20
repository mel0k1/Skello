import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:skello/l10n/generated/app_localizations.dart';
import 'package:skello/services/boot_image.dart';
import 'package:skello/src/rust/api/bootimg.dart';
import 'package:skello/ui/root_screen.dart';

class FakeReader implements BootImageReader {
  const FakeReader();

  @override
  Future<BootImageCheck> info(String path) async {
    return BootImageCheck.ok(
      BootImageInfo(
        headerVersion: 3,
        kernelSize: 33554432,
        ramdiskSize: 1048576,
        pageSize: 4096,
        osVersion: '12.0.0',
        osPatchLevel: '2023-05',
        fileSize: BigInt.from(16777216),
        hasAvbFooter: true,
      ),
    );
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
  Widget rootTestWidget() {
    return _wrap(const RootScreen(reader: FakeReader()));
  }

  testWidgets('root screen shows the three methods and the boot picker',
      (tester) async {
    tester.view.physicalSize = const Size(1200, 2200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(rootTestWidget());
    await tester.pumpAndSettle();

    expect(find.text('Magisk'), findsNWidgets(2));
    expect(find.text('KernelSU'), findsOneWidget);
    expect(find.text('APatch'), findsOneWidget);
    expect(find.text('Select boot.img'), findsOneWidget);
  });

  testWidgets('root screen switches guidance with the method', (tester) async {
    tester.view.physicalSize = const Size(1200, 2200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(rootTestWidget());
    await tester.pumpAndSettle();

    await tester.tap(find.text('APatch'));
    await tester.pumpAndSettle();

    expect(find.textContaining('kpatch exactly'), findsOneWidget);
  });

  testWidgets('check view shows image info for a valid image', (tester) async {
    tester.view.physicalSize = const Size(1200, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final check = await const FakeReader().info('boot.img');
    await tester.pumpWidget(_wrap(BootImageCheckView(check: check)));
    await tester.pumpAndSettle();

    expect(find.text('Valid Android boot image'), findsOneWidget);
    expect(find.text('v3'), findsOneWidget);
    expect(find.text('12.0.0'), findsOneWidget);
    expect(find.text('2023-05'), findsOneWidget);
    expect(find.text('present'), findsOneWidget);
  });

  testWidgets('check view shows an error for an unreadable file',
      (tester) async {
    tester.view.physicalSize = const Size(1200, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(_wrap(const BootImageCheckView(
      check: BootImageCheck.fail(BootImageCheckFailure.read),
    )));
    await tester.pumpAndSettle();

    expect(find.text('Check failed'), findsOneWidget);
    expect(find.text('This file is not a readable Android boot image.'),
        findsOneWidget);
  });
}
