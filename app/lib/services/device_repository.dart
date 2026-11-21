import 'dart:async';
import 'dart:io';

import 'package:path_provider/path_provider.dart';
import 'package:skello/src/rust/api/adb.dart' as adb_api;
import 'package:skello/src/rust/api/adb_usb.dart' as adb_usb_api;
import 'package:skello/src/rust/api/fastboot.dart' as fastboot_api;
import 'package:skello/src/rust/api/fastboot_usb.dart' as fastboot_usb_api;

import 'command_log.dart';
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
    final usb = <AdbDevice>[];
    if (rustCore.ready) {
      try {
        final keysDir = await _keysDir();
        if (keysDir != null) {
          await CommandLog.instance
              .write('adb-usb', '>> enumerate + handshake');
          for (final status
              in await adb_usb_api.adbUsbStatus(keysDir: keysDir)) {
            await CommandLog.instance
                .write('adb-usb', '<< ${status.serial}: ${status.state}');
            usb.add(AdbDevice(status.serial, status.state));
          }
        }
      } catch (e) {
        await CommandLog.instance.write('adb-usb', '<< failed: $e');
      }
    }
    final server = await _serverAdbDevices();
    final seen = usb.map((d) => d.serial).toSet();
    return [...usb, ...server.where((d) => !seen.contains(d.serial))];
  }

  Future<List<AdbDevice>> _serverAdbDevices() async {
    if (rustCore.ready) {
      try {
        final output = await adb_api.adbDevices();
        await CommandLog.instance.write('adb', '>> adb devices');
        await CommandLog.instance.write('adb', '<< $output');
        return parseAdbDevices(output);
      } catch (e) {
        await CommandLog.instance.write('adb', '<< failed: $e');
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
        final serials = await fastboot_usb_api.fastbootUsbDevices();
        await CommandLog.instance
            .write('fastboot-usb', '<< ${serials.join(', ')}');
        final seen = binary.map((d) => d.serial).toSet();
        for (final serial in serials) {
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
        final output = await fastboot_api.fastbootDevices();
        await CommandLog.instance.write('fastboot', '>> fastboot devices');
        await CommandLog.instance.write('fastboot', '<< $output');
        return parseFastbootDevices(output);
      } catch (e) {
        await CommandLog.instance.write('fastboot', '<< failed: $e');
        return _systemFastbootDevices();
      }
    }
    return _systemFastbootDevices();
  }

  Future<String?> fastbootVar(String serial, String name) async {
    if (rustCore.ready) {
      try {
        final value = await fastboot_usb_api.fastbootUsbGetvar(
          serial: serial,
          name: name,
        );
        await CommandLog.instance
            .write('fastboot-usb', '>> getvar:$name << $value');
        return value;
      } catch (e) {
        await CommandLog.instance
            .write('fastboot-usb', '>> getvar:$name << failed: $e');
      }
      try {
        final value = await fastboot_api.fastbootGetvar(
          serial: serial,
          name: name,
        );
        await CommandLog.instance
            .write('fastboot', '>> getvar:$name << $value');
        return value;
      } catch (_) {
        return _systemFastbootVar(serial, name);
      }
    }
    return _systemFastbootVar(serial, name);
  }

  Future<void> adbReboot(String serial, String target) async {
    if (rustCore.ready) {
      try {
        final keysDir = await _keysDir();
        if (keysDir != null) {
          await CommandLog.instance.write(
            'adb-usb',
            '>> reboot,${target.isEmpty ? 'system' : target} on $serial',
          );
          await adb_usb_api.adbUsbReboot(
            serial: serial,
            keysDir: keysDir,
            target: target,
          );
          await CommandLog.instance.write('adb-usb', '<< ok');
          return;
        }
      } catch (e) {
        await CommandLog.instance.write('adb-usb', '<< failed: $e');
      }
      try {
        await CommandLog.instance.write(
          'adb',
          '>> adb -s $serial reboot ${target.isEmpty ? '' : target}',
        );
        await adb_api.adbReboot(serial: serial, target: target);
        await CommandLog.instance.write('adb', '<< ok');
        return;
      } catch (e) {
        await CommandLog.instance.write('adb', '<< failed: $e');
      }
    }
    final result = await Process.run('adb', [
      '-s',
      serial,
      'reboot',
      if (target.isNotEmpty) target,
    ]);
    await CommandLog.instance.write(
      'adb',
      '<< exit ${result.exitCode}: ${result.stderr}',
    );
    if (result.exitCode != 0) {
      throw Exception('adb reboot failed: ${result.stderr}');
    }
  }

  Future<void> fastbootReboot(String serial) async {
    if (rustCore.ready) {
      try {
        await CommandLog.instance.write('fastboot-usb', '>> reboot on $serial');
        await fastboot_usb_api.fastbootUsbReboot(serial: serial);
        await CommandLog.instance.write('fastboot-usb', '<< ok');
        return;
      } catch (e) {
        await CommandLog.instance.write('fastboot-usb', '<< failed: $e');
      }
      try {
        await CommandLog.instance.write('fastboot', '>> reboot on $serial');
        await fastboot_api.fastbootReboot(serial: serial);
        await CommandLog.instance.write('fastboot', '<< ok');
        return;
      } catch (e) {
        await CommandLog.instance.write('fastboot', '<< failed: $e');
      }
    }
    final result = await Process.run('fastboot', ['-s', serial, 'reboot']);
    await CommandLog.instance.write(
      'fastboot',
      '<< exit ${result.exitCode}: ${result.stderr}',
    );
    if (result.exitCode != 0) {
      throw Exception('fastboot reboot failed: ${result.stderr}');
    }
  }

  Future<void> fastbootUnlock(String serial) async {
    if (rustCore.ready) {
      try {
        await CommandLog.instance
            .write('fastboot-usb', '>> flashing:unlock on $serial');
        await fastboot_usb_api.fastbootUsbUnlock(serial: serial);
        await CommandLog.instance.write('fastboot-usb', '<< ok');
        return;
      } catch (e) {
        await CommandLog.instance.write('fastboot-usb', '<< failed: $e');
      }
      try {
        await CommandLog.instance.write('fastboot', '>> flashing:unlock');
        await fastboot_api.fastbootFlashingUnlock(serial: serial);
        await CommandLog.instance.write('fastboot', '<< ok');
        return;
      } catch (e) {
        await CommandLog.instance.write('fastboot', '<< failed: $e');
      }
    }
    final result = await Process.run('fastboot', [
      '-s',
      serial,
      'flashing',
      'unlock',
    ]);
    await CommandLog.instance.write(
      'fastboot',
      '<< exit ${result.exitCode}: ${result.stderr}',
    );
    if (result.exitCode != 0) {
      throw Exception('fastboot flashing unlock failed: ${result.stderr}');
    }
  }

  /// Erases a partition over USB; requires an unlocked bootloader.
  Future<String> fastbootErase(String serial, String partition) async {
    final unlocked = await fastbootVar(serial, 'unlocked');
    if (unlocked?.toLowerCase() != 'yes') {
      throw const BootloaderLocked();
    }
    if (rustCore.ready) {
      try {
        await CommandLog.instance
            .write('fastboot-usb', '>> erase:$partition on $serial');
        final output = await fastboot_usb_api.fastbootUsbErase(
          serial: serial,
          partition: partition,
        );
        await CommandLog.instance.write('fastboot-usb', '<< $output');
        return output;
      } catch (e) {
        await CommandLog.instance.write('fastboot-usb', '<< failed: $e');
      }
      try {
        final output = await fastboot_api.fastbootErase(
          serial: serial,
          partition: partition,
        );
        await CommandLog.instance.write('fastboot', '<< $output');
        return output;
      } catch (_) {}
    }
    final result = await Process.run('fastboot', [
      '-s',
      serial,
      'erase',
      partition,
    ]);
    final output = '${result.stdout}${result.stderr}'.trim();
    await CommandLog.instance
        .write('fastboot', '<< exit ${result.exitCode}: $output');
    if (result.exitCode != 0) {
      throw Exception('fastboot erase failed: $output');
    }
    return output;
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
      await CommandLog.instance.write('flash', 'unlocked != yes, refuse');
      throw const BootloaderLocked();
    }
    final socOk = await fastbootVar(serial, 'battery-soc-ok');
    if (socOk != null && socOk.toLowerCase() != 'yes') {
      await CommandLog.instance
          .write('flash', 'battery-soc-ok = $socOk, refuse');
      throw const BatteryNotReady();
    }
    final maxSize = await fastbootVar(serial, 'max-download-size');
    final maxBytes = maxSize == null ? null : parseDownloadSize(maxSize);
    final imageSize = await File(imagePath).length();
    if (maxBytes != null && imageSize > maxBytes) {
      await CommandLog.instance
          .write('flash', 'image $imageSize > max $maxBytes, refuse');
      throw DownloadTooBig(maxBytes, imageSize);
    }
    if (rustCore.ready) {
      try {
        return await _usbFlash(serial, partition, imagePath, onProgress);
      } catch (e) {
        await CommandLog.instance.write('flash', 'usb failed: $e');
      }
    }
    final result = await Process.run('fastboot', [
      '-s',
      serial,
      'flash',
      partition,
      imagePath,
    ]);
    final output = '${result.stdout}${result.stderr}'.trim();
    await CommandLog.instance
        .write('flash', '<< exit ${result.exitCode}: $output');
    if (result.exitCode != 0) {
      throw Exception('fastboot flash failed: $output');
    }
    return output;
  }

  /// Waits until the device reappears in adb after a reboot.
  Future<bool> waitForAdbDevice(
    String serial, {
    Duration timeout = const Duration(seconds: 90),
  }) async {
    final deadline = DateTime.now().add(timeout);
    while (DateTime.now().isBefore(deadline)) {
      final devices = await adbDevices();
      if (devices.any((d) => d.serial == serial && d.state == 'device')) {
        return true;
      }
      await Future<void>.delayed(const Duration(seconds: 2));
    }
    return false;
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
          if (!done.isCompleted) {
            done.complete(event.message);
            CommandLog.instance
                .write('flash', '<< ${event.message.replaceAll('\n', ' | ')}');
          }
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

  Future<String?> _keysDir() async {
    try {
      final dir = await getApplicationSupportDirectory();
      return dir.path;
    } catch (_) {
      return null;
    }
  }

  Future<List<AdbDevice>> _systemAdbDevices() async {
    final result = await Process.run('adb', const ['devices']);
    await CommandLog.instance.write('adb', '<< ${result.stdout}');
    return parseAdbDevices(result.stdout.toString());
  }

  Future<List<FastbootDevice>> _systemFastbootDevices() async {
    final result = await Process.run('fastboot', const ['devices']);
    await CommandLog.instance.write('fastboot', '<< ${result.stdout}');
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
    await CommandLog.instance.write('fastboot', '<< $combined');
    return parseFastbootVar(combined, name);
  }
}
