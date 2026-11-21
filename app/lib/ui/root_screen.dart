import 'package:file_selector/file_selector.dart';
import 'package:intl/intl.dart';
import 'package:material_ui/material_ui.dart';

import '../l10n/generated/app_localizations.dart';
import '../services/boot_image.dart';
import '../services/device_output.dart';
import '../services/device_repository.dart';

enum RootMethod { magisk, kernelsu, apatch }

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
  bool _flashing = false;
  String? _flashOutput;
  String? _flashError;

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

  Future<void> _flashBoot() async {
    final serial = _selectedSerial;
    final path = _imagePath;
    if (serial == null || path == null) return;
    final strings = AppLocalizations.of(context)!;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text(strings.flashConfirmTitle),
        content: Text(strings.flashConfirmBody(serial)),
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
            child: Text(strings.flashBootButton),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    setState(() {
      _flashing = true;
      _flashOutput = null;
      _flashError = null;
    });
    try {
      final output =
          await widget.repository.fastbootFlash(serial, 'boot', path);
      if (!mounted) return;
      setState(() {
        _flashing = false;
        _flashOutput = output;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _flashing = false;
        _flashError = e.toString();
      });
    }
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
            if (_check?.info == null) ...[
              const SizedBox(height: 12),
              Text(
                strings.flashPickFirst,
                style: Theme.of(context).textTheme.bodyMedium,
              ),
            ],
            const SizedBox(height: 16),
            FilledButton.icon(
              style: FilledButton.styleFrom(
                backgroundColor: scheme.error,
                foregroundColor: scheme.onError,
              ),
              onPressed: _canFlash() ? _flashBoot : null,
              icon: const Icon(Icons.flash_on),
              label: Text(strings.flashBootButton),
            ),
            if (_flashing) ...[
              const SizedBox(height: 16),
              const LinearProgressIndicator(),
              const SizedBox(height: 8),
              Text(strings.flashRunning),
            ],
            if (_flashOutput != null) ...[
              const SizedBox(height: 16),
              Text(
                strings.flashDone,
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
              Text(
                body,
                style: TextStyle(color: scheme.onErrorContainer),
              ),
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
                _infoRow(context, strings.infoHeaderVersion,
                    'v${info.headerVersion}'),
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
