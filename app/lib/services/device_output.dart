class AdbDevice {
  const AdbDevice(this.serial, this.state);

  final String serial;
  final String state;
}

class FastbootDevice {
  const FastbootDevice(this.serial, this.state);

  final String serial;
  final String state;
}

class DeviceRow {
  const DeviceRow(this.serial, this.state, this.source);

  final String serial;
  final String state;
  final String source;

  bool get isActive => state == 'device' || state == 'fastboot';
  bool get isProblem => state == 'offline' || state == 'unauthorized';
}

const _adbStates = {
  'device',
  'offline',
  'unauthorized',
  'bootloader',
  'recovery',
  'sideload',
  'rescue',
  'no-device',
};

List<AdbDevice> parseAdbDevices(String output) {
  final devices = <AdbDevice>[];
  for (final line in output.split('\n')) {
    final parts = line.trim().split(RegExp(r'\s+'));
    if (parts.length == 2 && _adbStates.contains(parts[1])) {
      devices.add(AdbDevice(parts[0], parts[1]));
    }
  }
  return devices;
}

List<FastbootDevice> parseFastbootDevices(String output) {
  final devices = <FastbootDevice>[];
  for (final line in output.split('\n')) {
    final parts = line.trim().split(RegExp(r'\s+'));
    if (parts.length >= 2 && parts[1] == 'fastboot') {
      devices.add(FastbootDevice(parts[0], 'fastboot'));
    }
  }
  return devices;
}
