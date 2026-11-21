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
}
