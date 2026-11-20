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

  Future<List<AdbDevice>> _systemAdbDevices() async {
    final result = await Process.run('adb', const ['devices']);
    return parseAdbDevices(result.stdout.toString());
  }

  Future<List<FastbootDevice>> _systemFastbootDevices() async {
    final result = await Process.run('fastboot', const ['devices']);
    return parseFastbootDevices(result.stdout.toString());
  }
}
