import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:skello/main.dart';

void main() {
  testWidgets('home screen shows the device empty state', (tester) async {
    await tester.pumpWidget(const SkelloApp());

    expect(find.text('Skello'), findsOneWidget);
    expect(find.text('Device not connected'), findsOneWidget);
    expect(find.byIcon(Icons.phone_android), findsOneWidget);
  });
}
