import 'dart:async';
import 'dart:io';

import 'package:skello/src/rust/api/adb.dart' as adb_api;
import 'package:skello/src/rust/api/fastboot.dart' as fastboot_api;
import 'package:skello/src/rust/api/fastboot_usb.dart' as fastboot_usb_api;

import 'core.dart';
import 'device_output.dart';
import 'flash_errors.dart';

/// Progress of a USB flash: `phase` is "download" while the image streams
/// to the device, then "flash" while the device writes the partition.
class FlashProgressEvent {
  const FlashProgressEvent(this.sent, this.total, this.phase);

  final int sent;
  final int total;
  final String phase;
}

typedef FlashProgressCallback = void Function(FlashProgressEvent event);

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
    final binary = await _binaryFastbootDevices();
    final merged = [...binary];
    if (rustCore.ready) {
      try {
        final seen = binary.map((d) => d.serial).toSet();
        for (final serial in await fastboot_usb_api.fastbootUsbDevices()) {
          if (!seen.contains(serial)) {
            merged.add(FastbootDevice(serial, 'fastboot'));
          }
        }
      } catch (_) {}
    }
    return merged;
  }

  Future<List<FastbootDevice>> _binaryFastbootDevices() async {
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
        return await fastboot_usb_api.fastbootUsbGetvar(
          serial: serial,
          name: name,
        );
      } catch (_) {}
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
        await fastboot_usb_api.fastbootUsbReboot(serial: serial);
        return;
      } catch (_) {}
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

  /// Pre-flash checks, then flash over USB with progress; falls back to the
  /// system fastboot binary without progress when USB is unavailable.
  Future<String> flashPartition(
    String serial,
    String partition,
    String imagePath,
    FlashProgressCallback? onProgress,
  ) async {
    final unlocked = await fastbootVar(serial, 'unlocked');
    if (unlocked?.toLowerCase() != 'yes') {
      throw const BootloaderLocked();
    }
    final socOk = await fastbootVar(serial, 'battery-soc-ok');
    if (socOk != null && socOk.toLowerCase() != 'yes') {
      throw const BatteryNotReady();
    }
    final maxSize = await fastbootVar(serial, 'max-download-size');
    final maxBytes = maxSize == null ? null : parseDownloadSize(maxSize);
    final imageSize = await File(imagePath).length();
    if (maxBytes != null && imageSize > maxBytes) {
      throw DownloadTooBig(maxBytes, imageSize);
    }
    if (rustCore.ready) {
      try {
        return await _usbFlash(serial, partition, imagePath, onProgress);
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

  Future<String> _usbFlash(
    String serial,
    String partition,
    String imagePath,
    FlashProgressCallback? onProgress,
  ) {
    final done = Completer<String>();
    late final StreamSubscription<fastboot_usb_api.FlashProgress> sub;
    sub = fastboot_usb_api
        .fastbootFlashUsb(
      serial: serial,
      partition: partition,
      imagePath: imagePath,
    )
        .listen(
      (event) {
        if (event.phase == 'done') {
          if (!done.isCompleted) done.complete(event.message);
        } else if (onProgress != null) {
          final sent = event.sent.toInt();
          final total = event.total.toInt();
          onProgress(FlashProgressEvent(
            sent,
            total,
            sent >= total && total > 0 ? 'flash' : event.phase,
          ));
        }
      },
      onError: (Object error) {
        if (!done.isCompleted) done.completeError(error);
      },
      onDone: () {
        if (!done.isCompleted) {
          done.completeError(
            Exception('flash stream ended without result'),
          );
        }
      },
    );
    return done.future.whenComplete(() => sub.cancel());
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
