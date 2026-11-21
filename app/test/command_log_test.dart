import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:skello/services/command_log.dart';

void main() {
  test('writes timestamped flattened lines to the log file', () async {
    final dir = await Directory.systemTemp.createTemp('skello-log');
    addTearDown(() => dir.delete(recursive: true));
    final file =
        File('${dir.path}${Platform.pathSeparator}skello-commands.log');
    final log = CommandLog();
    log.bindFileForTesting(file);
    await log.write('fastboot-usb', '>> getvar:unlocked << yes');
    await log.write('flash', '<< OKAY line1\nline2');
    final content = file.readAsStringSync();
    final lines = content.trim().split('\n');
    expect(lines, hasLength(2));
    expect(lines[0], contains('[fastboot-usb] >> getvar:unlocked << yes'));
    expect(lines[1], contains('line1\\nline2'));
    expect(lines[0].length, greaterThan(20));
  });
}
