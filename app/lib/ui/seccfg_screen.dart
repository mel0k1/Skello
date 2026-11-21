import 'package:material_ui/material_ui.dart';

import '../l10n/generated/app_localizations.dart';
import '../services/device_repository.dart';

/// MediaTek seccfg screen: reads the lock state over adb (root) and
/// patches software-SEJ partitions to unlocked.
class SeccfgScreen extends StatefulWidget {
  const SeccfgScreen({
    super.key,
    required this.serial,
    required this.repository,
  });

  final String serial;
  final DeviceRepository repository;

  @override
  State<SeccfgScreen> createState() => _SeccfgScreenState();
}

class _SeccfgScreenState extends State<SeccfgScreen> {
  SeccfgStatus? _status;
  String? _error;
  String? _result;
  bool _busy = true;

  @override
  void initState() {
    super.initState();
    _loadStatus();
  }

  Future<void> _loadStatus() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final status =
          await widget.repository.mediatekSeccfgStatus(widget.serial);
      if (!mounted) return;
      setState(() {
        _status = status;
        _busy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busy = false;
      });
    }
  }

  Future<void> _unlock() async {
    final strings = AppLocalizations.of(context)!;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text(strings.seccfgUnlockConfirmTitle),
        content: Text(strings.seccfgUnlockConfirmBody(widget.serial)),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: Text(strings.cancelLabel),
          ),
          FilledButton(
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(dialogContext).colorScheme.error,
              foregroundColor: Theme.of(dialogContext).colorScheme.onError,
            ),
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: Text(strings.seccfgUnlockButton),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    setState(() {
      _busy = true;
      _result = null;
      _error = null;
    });
    try {
      final message =
          await widget.repository.mediatekSeccfgUnlock(widget.serial);
      if (!mounted) return;
      setState(() {
        _result = message;
        _busy = false;
      });
      await _loadStatus();
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _busy = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context)!;
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      appBar: AppBar(title: Text('${strings.seccfgTitle} ${widget.serial}')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          if (_busy) ...[
            const Center(child: CircularProgressIndicator()),
          ] else ...[
            if (_error != null) ...[
              Text(
                strings.seccfgFailed,
                style:
                    TextStyle(color: scheme.error, fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 8),
              SelectableText(
                _error!,
                style: TextStyle(color: scheme.error, fontSize: 12),
              ),
              const SizedBox(height: 8),
              Text(
                _error!.contains('root')
                    ? strings.seccfgRootHint
                    : strings.seccfgNeedsSejHint,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ] else if (_status != null) ...[
              Text(
                '${strings.seccfgStateLabel}: ${_status!.state}',
                style: const TextStyle(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 4),
              Text('${strings.seccfgVersionLabel}: ${_status!.version}'),
              const SizedBox(height: 4),
              Text(
                '${strings.seccfgDetailLabel}: ${_status!.detail}',
                style: Theme.of(context).textTheme.bodySmall,
              ),
              const SizedBox(height: 16),
              FilledButton.icon(
                style: FilledButton.styleFrom(
                  backgroundColor: scheme.error,
                  foregroundColor: scheme.onError,
                ),
                onPressed: _status!.state == 'unlocked' ? null : _unlock,
                icon: const Icon(Icons.lock_open),
                label: Text(strings.seccfgUnlockButton),
              ),
            ],
            if (_result != null) ...[
              const SizedBox(height: 16),
              Text(
                strings.seccfgUnlockDone,
                style: TextStyle(
                    color: scheme.primary, fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 8),
              SelectableText(
                _result!,
                style: const TextStyle(fontSize: 12),
              ),
            ],
            const SizedBox(height: 16),
            Text(
              strings.seccfgNeedsSejHint,
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
        ],
      ),
    );
  }
}
