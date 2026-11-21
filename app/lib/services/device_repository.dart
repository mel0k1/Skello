import 'dart:io';

import 'package:skello/src/rust/api/adb.dart' as adb_api;
import 'package:skello/src/rust/api/fastboot.dart' as fastboot_api;

import 'core.dart';
import 'device_output.dart';

class DeviceRepository {
  const DeviceRepository();

  Future<List<AdbDevice>> adbDevices() async {
    if (rustCore.ready) {
      try {
        return parseAdbDevices(await adb_api.adbDevices());
      } catch (_) {
        return _systemAdbDevices();
      }
    }
    return _systemAdbDevices();
  }

  Future<List<FastbootDevice>> fastbootDevices() async {
    if (rustCore.ready) {
      try {
        return parseFastbootDevices(await fastboot_api.fastbootDevices());
      } catch (_) {
        return _systemFastbootDevices();
      }
    }
    return _systemFastbootDevices();
  }

  Future<String?> fastbootVar(String serial, String name) async {
    if (rustCore.ready) {
      try {
        return await fastboot_api.fastbootGetvar(serial: serial, name: name);
      } catch (_) {
        return _systemFastbootVar(serial, name);
      }
    }
    return _systemFastbootVar(serial, name);
  }

  Future<void> adbReboot(String serial, String target) async {
    if (rustCore.ready) {
      try {
        await adb_api.adbReboot(serial: serial, target: target);
        return;
      } catch (_) {}
    }
    final result = await Process.run('adb', [
      '-s',
      serial,
      'reboot',
      if (target.isNotEmpty) target,
    ]);
    if (result.exitCode != 0) {
      throw Exception('adb reboot failed: ${result.stderr}');
    }
  }

  Future<void> fastbootReboot(String serial) async {
    if (rustCore.ready) {
      try {
        await fastboot_api.fastbootReboot(serial: serial);
        return;
      } catch (_) {}
    }
    final result = await Process.run('fastboot', ['-s', serial, 'reboot']);
    if (result.exitCode != 0) {
      throw Exception('fastboot reboot failed: ${result.stderr}');
    }
  }

  Future<void> fastbootUnlock(String serial) async {
    if (rustCore.ready) {
      try {
        await fastboot_api.fastbootFlashingUnlock(serial: serial);
        return;
      } catch (_) {}
    }
    final result = await Process.run('fastboot', [
      '-s',
      serial,
      'flashing',
      'unlock',
    ]);
    if (result.exitCode != 0) {
      throw Exception('fastboot flashing unlock failed: ${result.stderr}');
    }
  }

  Future<String> fastbootFlash(
    String serial,
    String partition,
    String imagePath,
  ) async {
    if (rustCore.ready) {
      try {
        return await fastboot_api.fastbootFlash(
          serial: serial,
          partition: partition,
          image: imagePath,
        );
      } catch (_) {}
    }
    final result = await Process.run('fastboot', [
      '-s',
      serial,
      'flash',
      partition,
      imagePath,
    ]);
    final output = '${result.stdout}${result.stderr}'.trim();
    if (result.exitCode != 0) {
      throw Exception('fastboot flash failed: $output');
    }
    return output;
  }

  Future<List<AdbDevice>> _systemAdbDevices() async {
    final result = await Process.run('adb', const ['devices']);
    return parseAdbDevices(result.stdout.toString());
  }

  Future<List<FastbootDevice>> _systemFastbootDevices() async {
    final result = await Process.run('fastboot', const ['devices']);
    return parseFastbootDevices(result.stdout.toString());
  }

  Future<String?> _systemFastbootVar(String serial, String name) async {
    final result = await Process.run('fastboot', [
      '-s',
      serial,
      'getvar',
      name,
    ]);
    final combined = '${result.stdout}${result.stderr}';
    return parseFastbootVar(combined, name);
  }
}
