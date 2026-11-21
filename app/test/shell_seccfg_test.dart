import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:skello/l10n/generated/app_localizations.dart';
import 'package:skello/services/device_output.dart';
import 'package:skello/ui/home_screen.dart';
import 'package:skello/ui/seccfg_screen.dart';
import 'package:skello/ui/shell_screen.dart';

import 'fake_repository.dart';

Widget _wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: child,
  );
}

void main() {
  testWidgets('shell screen runs a command and shows the output', (
    tester,
  ) async {
    final repository = FakeRepository();
    repository.shellOutput = 'uid=0(root)';
    await tester.pumpWidget(
      _wrap(ShellScreen(serial: 'ABC123', repository: repository)),
    );
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), 'id');
    await tester.tap(find.byType(IconButton));
    await tester.pumpAndSettle();

    expect(find.text('> id'), findsOneWidget);
    expect(find.text('uid=0(root)'), findsOneWidget);
    expect(repository.calls, contains('shell ABC123 id'));
    expect(find.text('(no output)'), findsNothing);
  });

  testWidgets('shell screen keeps the entry enabled while running', (
    tester,
  ) async {
    final repository = FakeRepository();
    await tester.pumpWidget(
      _wrap(ShellScreen(serial: 'ABC123', repository: repository)),
    );
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), 'getprop');
    await tester.tap(find.byType(IconButton));
    await tester.pumpAndSettle();

    expect(find.text('> getprop'), findsOneWidget);
    expect(find.text('ok'), findsOneWidget);
    // the field is re-enabled after the run
    final field = tester.widget<TextField>(find.byType(TextField));
    expect(field.enabled, isTrue);
  });

  testWidgets('seccfg screen shows the lock state and unlocks', (tester) async {
    final repository = FakeRepository();
    repository.seccfgState = 'locked';
    await tester.pumpWidget(
      _wrap(SeccfgScreen(serial: 'ABC123', repository: repository)),
    );
    await tester.pumpAndSettle();

    expect(find.text('Lock state: locked'), findsOneWidget);
    expect(find.text('Format: V3'), findsOneWidget);

    await tester.tap(find.text('Unlock (seccfg)'));
    await tester.pumpAndSettle();
    expect(find.text('Patch seccfg?'), findsOneWidget);
    expect(find.text('Cancel'), findsOneWidget);
    await tester
        .tap(find.widgetWithText(FilledButton, 'Unlock (seccfg)').last);
    await tester.pumpAndSettle();

    expect(find.text('seccfg patched to unlocked'), findsOneWidget);
    expect(find.text('Lock state: locked'), findsNothing);
    expect(repository.calls, contains('seccfg-unlock ABC123'));
  });

  testWidgets('seccfg screen reports a root error', (tester) async {
    final repository = FakeRepository();
    repository.rooted = false;
    await tester.pumpWidget(
      _wrap(SeccfgScreen(serial: 'ABC123', repository: repository)),
    );
    await tester.pumpAndSettle();

    expect(find.text('seccfg failed'), findsOneWidget);
    expect(
      find.text('Root access is required to read the seccfg block device.'),
      findsOneWidget,
    );
    expect(repository.calls, contains('seccfg-status ABC123'));
  });

  testWidgets('adb card menu offers the shell and seccfg screens', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 2200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final repository = FakeRepository(
      adb: const [AdbDevice('ABC123', 'device')],
    );
    await tester.pumpWidget(_wrap(HomeScreen(repository: repository)));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.more_vert));
    await tester.pumpAndSettle();

    expect(find.text('Shell'), findsOneWidget);
    expect(find.text('MediaTek seccfg'), findsOneWidget);

    await tester.tap(find.text('Shell'));
    await tester.pumpAndSettle();
    expect(find.text('Shell ABC123'), findsOneWidget);
  });
}
