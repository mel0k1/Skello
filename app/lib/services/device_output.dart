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

String? parseFastbootVar(String output, String name) {
  for (final line in output.split('\n')) {
    var text = line.trim();
    text = text.replaceFirst(RegExp(r'^\(bootloader\)\s*'), '');
    final prefix = '$name:';
    if (text.startsWith(prefix)) {
      return text.substring(prefix.length).trim();
    }
  }
  return null;
}

/// Parses fastboot max-download-size values: `0x10000000`, `536870912`,
/// `64M`, `512K`, `1G`. Returns bytes, or null if the value is unreadable.
int? parseDownloadSize(String value) {
  final text = value.trim();
  if (text.isEmpty) return null;
  if (text.toLowerCase().startsWith('0x')) {
    final hex = text.substring(2);
    if (hex.isEmpty) return null;
    return int.tryParse(hex, radix: 16);
  }
  final multiplier = switch (text[text.length - 1]) {
    'K' || 'k' => 1024,
    'M' || 'm' => 1024 * 1024,
    'G' || 'g' => 1024 * 1024 * 1024,
    _ => 1,
  };
  final number = multiplier == 1 ? text : text.substring(0, text.length - 1);
  final parsed = int.tryParse(number.trim());
  if (parsed == null || parsed < 0) return null;
  return parsed * multiplier;
}
