import 'package:material_ui/material_ui.dart';

import '../l10n/generated/app_localizations.dart';
import '../services/device_repository.dart';

/// A minimal adb shell terminal over the USB transport.
class ShellScreen extends StatefulWidget {
  const ShellScreen({
    super.key,
    required this.serial,
    required this.repository,
  });

  final String serial;
  final DeviceRepository repository;

  @override
  State<ShellScreen> createState() => _ShellScreenState();
}

class _ShellScreenState extends State<ShellScreen> {
  final _controller = TextEditingController();
  final _entries = <String>[];
  bool _running = false;
  final _scroll = ScrollController();

  Future<void> _send() async {
    final command = _controller.text.trim();
    final strings = AppLocalizations.of(context)!;
    if (command.isEmpty || _running) return;
    setState(() {
      _running = true;
      _entries.add('> $command');
    });
    _controller.clear();
    try {
      final output = await widget.repository.adbShell(widget.serial, command);
      if (!mounted) return;
      setState(() => _entries.add(output.isEmpty ? '(no output)' : output));
    } catch (e) {
      if (!mounted) return;
      setState(() => _entries.add('${strings.shellFailed}: $e'));
    } finally {
      if (mounted) {
        setState(() => _running = false);
        await Future<void>.delayed(const Duration(milliseconds: 50));
        if (_scroll.hasClients) {
          _scroll.jumpTo(_scroll.position.maxScrollExtent);
        }
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(title: Text('${strings.shellTitle} ${widget.serial}')),
      body: Column(
        children: [
          Expanded(
            child: ListView.builder(
              controller: _scroll,
              padding: const EdgeInsets.all(12),
              itemCount: _entries.length,
              itemBuilder: (_, index) => SelectableText(
                _entries[index],
                style: const TextStyle(
                  fontFamily: 'monospace',
                  fontSize: 13,
                  height: 1.3,
                ),
              ),
            ),
          ),
          SafeArea(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _controller,
                      enabled: !_running,
                      onSubmitted: (_) => _send(),
                      decoration: InputDecoration(
                        hintText: strings.shellHint,
                        border: const OutlineInputBorder(),
                        isDense: true,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  IconButton.filled(
                    onPressed: _running ? null : _send,
                    icon: _running
                        ? const SizedBox(
                            width: 18,
                            height: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.send),
                    tooltip: strings.shellSend,
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
