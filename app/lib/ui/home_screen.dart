import 'package:material_ui/material_ui.dart';

import '../l10n/generated/app_localizations.dart';
import '../services/device_output.dart';
import '../services/device_repository.dart';
import 'chipsets_screen.dart';
import 'root_screen.dart';
import 'seccfg_screen.dart';
import 'shell_screen.dart';
import 'unlock_screen.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key, this.repository = const DeviceRepository()});

  final DeviceRepository repository;

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  List<DeviceRow> _rows = const [];
  bool _busy = true;
  bool _failed = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _busy = true;
      _failed = false;
    });
    try {
      final adbRows = await widget.repository.adbDevices();
      final fastbootRows = await widget.repository.fastbootDevices();
      if (!mounted) return;
      setState(() {
        _rows = [
          ...adbRows.map((d) => DeviceRow(d.serial, d.state, 'adb')),
          ...fastbootRows.map((d) => DeviceRow(d.serial, d.state, 'fastboot')),
        ];
        _busy = false;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _rows = const [];
        _failed = true;
        _busy = false;
      });
    }
  }

  void _openRoot() {
    Navigator.of(context)
        .push(MaterialPageRoute<void>(builder: (_) => const RootScreen()));
  }

  void _openChipsets() {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => ChipsetsScreen(repository: widget.repository),
      ),
    );
  }

  void _openUnlock(DeviceRow row) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) =>
            UnlockScreen(serial: row.serial, repository: widget.repository),
      ),
    );
  }

  void _openShell(DeviceRow row) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) =>
            ShellScreen(serial: row.serial, repository: widget.repository),
      ),
    );
  }

  void _openSeccfg(DeviceRow row) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) =>
            SeccfgScreen(serial: row.serial, repository: widget.repository),
      ),
    );
  }

  Future<void> _reboot(DeviceRow row, String target) async {
    final strings = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);
    try {
      if (row.source == 'adb') {
        await widget.repository.adbReboot(row.serial, target);
      } else {
        await widget.repository.fastbootReboot(row.serial);
      }
      if (!mounted) return;
      messenger.showSnackBar(SnackBar(content: Text(strings.actionStarted)));
      await Future<void>.delayed(const Duration(seconds: 2));
      if (mounted) _load();
    } catch (e) {
      if (!mounted) return;
      messenger.showSnackBar(
        SnackBar(content: Text('${strings.actionFailed}: $e')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(
        title: Text(strings.appName),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            tooltip: strings.refreshTooltip,
            onPressed: _busy ? null : _load,
          ),
          IconButton(
            icon: const Icon(Icons.security_outlined),
            tooltip: strings.rootTooltip,
            onPressed: _openRoot,
          ),
          IconButton(
            icon: const Icon(Icons.memory),
            tooltip: strings.chipsetsTooltip,
            onPressed: _openChipsets,
          ),
        ],
      ),
      body: LayoutBuilder(
        builder: (context, constraints) {
          final side = ((constraints.maxWidth - 520) / 2)
              .clamp(16.0, double.infinity)
              .toDouble();
          if (_busy) return _busyView(strings);
          if (_failed) return _failureView(context, strings);
          if (_rows.isEmpty) return _emptyView(context, strings);
          return ListView(
            padding: EdgeInsets.fromLTRB(side, 24, side, 24),
            children: [
              for (final row in _rows)
                _DeviceCard(
                  row: row,
                  onReboot: _reboot,
                  onUnlock: _openUnlock,
                  onShell: _openShell,
                  onSeccfg: _openSeccfg,
                ),
            ],
          );
        },
      ),
    );
  }

  Widget _busyView(AppLocalizations strings) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const CircularProgressIndicator(),
              const SizedBox(height: 16),
              Text(strings.loadingDevices),
            ],
          ),
        ),
      ),
    );
  }

  Widget _emptyView(BuildContext context, AppLocalizations strings) {
    final scheme = Theme.of(context).colorScheme;
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Icon(Icons.phone_android, size: 64, color: scheme.primary),
              const SizedBox(height: 24),
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(24),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        strings.deviceNotConnectedTitle,
                        style: Theme.of(context).textTheme.titleLarge,
                      ),
                      const SizedBox(height: 12),
                      Text(
                        strings.deviceNotConnectedBody,
                        style: Theme.of(context).textTheme.bodyMedium,
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _failureView(BuildContext context, AppLocalizations strings) {
    final scheme = Theme.of(context).colorScheme;
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Icon(Icons.error_outline, size: 64, color: scheme.error),
              const SizedBox(height: 24),
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(24),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        strings.sourcesUnavailableTitle,
                        style: Theme.of(context).textTheme.titleLarge,
                      ),
                      const SizedBox(height: 12),
                      Text(
                        strings.sourcesUnavailableBody,
                        style: Theme.of(context).textTheme.bodyMedium,
                      ),
                      const SizedBox(height: 16),
                      FilledButton.tonal(
                        onPressed: _load,
                        child: Text(strings.retryLabel),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _DeviceCard extends StatelessWidget {
  const _DeviceCard({
    required this.row,
    required this.onReboot,
    required this.onUnlock,
    required this.onShell,
    required this.onSeccfg,
  });

  final DeviceRow row;
  final void Function(DeviceRow row, String target) onReboot;
  final void Function(DeviceRow row) onUnlock;
  final void Function(DeviceRow row) onShell;
  final void Function(DeviceRow row) onSeccfg;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final strings = AppLocalizations.of(context)!;
    final Color container;
    final Color onContainer;
    final IconData icon;
    if (row.isActive) {
      container = scheme.primaryContainer;
      onContainer = scheme.onPrimaryContainer;
      icon = Icons.smartphone;
    } else if (row.isProblem) {
      container = scheme.errorContainer;
      onContainer = scheme.onErrorContainer;
      icon = Icons.warning_amber_rounded;
    } else {
      container = scheme.tertiaryContainer;
      onContainer = scheme.onTertiaryContainer;
      icon = Icons.restart_alt;
    }
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: ListTile(
        leading: Icon(icon, color: onContainer),
        title: Text(row.serial),
        subtitle: Text(row.source),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Chip(
              label: Text(row.state),
              labelStyle: TextStyle(color: onContainer),
              backgroundColor: container,
              visualDensity: VisualDensity.compact,
              side: BorderSide.none,
            ),
            if (row.isActive)
              PopupMenuButton<String>(
                icon: const Icon(Icons.more_vert),
                tooltip: strings.actionsTooltip,
                onSelected: (value) {
                  if (value == 'unlock') {
                    onUnlock(row);
                  } else if (value == 'shell') {
                    onShell(row);
                  } else if (value == 'seccfg') {
                    onSeccfg(row);
                  } else {
                    onReboot(row, value == 'reboot' ? '' : value);
                  }
                },
                itemBuilder: (_) => [
                  if (row.source == 'adb') ...[
                    PopupMenuItem(
                      value: '',
                      child: Text(strings.rebootMenuSystem),
                    ),
                    PopupMenuItem(
                      value: 'bootloader',
                      child: Text(strings.rebootMenuBootloader),
                    ),
                    PopupMenuItem(
                      value: 'recovery',
                      child: Text(strings.rebootMenuRecovery),
                    ),
                    PopupMenuItem(
                      value: 'shell',
                      child: Text(strings.shellMenu),
                    ),
                    PopupMenuItem(
                      value: 'seccfg',
                      child: Text(strings.mediatekMenu),
                    ),
                  ],
                  if (row.source == 'fastboot') ...[
                    PopupMenuItem(
                      value: 'reboot',
                      child: Text(strings.rebootMenuSystem),
                    ),
                    PopupMenuItem(
                      value: 'unlock',
                      child: Text(strings.unlockMenu),
                    ),
                  ],
                ],
              ),
          ],
        ),
      ),
    );
  }
}
