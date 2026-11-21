import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:skello/l10n/generated/app_localizations.dart';
import 'package:skello/ui/unlock_screen.dart';

import 'fake_repository.dart';

Widget _wrap(Widget child) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: child,
  );
}

void main() {
  testWidgets('locked bootloader unlocks after confirmation', (tester) async {
    tester.view.physicalSize = const Size(1200, 1600);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final repository = FakeRepository();
    await tester.pumpWidget(
      _wrap(UnlockScreen(serial: 'ABC123', repository: repository)),
    );
    await tester.pumpAndSettle();

    expect(find.text('Bootloader is locked'), findsOneWidget);
    final button = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Unlock'),
    );
    expect(button.onPressed, isNull);

    await tester.tap(find.byType(Checkbox));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Unlock'));
    await tester.pumpAndSettle();

    expect(repository.calls, contains('flashing-unlock ABC123'));
    expect(find.text('Bootloader is already unlocked'), findsOneWidget);
  });

  testWidgets('already unlocked bootloader shows the done state',
      (tester) async {
    tester.view.physicalSize = const Size(1200, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final repository = FakeRepository(unlockedValue: 'yes');
    await tester.pumpWidget(
      _wrap(UnlockScreen(serial: 'ABC123', repository: repository)),
    );
    await tester.pumpAndSettle();

    expect(find.text('Bootloader is already unlocked'), findsOneWidget);
    expect(repository.calls, isNot(contains('flashing-unlock ABC123')));
  });

  testWidgets('unknown state offers a retry', (tester) async {
    tester.view.physicalSize = const Size(1200, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    final repository = FakeRepository(unlockedValue: null);
    await tester.pumpWidget(
      _wrap(UnlockScreen(serial: 'ABC123', repository: repository)),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('Could not read the bootloader state'),
        findsOneWidget);
    expect(find.text('Retry'), findsOneWidget);
  });
}
