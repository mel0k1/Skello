import 'dart:io';

import 'package:path_provider/path_provider.dart';

/// Appends every device command and response to
/// `skello-commands.log` in the application support directory, so
/// what happened to the phone stays reconstructable.
class CommandLog {
  static final CommandLog instance = CommandLog();

  CommandLog();

  File? _file;
  bool _resolved = false;

  /// For tests: pin the log file instead of resolving the app directory.
  void bindFileForTesting(File file) {
    _file = file;
    _resolved = true;
  }

  Future<void> write(String source, String message) async {
    await _ensure();
    final file = _file;
    if (file == null) return;
    final stamp = DateTime.now().toUtc().toIso8601String();
    final flat = message.replaceAll('\n', r'\n');
    try {
      await file.writeAsString('$stamp [$source] $flat\n',
          mode: FileMode.append, flush: true);
    } catch (_) {}
  }

  Future<void> _ensure() async {
    if (_resolved) return;
    _resolved = true;
    try {
      final dir = await getApplicationSupportDirectory();
      _file = File(
        '${dir.path}${Platform.pathSeparator}skello-commands.log',
      );
    } catch (_) {
      _file = null;
    }
  }
}
