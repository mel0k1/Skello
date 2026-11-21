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
  Future<String> fastbootFlash(
    String serial,
    String partition,
    String imagePath,
  ) async {
    calls.add('flash $serial $partition');
    return 'Sending boot.img OKAY\nWriting boot.img OKAY';
  }
}
