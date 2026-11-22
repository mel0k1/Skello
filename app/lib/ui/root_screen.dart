import 'package:file_selector/file_selector.dart';
import 'package:intl/intl.dart';
import 'package:material_ui/material_ui.dart';

import '../l10n/generated/app_localizations.dart';
import '../services/boot_image.dart';
import '../services/device_output.dart';
import '../services/device_repository.dart';
import '../services/flash_errors.dart';

enum RootMethod { magisk, kernelsu, apatch }

enum RebootCheck { none, waiting, back, timeout }

const flashPartitions = ['boot', 'recovery', 'vendor_boot'];

class RootScreen extends StatefulWidget {
  const RootScreen({
    super.key,
    this.reader = const BootImageReader(),
    this.repository = const DeviceRepository(),
  });

  final BootImageReader reader;
  final DeviceRepository repository;

  @override
  State<RootScreen> createState() => _RootScreenState();
}

class _RootScreenState extends State<RootScreen> {
  RootMethod _method = RootMethod.magisk;
  BootImageCheck? _check;
  bool _checking = false;
  String? _imagePath;
  List<FastbootDevice> _devices = const [];
  String? _selectedSerial;
  bool? _unlocked;
  String _partition = 'boot';
  FlashProgressEvent? _progress;
  bool _flashing = false;
  String? _flashOutput;
  String? _flashError;
  bool _lastActionErase = false;
  RebootCheck _rebootCheck = RebootCheck.none;

  @override
  void initState() {
    super.initState();
    _loadDevices();
  }

  Future<void> _loadDevices() async {
    try {
      final devices = await widget.repository.fastbootDevices();
      if (!mounted) return;
      setState(() {
        _devices = devices;
        if (devices.isNotEmpty &&
            !devices.any((d) => d.serial == _selectedSerial)) {
          _selectedSerial = devices.first.serial;
          _unlocked = null;
          _checkUnlocked();
        }
      });
    } catch (_) {
      if (!mounted) return;
      setState(() => _devices = const []);
    }
  }

  Future<void> _checkUnlocked() async {
    final serial = _selectedSerial;
    if (serial == null) return;
    try {
      final value = await widget.repository.fastbootVar(serial, 'unlocked');
      if (!mounted) return;
      setState(() => _unlocked = value == null ? null : value == 'yes');
    } catch (_) {
      if (!mounted) return;
      setState(() => _unlocked = null);
    }
  }

  bool _canFlash() {
    return _selectedSerial != null &&
        _imagePath != null &&
        _unlocked == true &&
        !_flashing;
  }

  bool _canErase() {
    return _selectedSerial != null && _unlocked == true && !_flashing;
  }

  Future<bool> _confirmDestructive(
    AppLocalizations strings,
    String title,
    String body,
    String confirmLabel,
  ) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text(title),
        content: Text(body),
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
            child: Text(confirmLabel),
          ),
        ],
      ),
    );
    return confirmed == true;
  }

  Future<void> _flashPartition() async {
    final serial = _selectedSerial;
    final path = _imagePath;
    if (serial == null || path == null) return;
    final strings = AppLocalizations.of(context)!;
    final confirmed = await _confirmDestructive(
      strings,
      strings.flashConfirmTitle,
      strings.flashConfirmBody(_partition, serial),
      strings.flashPartitionButton(_partition),
    );
    if (confirmed != true) return;
    setState(() {
      _flashing = true;
      _progress = null;
      _flashOutput = null;
      _flashError = null;
      _lastActionErase = false;
      _rebootCheck = RebootCheck.none;
    });
    try {
      final output = await widget.repository.flashPartition(
        serial,
        _partition,
        path,
        (event) {
          if (!mounted) return;
          setState(() => _progress = event);
        },
      );
      if (!mounted) return;
      setState(() {
        _flashing = false;
        _flashOutput = output;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _flashing = false;
        _flashError = _flashErrorText(e, strings);
      });
    }
  }

  Future<void> _erasePartition() async {
    final serial = _selectedSerial;
    if (serial == null) return;
    final strings = AppLocalizations.of(context)!;
    final confirmed = await _confirmDestructive(
      strings,
      strings.eraseConfirmTitle,
      strings.eraseConfirmBody(_partition, serial),
      strings.erasePartitionButton(_partition),
    );
    if (confirmed != true) return;
    setState(() {
      _flashing = true;
      _flashOutput = null;
      _flashError = null;
      _lastActionErase = true;
      _rebootCheck = RebootCheck.none;
    });
    try {
      final output = await widget.repository.fastbootErase(serial, _partition);
      if (!mounted) return;
      setState(() {
        _flashing = false;
        _flashOutput = output;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _flashing = false;
        _flashError = _flashErrorText(e, strings);
      });
    }
  }

  Future<void> _rebootAndCheck() async {
    final serial = _selectedSerial;
    if (serial == null) return;
    setState(() => _rebootCheck = RebootCheck.waiting);
    try {
      await widget.repository.fastbootReboot(serial);
    } catch (_) {}
    final back = await widget.repository.waitForAdbDevice(serial);
    if (!mounted) return;
    setState(
      () => _rebootCheck = back ? RebootCheck.back : RebootCheck.timeout,
    );
  }

  String _flashErrorText(Object error, AppLocalizations strings) {
    if (error is BootloaderLocked) return strings.flashLocked;
    if (error is BatteryNotReady) return strings.flashBatteryError;
    if (error is DownloadTooBig) {
      final format = NumberFormat.decimalPattern();
      return strings.flashSizeError(
        format.format(error.maxSize),
        format.format(error.imageSize),
      );
    }
    return error.toString();
  }

  String _flashPhaseText(AppLocalizations strings) {
    final progress = _progress;
    if (progress == null) return strings.flashRunning;
    if (progress.phase == 'flash') return strings.flashPhaseFlash;
    final percent =
        progress.total > 0 ? (progress.sent * 100 / progress.total).round() : 0;
    return '${strings.flashPhaseDownload} $percent%';
  }

  Future<void> _pickImage() async {
    final strings = AppLocalizations.of(context)!;
    final file = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(
          label: strings.bootImageTypeLabel,
          extensions: const ['img'],
        ),
      ],
    );
    if (file == null) return;
    if (!mounted) return;
    setState(() {
      _checking = true;
      _check = null;
      _imagePath = file.path;
      _flashOutput = null;
      _flashError = null;
    });
    final check = await widget.reader.info(file.path);
    if (!mounted) return;
    setState(() {
      _checking = false;
      _check = check;
    });
  }

  String _methodTitle(AppLocalizations strings) {
    return switch (_method) {
      RootMethod.magisk => 'Magisk',
      RootMethod.kernelsu => 'KernelSU',
      RootMethod.apatch => 'APatch',
    };
  }

  String _guide(AppLocalizations strings) {
    return switch (_method) {
      RootMethod.magisk => strings.magiskGuide,
      RootMethod.kernelsu => strings.kernelsuGuide,
      RootMethod.apatch => strings.apatchGuide,
    };
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context)!;
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      appBar: AppBar(title: Text(strings.rootTitle)),
      body: LayoutBuilder(
        builder: (context, constraints) {
          final side = ((constraints.maxWidth - 560) / 2)
              .clamp(16.0, double.infinity)
              .toDouble();
          return ListView(
            padding: EdgeInsets.fromLTRB(side, 24, side, 24),
            children: [
              Text(
                strings.rootIntro,
                style: Theme.of(context).textTheme.bodyLarge,
              ),
              const SizedBox(height: 16),
              RadioGroup<RootMethod>(
                groupValue: _method,
                onChanged: (value) => setState(() => _method = value!),
                child: Column(
                  children: [
                    _methodCard(
                      RootMethod.magisk,
                      'Magisk',
                      strings.methodMagiskSubtitle,
                    ),
                    _methodCard(
                      RootMethod.kernelsu,
                      'KernelSU',
                      strings.methodKernelSuSubtitle,
                    ),
                    _methodCard(
                      RootMethod.apatch,
                      'APatch',
                      strings.methodApatchSubtitle,
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
              Text(
                strings.bootSectionTitle,
                style: Theme.of(context).textTheme.titleLarge,
              ),
              const SizedBox(height: 12),
              FilledButton.tonalIcon(
                onPressed: _checking ? null : _pickImage,
                icon: const Icon(Icons.file_open_outlined),
                label: Text(strings.pickBootImage),
              ),
              if (_checking)
                const Padding(
                  padding: EdgeInsets.only(top: 16),
                  child: LinearProgressIndicator(),
                ),
              if (_check != null) ...[
                const SizedBox(height: 16),
                BootImageCheckView(check: _check!),
              ],
              const SizedBox(height: 24),
              Text(
                strings.flashSectionTitle,
                style: Theme.of(context).textTheme.titleLarge,
              ),
              const SizedBox(height: 12),
              _flashSection(strings, scheme),
              const SizedBox(height: 16),
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        _methodTitle(strings),
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      const SizedBox(height: 8),
                      Text(
                        _guide(strings),
                        style: const TextStyle(height: 1.5),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 16),
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
                            strings.warningTitle,
                            style: TextStyle(
                              color: scheme.onErrorContainer,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 8),
                      Text(
                        strings.warningBody,
                        style: TextStyle(color: scheme.onErrorContainer),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  Widget _methodCard(RootMethod method, String title, String subtitle) {
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: RadioListTile<RootMethod>(
        value: method,
        title: Text(title),
        subtitle: Text(subtitle),
      ),
    );
  }

  Widget _flashSection(AppLocalizations strings, ColorScheme scheme) {
    if (_devices.isEmpty) {
      return Card(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Text(strings.flashNoDevice),
        ),
      );
    }
    final IconData unlockedIcon = switch (_unlocked) {
      true => Icons.lock_open,
      false => Icons.lock,
      null => Icons.help_outline,
    };
    final Color unlockedColor = switch (_unlocked) {
      true => scheme.primary,
      false => scheme.error,
      null => scheme.onSurfaceVariant,
    };
    final String unlockedText = switch (_unlocked) {
      true => strings.flashUnlocked,
      false => strings.flashLocked,
      null => strings.flashStateUnknown,
    };
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              strings.flashDeviceLabel,
              style: Theme.of(context).textTheme.labelMedium,
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: Container(
                    padding: const EdgeInsets.symmetric(horizontal: 12),
                    decoration: BoxDecoration(
                      border: Border.all(color: scheme.outline),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: DropdownButton<String>(
                      value: _selectedSerial,
                      isExpanded: true,
                      underline: const SizedBox.shrink(),
                      items: [
                        for (final device in _devices)
                          DropdownMenuItem(
                            value: device.serial,
                            child: Text(device.serial),
                          ),
                      ],
                      onChanged: (value) {
                        setState(() {
                          _selectedSerial = value;
                          _unlocked = null;
                        });
                        _checkUnlocked();
                      },
                    ),
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.refresh),
                  tooltip: strings.refreshTooltip,
                  onPressed: _loadDevices,
                ),
              ],
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Icon(unlockedIcon, color: unlockedColor, size: 20),
                const SizedBox(width: 8),
                Expanded(child: Text(unlockedText)),
              ],
            ),
            const SizedBox(height: 12),
            Text(
              strings.flashPartitionLabel,
              style: Theme.of(context).textTheme.labelMedium,
            ),
            const SizedBox(height: 8),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              decoration: BoxDecoration(
                border: Border.all(color: scheme.outline),
                borderRadius: BorderRadius.circular(8),
              ),
              child: DropdownButton<String>(
                value: _partition,
                isExpanded: true,
                underline: const SizedBox.shrink(),
                items: [
                  for (final partition in flashPartitions)
                    DropdownMenuItem(value: partition, child: Text(partition)),
                ],
                onChanged: _flashing
                    ? null
                    : (value) {
                        setState(() {
                          _partition = value!;
                          _flashOutput = null;
                          _flashError = null;
                        });
                      },
              ),
            ),
            if (_check?.info == null) ...[
              const SizedBox(height: 12),
              Text(
                strings.flashPickFirst,
                style: Theme.of(context).textTheme.bodyMedium,
              ),
            ],
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  child: FilledButton.icon(
                    style: FilledButton.styleFrom(
                      backgroundColor: scheme.error,
                      foregroundColor: scheme.onError,
                    ),
                    onPressed: _canFlash() ? _flashPartition : null,
                    icon: const Icon(Icons.flash_on),
                    label: Text(strings.flashPartitionButton(_partition)),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: OutlinedButton.icon(
                    style: OutlinedButton.styleFrom(
                      foregroundColor: scheme.error,
                      side: BorderSide(color: scheme.error),
                    ),
                    onPressed: _canErase() ? _erasePartition : null,
                    icon: const Icon(Icons.delete_outline),
                    label: Text(strings.erasePartitionButton(_partition)),
                  ),
                ),
              ],
            ),
            if (_flashing) ...[
              const SizedBox(height: 16),
              if (_progress != null && _progress!.total > 0)
                LinearProgressIndicator(
                  value: _progress!.sent / _progress!.total,
                )
              else
                const LinearProgressIndicator(),
              const SizedBox(height: 8),
              Text(_flashPhaseText(strings)),
            ],
            if (_flashOutput != null) ...[
              const SizedBox(height: 16),
              Text(
                _lastActionErase ? strings.eraseDone : strings.flashDone,
                style: TextStyle(
                  color: scheme.primary,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                strings.flashOutput,
                style: Theme.of(context).textTheme.labelSmall,
              ),
              const SizedBox(height: 4),
              Text(
                _flashOutput!,
                style: const TextStyle(fontSize: 12, height: 1.3),
              ),
            ],
            if (_flashError != null) ...[
              const SizedBox(height: 16),
              Text(
                strings.flashFailed,
                style: TextStyle(
                  color: scheme.error,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                _flashError!,
                style: TextStyle(color: scheme.error, fontSize: 12),
              ),
            ],
            if (_unlocked == true && !_flashing) ...[
              const SizedBox(height: 16),
              OutlinedButton.icon(
                onPressed: _rebootCheck == RebootCheck.waiting
                    ? null
                    : _rebootAndCheck,
                icon: const Icon(Icons.restart_alt),
                label: Text(strings.rebootCheckButton),
              ),
              if (_rebootCheck == RebootCheck.waiting) ...[
                const SizedBox(height: 8),
                Row(
                  children: [
                    const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                    const SizedBox(width: 8),
                    Expanded(child: Text(strings.rebootWaiting)),
                  ],
                ),
              ],
              if (_rebootCheck == RebootCheck.back) ...[
                const SizedBox(height: 8),
                Text(
                  strings.rebootBack,
                  style: TextStyle(color: scheme.primary),
                ),
              ],
              if (_rebootCheck == RebootCheck.timeout) ...[
                const SizedBox(height: 8),
                Text(
                  strings.rebootTimeout,
                  style: TextStyle(color: scheme.error),
                ),
              ],
            ],
          ],
        ),
      ),
    );
  }
}

class BootImageCheckView extends StatelessWidget {
  const BootImageCheckView({super.key, required this.check});

  final BootImageCheck check;

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context)!;
    final scheme = Theme.of(context).colorScheme;
    final info = check.info;
    if (info == null) {
      final title = check.failure == BootImageCheckFailure.core
          ? strings.sourcesUnavailableTitle
          : strings.checkErrorTitle;
      final body = check.failure == BootImageCheckFailure.core
          ? strings.sourcesUnavailableBody
          : strings.invalidBootImage;
      return Card(
        color: scheme.errorContainer,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                title,
                style: TextStyle(
                  color: scheme.onErrorContainer,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              Text(body, style: TextStyle(color: scheme.onErrorContainer)),
            ],
          ),
        ),
      );
    }
    final format = NumberFormat.decimalPattern();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Icon(Icons.check_circle_outline, color: scheme.primary),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                strings.validBootImage,
                style: TextStyle(
                  color: scheme.primary,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                _infoRow(
                  context,
                  strings.infoHeaderVersion,
                  'v${info.headerVersion}',
                ),
                _infoRow(
                  context,
                  strings.infoPageSize,
                  info.pageSize == null
                      ? '—'
                      : '${format.format(info.pageSize!)} B',
                ),
                _infoRow(
                  context,
                  strings.infoKernelSize,
                  '${format.format(info.kernelSize)} B',
                ),
                _infoRow(
                  context,
                  strings.infoRamdiskSize,
                  '${format.format(info.ramdiskSize)} B',
                ),
                _infoRow(context, strings.infoOsVersion, info.osVersion ?? '—'),
                _infoRow(
                  context,
                  strings.infoOsPatchLevel,
                  info.osPatchLevel ?? '—',
                ),
                _infoRow(
                  context,
                  strings.infoFileSize,
                  '${format.format(info.fileSize.toInt())} B',
                ),
                _infoRow(
                  context,
                  strings.infoAvbFooter,
                  info.hasAvbFooter ? strings.avbPresent : strings.avbAbsent,
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }

  Widget _infoRow(BuildContext context, String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          Expanded(
            child: Text(label, style: Theme.of(context).textTheme.bodyMedium),
          ),
          Text(value),
        ],
      ),
    );
  }
}
