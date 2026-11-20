import 'package:material_ui/material_ui.dart';

import 'theme/skello_theme.dart';
import 'ui/home_screen.dart';

void main() {
  runApp(const SkelloApp());
}

class SkelloApp extends StatelessWidget {
  const SkelloApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Skello',
      theme: skelloLightTheme(),
      darkTheme: skelloDarkTheme(),
      themeMode: ThemeMode.system,
      home: const HomeScreen(),
    );
  }
}
