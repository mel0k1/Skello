import 'package:material_ui/material_ui.dart';

import '../l10n/generated/app_localizations.dart';
import '../services/device_repository.dart';

class UnlockScreen extends StatefulWidget {
  const UnlockScreen({
    super.key,
    required this.serial,
    this.repository = const DeviceRepository(),
  });

  final String serial;
  final DeviceRepository repository;

  @override
  State<UnlockScreen> createState() => _UnlockScreenState();
}

class _UnlockScreenState extends State<UnlockScreen> {
  bool? _unlocked;
  bool _checking = true;
  bool _unlocking = false;
  bool _confirmed = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _check();
  }

  Future<void> _check() async {
    setState(() {
      _checking = true;
      _error = null;
    });
    try {
      final value = await widget.repository.fastbootVar(
        widget.serial,
        'unlocked',
      );
      if (!mounted) return;
      setState(() {
        _unlocked = value == null ? null : value == 'yes';
        _checking = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _unlocked = null;
        _checking = false;
      });
    }
  }

  Future<void> _runUnlock() async {
    setState(() {
      _unlocking = true;
      _error = null;
    });
    try {
      await widget.repository.fastbootUnlock(widget.serial);
      final value = await widget.repository.fastbootVar(
        widget.serial,
        'unlocked',
      );
      if (!mounted) return;
      setState(() {
        _unlocking = false;
        _unlocked = value == null ? null : value == 'yes';
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _unlocking = false;
        _error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context)!;
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      appBar: AppBar(title: Text(strings.unlockTitle)),
      body: ListView(
        padding: EdgeInsets.fromLTRB(
          ((MediaQuery.sizeOf(context).width - 520) / 2)
              .clamp(16.0, double.infinity)
              .toDouble(),
          24,
          ((MediaQuery.sizeOf(context).width - 520) / 2)
              .clamp(16.0, double.infinity)
              .toDouble(),
          24,
        ),
        children: [
          Card(
            color: scheme.errorContainer,
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        Icons.warning_amber_rounded,
                        color: scheme.onErrorContainer,
                      ),
                      const SizedBox(width: 8),
                      Text(
                        strings.unlockWarningTitle,
                        style: TextStyle(
                          color: scheme.onErrorContainer,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    strings.unlockWarningBody,
                    style: TextStyle(color: scheme.onErrorContainer),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    '${strings.unlockDevice}: ${widget.serial}',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    strings.unlockVendorNote,
                    style: Theme.of(context).textTheme.bodyMedium,
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),
          _statusCard(context, strings, scheme),
        ],
      ),
    );
  }

  Widget _statusCard(
    BuildContext context,
    AppLocalizations strings,
    ColorScheme scheme,
  ) {
    if (_checking) {
      return Card(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            children: [
              const CircularProgressIndicator(),
              const SizedBox(height: 16),
              Text(strings.unlockChecking),
            ],
          ),
        ),
      );
    }
    if (_error != null) {
      return Card(
        color: scheme.errorContainer,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                strings.unlockFailed,
                style: TextStyle(
                  color: scheme.onErrorContainer,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                _error!,
                style: TextStyle(color: scheme.onErrorContainer),
              ),
              const SizedBox(height: 12),
              FilledButton.tonal(
                onPressed: _check,
                child: Text(strings.retryLabel),
              ),
            ],
          ),
        ),
      );
    }
    if (_unlocked == true) {
      return Card(
        color: scheme.primaryContainer,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Icon(Icons.lock_open, color: scheme.onPrimaryContainer),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  strings.unlockAlready,
                  style: TextStyle(
                    color: scheme.onPrimaryContainer,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ],
          ),
        ),
      );
    }
    if (_unlocked == null) {
      return Card(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(strings.unlockUnknown),
              const SizedBox(height: 12),
              FilledButton.tonal(
                onPressed: _check,
                child: Text(strings.retryLabel),
              ),
            ],
          ),
        ),
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(Icons.lock, color: scheme.error),
                    const SizedBox(width: 12),
                    Text(
                      strings.unlockLocked,
                      style: const TextStyle(fontWeight: FontWeight.w600),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Text(
                  strings.unlockSteps,
                  style: const TextStyle(height: 1.5),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 12),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                CheckboxListTile(
                  value: _confirmed,
                  onChanged: (value) =>
                      setState(() => _confirmed = value ?? false),
                  title: Text(strings.unlockConfirm),
                  controlAffinity: ListTileControlAffinity.leading,
                  contentPadding: EdgeInsets.zero,
                ),
                const SizedBox(height: 8),
                FilledButton.icon(
                  onPressed: _confirmed && !_unlocking ? _runUnlock : null,
                  icon: const Icon(Icons.lock_open),
                  label: Text(strings.unlockRun),
                ),
              ],
            ),
          ),
        ),
        if (_unlocking) ...[
          const SizedBox(height: 12),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                children: [
                  const CircularProgressIndicator(),
                  const SizedBox(height: 16),
                  Text(strings.unlockWaiting),
                ],
              ),
            ),
          ),
        ],
      ],
    );
  }
}
