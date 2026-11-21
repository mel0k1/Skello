/// Typed failures of the pre-flash checks; the UI maps them to
/// localized messages.
class BootloaderLocked implements Exception {
  const BootloaderLocked();
}

class BatteryNotReady implements Exception {
  const BatteryNotReady();
}

class DownloadTooBig implements Exception {
  const DownloadTooBig(this.maxSize, this.imageSize);

  final int maxSize;
  final int imageSize;
}
