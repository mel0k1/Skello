import 'package:material_ui/material_ui.dart';

const Color _seedColor = Color(0xFF2E6D64);

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
