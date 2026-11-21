import 'package:skello/services/device_output.dart';
import 'package:skello/services/device_repository.dart';

class FakeRepository implements DeviceRepository {
  FakeRepository({
    this.adb = const [],
    this.fastboot = const [],
    this.unlockedValue = 'no',
  });

  List<AdbDevice> adb;
  List<FastbootDevice> fastboot;
  String? unlockedValue;
  bool comesBack = true;
  bool rooted = true;
  String seccfgState = 'locked';
  String? shellOutput;
  final List<String> calls = [];

  @override
  Future<List<AdbDevice>> adbDevices() async => adb;

  @override
  Future<List<FastbootDevice>> fastbootDevices() async => fastboot;

  @override
  Future<String?> fastbootVar(String serial, String name) async {
    calls.add('getvar $serial $name');
    return unlockedValue;
  }

  @override
  Future<void> adbReboot(String serial, String target) async {
    calls.add('adb-reboot $serial $target');
  }

  @override
  Future<void> fastbootReboot(String serial) async {
    calls.add('fastboot-reboot $serial');
  }

  @override
  Future<void> fastbootUnlock(String serial) async {
    calls.add('flashing-unlock $serial');
    unlockedValue = 'yes';
  }

  @override
  Future<String> flashPartition(
    String serial,
    String partition,
    String imagePath,
    FlashProgressCallback? onProgress,
  ) async {
    calls.add('flash $serial $partition');
    onProgress?.call(const FlashProgressEvent(50, 100, 'download'));
    onProgress?.call(const FlashProgressEvent(100, 100, 'flash'));
    return 'Sending OKAY\nWriting OKAY';
  }

  @override
  Future<String> fastbootErase(String serial, String partition) async {
    calls.add('erase $serial $partition');
    return 'Erasing OKAY';
  }

  @override
  Future<String> adbShell(String serial, String command) async {
    calls.add('shell $serial $command');
    if (shellOutput != null) return shellOutput!;
    return 'ok';
  }

  @override
  Future<SeccfgStatus> mediatekSeccfgStatus(String serial) async {
    calls.add('seccfg-status $serial');
    if (!rooted) throw Exception('root access required');
    return SeccfgStatus('V3', seccfgState, 'body attr default');
  }

  @override
  Future<String> mediatekSeccfgUnlock(String serial) async {
    calls.add('seccfg-unlock $serial');
    if (!rooted) throw Exception('root access required');
    if (seccfgState == 'unlocked') {
      throw Exception('seccfg bad header: already unlocked');
    }
    seccfgState = 'unlocked';
    return 'seccfg patched to unlocked; reboot and verify';
  }

  @override
  Future<bool> waitForAdbDevice(
    String serial, {
    Duration timeout = const Duration(seconds: 90),
  }) async {
    calls.add('wait-for-device $serial');
    return comesBack;
  }

  List<String> mtkPorts = const [];
  List<String> unisocPorts = const [];
  List<String> qualcommPorts = const [];
  List<ChipsetProgramEntry> parsedEntries = const [];
  String daUnlockMessage = 'seccfg patched over the DA';
  String unisocFlashMessage = 'partition written';
  String edlFlashMessage = 'flash OKAY (all entries)';
  String v6BootMessage = 'DA V6 booted, device set to fastboot';
  List<UnisocPartitionRow> unisocTable = const [];
  List<String> serialPorts = const [];
  String serialFlashMessage = 'flash OKAY over serial';
  bool failChipsetCalls = false;

  ChipsetPortGroups get portGroups => ChipsetPortGroups(
    mtkPorts,
    unisocPorts,
    qualcommPorts,
  );

  @override
  Future<ChipsetPortGroups> chipsetPorts() async {
    calls.add('chipset-ports');
    if (failChipsetCalls) throw Exception('usb enumeration failed');
    return portGroups;
  }

  @override
  Future<String> mediatekDaUnlock(String loaderPath) async {
    calls.add('mtk-da-unlock $loaderPath');
    if (failChipsetCalls) throw Exception('brom handshake failed');
    return daUnlockMessage;
  }

  @override
  Future<String> unisocFlashPartition({
    required String fdl1Path,
    required String fdl2Path,
    required int fdl1Addr,
    required int fdl2Addr,
    required String partition,
    required String imagePath,
  }) async {
    calls.add(
      'unisoc-flash $partition $fdl1Addr $fdl2Addr',
    );
    if (failChipsetCalls) throw Exception('fdl2 dead');
    return unisocFlashMessage;
  }

  @override
  Future<List<ChipsetProgramEntry>> parseRawprogram(String xmlPath) async {
    calls.add('rawprogram-parse $xmlPath');
    if (failChipsetCalls) throw Exception('bad xml');
    return parsedEntries;
  }

  @override
  Future<String> qualcommEdlFlash(
    String programmerPath,
    List<ChipsetProgramEntry> entries,
  ) async {
    calls.add('edl-flash ${entries.length} via $programmerPath');
    if (failChipsetCalls) throw Exception('sahara failed');
    return edlFlashMessage;
  }

  @override
  Future<String> mediatekV6Boot(String loaderPath, String mode) async {
    calls.add('v6-boot $mode $loaderPath');
    if (failChipsetCalls) throw Exception('xml da failed');
    return v6BootMessage;
  }

  @override
  Future<List<UnisocPartitionRow>> unisocPartitionList(
    String fdl1Path,
    String fdl2Path,
    int fdl1Addr,
    int fdl2Addr,
  ) async {
    calls.add('unisoc-table $fdl1Addr $fdl2Addr');
    if (failChipsetCalls) throw Exception('fdl2 dead');
    return unisocTable;
  }

  @override
  Future<List<String>> qualcommSerialPorts() async {
    calls.add('serial-ports');
    if (failChipsetCalls) throw Exception('serial scan failed');
    return serialPorts;
  }

  @override
  Future<String> qualcommEdlFlashSerial(
    String programmerPath,
    List<ChipsetProgramEntry> entries,
    String portPath,
  ) async {
    calls.add('edl-serial-flash ${entries.length} on $portPath');
    if (failChipsetCalls) throw Exception('sahara failed');
    return serialFlashMessage;
  }

  String bromDumpMessage = 'bootrom dumped (MT6768, 131072 bytes)';
  String exploitUnlockMessage = 'seccfg patched via exploit';
  String unisocUnlockMessage = 'unlock sent; confirm on the device screen';
  List<int> rpmbData = List.generate(256, (i) => i);

  @override
  Future<String> mediatekExploitDumpBrom(String outputPath) async {
    calls.add('exploit-dump $outputPath');
    if (failChipsetCalls) throw Exception('no kamakiri2 config');
    return bromDumpMessage;
  }

  @override
  Future<String> mediatekExploitUnlock(String loaderPath) async {
    calls.add('exploit-unlock $loaderPath');
    if (failChipsetCalls) throw Exception('payload ack mismatch');
    return exploitUnlockMessage;
  }

  @override
  Future<String> mediatekCarbonaraUnlock(String loaderPath) async {
    calls.add('carbonara-unlock $loaderPath');
    if (failChipsetCalls) throw Exception('loader is patched against carbonara');
    return exploitUnlockMessage;
  }

  @override
  Future<List<int>> mediatekRpmbRead(
    String loaderPath,
    int addr,
    int length,
  ) async {
    calls.add('rpmb-read $addr $length');
    if (failChipsetCalls) throw Exception('da refused rpmb');
    return rpmbData.take(length).toList();
  }

  @override
  Future<String> unisocUnlockBootloader() async {
    calls.add('unisoc-unlock');
    if (failChipsetCalls) throw Exception('no fastboot device found');
    return unisocUnlockMessage;
  }

  String pacInfo = 'TEST_FW 1.0 | CRC ok | flash files (1): boot (64 KB)';
  String pacFlashMessage = 'flashed 1: boot; reset';
  String eraseMessage = 'boot erased (2048 sectors)';
  String scatterMessage = 'scatter MT6768: flashed 2 partitions: preloader, boot';

  @override
  Future<String> unisocPacInfo(String pacPath) async {
    calls.add('pac-info $pacPath');
    if (failChipsetCalls) throw Exception('pac: not a Unisoc .pac package');
    return pacInfo;
  }

  @override
  Future<String> unisocPacFlash(String pacPath) async {
    calls.add('pac-flash $pacPath');
    if (failChipsetCalls) throw Exception('pac flash: package data crc mismatch');
    return pacFlashMessage;
  }

  @override
  Future<String> mediatekErasePartition(String loaderPath, String name) async {
    calls.add('erase $name');
    if (failChipsetCalls) throw Exception('partition $name not found');
    return eraseMessage;
  }

  @override
  Future<String> mediatekScatterFlash(String loaderPath, String scatterPath) async {
    calls.add('scatter-flash $scatterPath');
    if (failChipsetCalls) throw Exception('scatter line 14: not a hex number');
    return scatterMessage;
  }

  String v6Message = 'boot read, 65536 bytes';
  String qcMessage = 'boot erased (2048 sectors)';

  @override
  Future<String> mediatekV6ReadPartition(
    String loaderPath,
    String partition,
    String outPath,
  ) async {
    calls.add('v6-read $partition');
    if (failChipsetCalls) throw Exception('v6 read: partition boot not found');
    return v6Message;
  }

  @override
  Future<String> mediatekV6WritePartition(
    String loaderPath,
    String partition,
    String imagePath,
  ) async {
    calls.add('v6-write $partition');
    if (failChipsetCalls) throw Exception('v6 write: image is 4096 bytes, partition holds 2048');
    return v6Message;
  }

  @override
  Future<String> mediatekV6ErasePartition(
    String loaderPath,
    String partition,
  ) async {
    calls.add('v6-erase $partition');
    if (failChipsetCalls) throw Exception('v6 erase: partition boot not found');
    return v6Message;
  }

  @override
  Future<String> qualcommEdlErase(String programmerPath, String partition) async {
    calls.add('edl-erase $partition');
    if (failChipsetCalls) throw Exception('partition boot not found');
    return qcMessage;
  }

  @override
  Future<String> qualcommEdlPatch(String programmerPath, String patchPath) async {
    calls.add('edl-patch $patchPath');
    if (failChipsetCalls) throw Exception('no DISK patch entries found');
    return '3 patch entries applied';
  }
}
