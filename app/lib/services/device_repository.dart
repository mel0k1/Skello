import 'dart:async';
import 'dart:io';

import 'package:path_provider/path_provider.dart';
import 'package:skello/src/rust/api/adb.dart' as adb_api;
import 'package:skello/src/rust/api/adb_usb.dart' as adb_usb_api;
import 'package:skello/src/rust/api/fastboot.dart' as fastboot_api;
import 'package:skello/src/rust/api/fastboot_usb.dart' as fastboot_usb_api;
import 'package:skello/src/rust/api/chipsets.dart' as chipsets_api;
import 'package:skello/src/rust/api/mediatek.dart' as mediatek_api;

import 'command_log.dart';
import 'core.dart';
import 'device_output.dart';
import 'flash_errors.dart';

/// MediaTek seccfg partition state reported by the Rust core.
class SeccfgStatus {
  const SeccfgStatus(this.version, this.state, this.detail);

  final String version;
  final String state;
  final String detail;
}

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
          await CommandLog.instance.write(
            'adb-usb',
            '>> enumerate + handshake',
          );
          for (final status in await adb_usb_api.adbUsbStatus(
            keysDir: keysDir,
          )) {
            await CommandLog.instance.write(
              'adb-usb',
              '<< ${status.serial}: ${status.state}',
            );
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
        await CommandLog.instance.write(
          'fastboot-usb',
          '<< ${serials.join(', ')}',
        );
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
        await CommandLog.instance.write(
          'fastboot-usb',
          '>> getvar:$name << $value',
        );
        return value;
      } catch (e) {
        await CommandLog.instance.write(
          'fastboot-usb',
          '>> getvar:$name << failed: $e',
        );
      }
      try {
        final value = await fastboot_api.fastbootGetvar(
          serial: serial,
          name: name,
        );
        await CommandLog.instance.write(
          'fastboot',
          '>> getvar:$name << $value',
        );
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
        await CommandLog.instance.write(
          'fastboot-usb',
          '>> flashing:unlock on $serial',
        );
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
        await CommandLog.instance.write(
          'fastboot-usb',
          '>> erase:$partition on $serial',
        );
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
    await CommandLog.instance.write(
      'fastboot',
      '<< exit ${result.exitCode}: $output',
    );
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
      await CommandLog.instance.write(
        'flash',
        'battery-soc-ok = $socOk, refuse',
      );
      throw const BatteryNotReady();
    }
    final maxSize = await fastbootVar(serial, 'max-download-size');
    final maxBytes = maxSize == null ? null : parseDownloadSize(maxSize);
    final imageSize = await File(imagePath).length();
    if (maxBytes != null && imageSize > maxBytes) {
      await CommandLog.instance.write(
        'flash',
        'image $imageSize > max $maxBytes, refuse',
      );
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
    await CommandLog.instance.write(
      'flash',
      '<< exit ${result.exitCode}: $output',
    );
    if (result.exitCode != 0) {
      throw Exception('fastboot flash failed: $output');
    }
    return output;
  }

  /// Runs one adb shell command: USB transport first, then the adb
  /// server binary.
  Future<String> adbShell(String serial, String command) async {
    if (rustCore.ready) {
      try {
        final keysDir = await _keysDir();
        if (keysDir != null) {
          final out = await adb_usb_api.adbUsbShell(
            serial: serial,
            keysDir: keysDir,
            command: command,
          );
          await CommandLog.instance.write(
            'adb-usb',
            '>> shell:$command << ${out.length} bytes',
          );
          return out;
        }
      } catch (e) {
        await CommandLog.instance.write(
          'adb-usb',
          '>> shell:$command << failed: $e',
        );
      }
    }
    final result = await Process.run('adb', ['-s', serial, 'shell', command]);
    final out = result.stdout.toString();
    await CommandLog.instance.write(
      'adb',
      '>> shell:$command << exit ${result.exitCode}: $out',
    );
    if (result.exitCode != 0) {
      throw Exception('adb shell failed: ${result.stderr}');
    }
    return out;
  }

  /// Reads the MediaTek seccfg partition and reports the lock state.
  Future<SeccfgStatus> mediatekSeccfgStatus(String serial) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    final keysDir = await _keysDir();
    if (keysDir == null) {
      throw Exception('app support directory unavailable');
    }
    await CommandLog.instance.write('mediatek', '>> status on $serial');
    try {
      final report = await mediatek_api.mediatekSeccfgStatus(
        serial: serial,
        keysDir: keysDir,
      );
      await CommandLog.instance.write(
        'mediatek',
        '<< ${report.state} (${report.version})',
      );
      return SeccfgStatus(report.version, report.state, report.detail);
    } catch (e) {
      await CommandLog.instance.write('mediatek', '<< failed: $e');
      rethrow;
    }
  }

  /// Patches seccfg to unlocked over adb (rooted device).
  Future<String> mediatekSeccfgUnlock(String serial) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    final keysDir = await _keysDir();
    if (keysDir == null) {
      throw Exception('app support directory unavailable');
    }
    await CommandLog.instance.write('mediatek', '>> unlock on $serial');
    try {
      final message = await mediatek_api.mediatekSeccfgUnlock(
        serial: serial,
        keysDir: keysDir,
      );
      await CommandLog.instance.write('mediatek', '<< ok');
      return message;
    } catch (e) {
      await CommandLog.instance.write('mediatek', '<< failed: $e');
      rethrow;
    }
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

  /// Lists the vendor bootloader USB ports without opening them.
  Future<ChipsetPortGroups> chipsetPorts() async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    try {
      final ports = await chipsets_api.chipsetPorts();
      await CommandLog.instance.write(
        'chipsets',
        '<< ports mtk=${ports.mediatek.length} '
            'unisoc=${ports.unisoc.length} qualcomm=${ports.qualcomm.length}',
      );
      return ChipsetPortGroups(ports.mediatek, ports.unisoc, ports.qualcomm);
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// MediaTek unlock without root over BROM and the stock Download Agent.
  Future<String> mediatekDaUnlock(String loaderPath) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write('chipsets', '>> mtk da unlock');
    try {
      final message = await chipsets_api.mediatekDaUnlock(
        loaderPath: loaderPath,
      );
      await CommandLog.instance.write(
        'chipsets',
        '<< ${message.replaceAll('\n', ' | ')}',
      );
      return message;
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// Unisoc partition flash over the factory FDL1/FDL2 loaders.
  Future<String> unisocFlashPartition({
    required String fdl1Path,
    required String fdl2Path,
    required int fdl1Addr,
    required int fdl2Addr,
    required String partition,
    required String imagePath,
  }) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write(
      'chipsets',
      '>> unisoc flash $partition from $imagePath',
    );
    try {
      final message = await chipsets_api.unisocFlashPartition(
        fdl1Path: fdl1Path,
        fdl2Path: fdl2Path,
        fdl1Addr: fdl1Addr,
        fdl2Addr: fdl2Addr,
        partition: partition,
        imagePath: imagePath,
      );
      await CommandLog.instance.write(
        'chipsets',
        '<< ${message.replaceAll('\n', ' | ')}',
      );
      return message;
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// Parses a rawprogram XML into flashable entries.
  Future<List<ChipsetProgramEntry>> parseRawprogram(String xmlPath) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write('chipsets', '>> rawprogram parse');
    try {
      final entries = await chipsets_api.parseRawprogram(xmlPath: xmlPath);
      await CommandLog.instance.write(
        'chipsets',
        '<< ${entries.length} entries',
      );
      return [
        for (final e in entries)
          ChipsetProgramEntry(
            label: e.label,
            startSector: e.startSector,
            numSectors: e.numSectors,
            physPart: e.physPart,
            filePath: e.filePath,
          ),
      ];
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// Qualcomm EDL flash: Sahara programmer plus rawprogram entries.
  Future<String> qualcommEdlFlash(
    String programmerPath,
    List<ChipsetProgramEntry> entries,
  ) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write(
      'chipsets',
      '>> edl flash ${entries.length} entries',
    );
    try {
      final message = await chipsets_api.qualcommEdlFlash(
        programmerPath: programmerPath,
        entries: [
          for (final e in entries)
            chipsets_api.ProgramEntry(
              label: e.label,
              startSector: e.startSector,
              numSectors: e.numSectors,
              physPart: e.physPart,
              filePath: e.filePath,
            ),
        ],
      );
      await CommandLog.instance.write(
        'chipsets',
        '<< ${message.replaceAll('\n', ' | ')}',
      );
      return message;
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// Boots a V6 XML Download Agent into fastboot or meta.
  Future<String> mediatekV6Boot(String loaderPath, String mode) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write('chipsets', '>> v6 boot $mode');
    try {
      final message = await chipsets_api.mediatekV6Boot(
        loaderPath: loaderPath,
        mode: mode,
      );
      await CommandLog.instance.write(
        'chipsets',
        '<< ${message.replaceAll('\n', ' | ')}',
      );
      return message;
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// Reads the Unisoc partition table over the FDL loaders.
  Future<List<UnisocPartitionRow>> unisocPartitionList(
    String fdl1Path,
    String fdl2Path,
    int fdl1Addr,
    int fdl2Addr,
  ) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write('chipsets', '>> unisoc table');
    try {
      final table = await chipsets_api.unisocPartitionList(
        fdl1Path: fdl1Path,
        fdl2Path: fdl2Path,
        fdl1Addr: fdl1Addr,
        fdl2Addr: fdl2Addr,
      );
      await CommandLog.instance.write(
        'chipsets',
        '<< ${table.length} partitions',
      );
      return [
        for (final p in table) UnisocPartitionRow(p.name, p.sizeSectors),
      ];
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// Serial ports present on the system: the EDL UART candidates.
  Future<List<String>> qualcommSerialPorts() async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    try {
      final ports = await chipsets_api.qualcommSerialPorts();
      await CommandLog.instance.write(
        'chipsets',
        '<< serial ports: ${ports.length}',
      );
      return ports;
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// Qualcomm EDL flash over a serial port (Sahara + Firehose).
  Future<String> qualcommEdlFlashSerial(
    String programmerPath,
    List<ChipsetProgramEntry> entries,
    String portPath,
  ) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write(
      'chipsets',
      '>> edl serial flash ${entries.length} entries on $portPath',
    );
    try {
      final message = await chipsets_api.qualcommEdlFlashSerial(
        programmerPath: programmerPath,
        entries: [
          for (final e in entries)
            chipsets_api.ProgramEntry(
              label: e.label,
              startSector: e.startSector,
              numSectors: e.numSectors,
              physPart: e.physPart,
              filePath: e.filePath,
            ),
        ],
        portPath: portPath,
      );
      await CommandLog.instance.write(
        'chipsets',
        '<< ${message.replaceAll('\n', ' | ')}',
      );
      return message;
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }


  /// Kamakiri2 bootrom dump for offline analysis.
  Future<String> mediatekExploitDumpBrom(String outputPath) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write('chipsets', '>> exploit: brom dump');
    try {
      final message = await chipsets_api.mediatekExploitDumpBrom(
        outputPath: outputPath,
      );
      await CommandLog.instance.write(
        'chipsets',
        '<< ${message.replaceAll('\n', ' | ')}',
      );
      return message;
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// Kamakiri2 bypass plus the stock seccfg unlock.
  Future<String> mediatekExploitUnlock(String loaderPath) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write('chipsets', '>> exploit: kamakiri2 unlock');
    try {
      final message = await chipsets_api.mediatekExploitUnlock(
        loaderPath: loaderPath,
      );
      await CommandLog.instance.write(
        'chipsets',
        '<< ${message.replaceAll('\n', ' | ')}',
      );
      return message;
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// Kamakiri2 bypass, Carbonara patched stage 2, then the unlock.
  Future<String> mediatekCarbonaraUnlock(String loaderPath) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write('chipsets', '>> exploit: carbonara unlock');
    try {
      final message = await chipsets_api.mediatekCarbonaraUnlock(
        loaderPath: loaderPath,
      );
      await CommandLog.instance.write(
        'chipsets',
        '<< ${message.replaceAll('\n', ' | ')}',
      );
      return message;
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// Reads the RPMB block through the DA.
  Future<List<int>> mediatekRpmbRead(String loaderPath, int addr, int length) async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write('chipsets', '>> rpmb read $addr $length');
    try {
      final data = await chipsets_api.mediatekRpmbRead(
        loaderPath: loaderPath,
        addr: BigInt.from(addr),
        length: length,
      );
      await CommandLog.instance.write('chipsets', '<< ${data.length} bytes');
      return data;
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
  }

  /// Unisoc bootloader unlock (T618 key): token, signature, flash.
  Future<String> unisocUnlockBootloader() async {
    if (!rustCore.ready) {
      throw Exception('rust core unavailable');
    }
    await CommandLog.instance.write('chipsets', '>> unisoc unlock');
    try {
      final message = await chipsets_api.unisocUnlockBootloader();
      await CommandLog.instance.write(
        'chipsets',
        '<< ${message.replaceAll('\n', ' | ')}',
      );
      return message;
    } catch (e) {
      await CommandLog.instance.write('chipsets', '<< failed: $e');
      rethrow;
    }
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
            CommandLog.instance.write(
              'flash',
              '<< ${event.message.replaceAll('\n', ' | ')}',
            );
          }
        } else if (onProgress != null) {
          final sent = event.sent.toInt();
          final total = event.total.toInt();
          onProgress(
            FlashProgressEvent(
              sent,
              total,
              sent >= total && total > 0 ? 'flash' : event.phase,
            ),
          );
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
