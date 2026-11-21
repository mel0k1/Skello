import 'package:flutter_test/flutter_test.dart';
import 'package:skello/services/device_output.dart';

void main() {
  test('parses adb device lines', () {
    const output = 'List of devices attached\n'
        '* daemon started successfully\n'
        'R58N60ABCD\tsideload\n'
        'emulator-5554\toffline\n';
    final devices = parseAdbDevices(output);

    expect(devices.length, 2);
    expect(devices[0].serial, 'R58N60ABCD');
    expect(devices[0].state, 'sideload');
    expect(devices[1].serial, 'emulator-5554');
    expect(devices[1].state, 'offline');
  });

  test('ignores junk adb lines', () {
    expect(parseAdbDevices(''), isEmpty);
    expect(parseAdbDevices('List of devices attached\n'), isEmpty);
    expect(parseAdbDevices('* daemon started successfully\n'), isEmpty);
  });

  test('parses fastboot device lines', () {
    const output = 'ABC123\tfastboot\nXYZ789\tfastboot\n\n';
    final devices = parseFastbootDevices(output);

    expect(devices.length, 2);
    expect(devices.first.serial, 'ABC123');
    expect(devices.first.state, 'fastboot');
  });

  test('ignores non-fastboot lines', () {
    expect(parseFastbootDevices('waiting for device\n'), isEmpty);
    expect(parseFastbootDevices(''), isEmpty);
  });

  test('parses fastboot getvar value with bootloader prefix', () {
    const output = '(bootloader) unlocked: no\nFinished. Total time: 0.100s';
    expect(parseFastbootVar(output, 'unlocked'), 'no');
  });

  test('parses getvar value without prefix', () {
    expect(parseFastbootVar('unlocked: yes\n', 'unlocked'), 'yes');
  });

  test('returns null for a missing variable', () {
    expect(parseFastbootVar('FAILED (remote: unknown)\n', 'unlocked'), isNull);
    expect(parseFastbootVar('', 'unlocked'), isNull);
  });

  test('parses max-download-size values', () {
    expect(parseDownloadSize('0x1fe00000'), 0x1fe00000);
    expect(parseDownloadSize('536870912'), 536870912);
    expect(parseDownloadSize('64M'), 64 * 1024 * 1024);
    expect(parseDownloadSize('512K'), 512 * 1024);
    expect(parseDownloadSize('1G'), 1024 * 1024 * 1024);
  });

  test('rejects junk download sizes', () {
    expect(parseDownloadSize(''), isNull);
    expect(parseDownloadSize('0x'), isNull);
    expect(parseDownloadSize('unknown'), isNull);
  });
}
