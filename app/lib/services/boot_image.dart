import 'package:skello/src/rust/api/bootimg.dart';

import 'core.dart';

enum BootImageCheckFailure { core, read }

class BootImageCheck {
  const BootImageCheck.ok(this.info) : failure = null;
  const BootImageCheck.fail(this.failure) : info = null;

  final BootImageInfo? info;
  final BootImageCheckFailure? failure;
}

class BootImageReader {
  const BootImageReader();

  Future<BootImageCheck> info(String path) async {
    if (!rustCore.ready) {
      return const BootImageCheck.fail(BootImageCheckFailure.core);
    }
    try {
      return BootImageCheck.ok(await bootImageInfo(path: path));
    } catch (_) {
      return const BootImageCheck.fail(BootImageCheckFailure.read);
    }
  }
}
