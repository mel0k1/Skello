import 'package:file_selector/file_selector.dart';
import 'package:intl/intl.dart';
import 'package:material_ui/material_ui.dart';

import '../l10n/generated/app_localizations.dart';
import '../services/boot_image.dart';

enum RootMethod { magisk, kernelsu, apatch }

class RootScreen extends StatefulWidget {
  const RootScreen({super.key, this.reader = const BootImageReader()});

  final BootImageReader reader;

  @override
  State<RootScreen> createState() => _RootScreenState();
}

class _RootScreenState extends State<RootScreen> {
  RootMethod _method = RootMethod.magisk;
  BootImageCheck? _check;
  bool _checking = false;

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
