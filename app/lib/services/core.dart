import 'package:skello/src/rust/frb_generated.dart';

class RustCore {
  bool ready = false;

  Future<void> init() async {
    try {
      await RustLib.init();
      ready = true;
    } catch (_) {
      ready = false;
    }
  }
}

final rustCore = RustCore();
