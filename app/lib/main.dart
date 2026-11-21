import 'dart:io';

import 'package:flutter_localizations/flutter_localizations.dart'
    hide GlobalMaterialLocalizations;
import 'package:material_ui/material_ui.dart';
import 'package:path_provider/path_provider.dart';
import 'package:skello/src/rust/api/log.dart' as core_log;

import 'l10n/generated/app_localizations.dart';
import 'services/core.dart';
import 'services/device_repository.dart';
import 'theme/skello_theme.dart';
import 'ui/home_screen.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await rustCore.init();
  // The Rust-side transaction log writes to the same journal file as
  // the Dart CommandLog.
  try {
    final dir = await getApplicationSupportDirectory();
    await core_log.setCoreLogFile(
      path: '${dir.path}${Platform.pathSeparator}skello-commands.log',
    );
  } catch (_) {}
  runApp(const SkelloApp());
}

class SkelloApp extends StatelessWidget {
  const SkelloApp({super.key, this.repository = const DeviceRepository()});

  final DeviceRepository repository;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Skello',
      theme: skelloLightTheme(),
      darkTheme: skelloDarkTheme(),
      themeMode: ThemeMode.system,
      localizationsDelegates: [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      home: HomeScreen(repository: repository),
    );
  }
}
