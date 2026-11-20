import 'package:material_ui/material_ui.dart';

/// Seed picked once for both themes: a calm teal-green tied to Android's own
/// system color, so the app reads as a device tool without borrowing Google's
/// default purple. Accent work stays with the scheme's primary alone.
const Color _seedColor = Color(0xFF2E6D64);

/// Extra-large corner for the status card, expressive-large per M3: the one
/// surface on the home screen that carries the device state.
const ShapeBorder _statusCardShape = RoundedRectangleBorder(
  borderRadius: BorderRadius.all(Radius.circular(28)),
);

ThemeData skelloLightTheme() {
  return ThemeData(
    colorScheme: ColorScheme.fromSeed(seedColor: _seedColor),
    cardTheme: const CardThemeData(shape: _statusCardShape),
  );
}

ThemeData skelloDarkTheme() {
  return ThemeData(
    colorScheme: ColorScheme.fromSeed(
      seedColor: _seedColor,
      brightness: Brightness.dark,
    ),
    cardTheme: const CardThemeData(shape: _statusCardShape),
  );
}
