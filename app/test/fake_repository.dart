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
}