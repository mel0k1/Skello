import 'package:file_selector/file_selector.dart';
import 'package:material_ui/material_ui.dart';

import '../l10n/generated/app_localizations.dart';
import '../services/device_output.dart';
import '../services/device_repository.dart';

/// Vendor-mode screen: lists the MediaTek BROM, Unisoc BROM and
/// Qualcomm EDL USB ports and runs the stage-one wizards over raw
/// USB - the DA unlock for MediaTek, FDL flashing for Unisoc and
/// the Sahara + Firehose run for Qualcomm.
class ChipsetsScreen extends StatefulWidget {
  const ChipsetsScreen({super.key, required this.repository});

  final DeviceRepository repository;

  @override
  State<ChipsetsScreen> createState() => _ChipsetsScreenState();
}

class _ChipsetsScreenState extends State<ChipsetsScreen> {
  ChipsetPortGroups? _ports;
  String? _loadError;

  String? _loaderPath;
  bool _mtkBusy = false;
  String? _mtkResult;
  String? _mtkError;
  bool _v6Busy = false;
  String? _v6Result;
  String? _v6Error;
  bool _exploitBusy = false;
  String? _exploitResult;
  String? _exploitError;
  bool _rpmbBusy = false;
  String? _rpmbResult;
  String? _rpmbError;

  final _fdl1AddrCtrl = TextEditingController(text: '0x00090000');
  final _fdl2AddrCtrl = TextEditingController(text: '0x50000000');
  final _partitionCtrl = TextEditingController();
  String? _fdl1Path;
  String? _fdl2Path;
  String? _unisocImagePath;
  bool _unisocBusy = false;
  String? _unisocResult;
  String? _unisocError;
  bool _tableBusy = false;
  List<UnisocPartitionRow>? _tableRows;
  String? _tableError;
  bool _unlockBusy = false;
  String? _unlockResult;
  String? _unlockError;

  String? _programmerPath;
  String? _xmlPath;
  List<ChipsetProgramEntry>? _entries;
  bool _qualBusy = false;
  String? _qualResult;
  String? _qualError;
  bool _serialBusy = false;
  List<String>? _serialPorts;
  String? _selectedPort;
  String? _serialResult;
  String? _serialError;

  @override
  void initState() {
    super.initState();
    _loadPorts();
    _partitionCtrl.addListener(_refreshButtons);
  }

  @override
  void dispose() {
    _fdl1AddrCtrl.dispose();
    _fdl2AddrCtrl.dispose();
    _partitionCtrl.dispose();
    super.dispose();
  }

  void _refreshButtons() {
    if (mounted) setState(() {});
  }

  Future<void> _loadPorts() async {
    setState(() {
      _loadError = null;
    });
    try {
      final ports = await widget.repository.chipsetPorts();
      if (!mounted) return;
      setState(() {
        _ports = ports;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _loadError = e.toString();
      });
    }
  }

  Future<String?> _pickPath(List<String> extensions) async {
    final strings = AppLocalizations.of(context)!;
    final file = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(label: strings.chipsetsPickType, extensions: extensions),
      ],
    );
    return file?.path;
  }

  Future<bool> _confirm(String title, String body) async {
    final strings = AppLocalizations.of(context)!;
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
            child: Text(strings.chipsetsConfirmGo),
          ),
        ],
      ),
    );
    return confirmed == true;
  }

  int? _parseHex(String text) {
    final trimmed = text.trim().toLowerCase();
    final body = trimmed.startsWith('0x') ? trimmed.substring(2) : trimmed;
    return int.tryParse(body, radix: 16);
  }

  String _baseName(String path) {
    return path.replaceAll('\\', '/').split('/').last;
  }

  Future<void> _runMtkUnlock() async {
    final strings = AppLocalizations.of(context)!;
    final go = await _confirm(
      strings.chipsetsMtkUnlockConfirmTitle,
      strings.chipsetsMtkUnlockConfirmBody,
    );
    if (!go) return;
    setState(() {
      _mtkBusy = true;
      _mtkResult = null;
      _mtkError = null;
    });
    try {
      final message = await widget.repository.mediatekDaUnlock(_loaderPath!);
      if (!mounted) return;
      setState(() {
        _mtkResult = message;
        _mtkBusy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _mtkError = e.toString();
        _mtkBusy = false;
      });
    }
  }

  Future<void> _runUnisocFlash() async {
    final strings = AppLocalizations.of(context)!;
    final fdl1Addr = _parseHex(_fdl1AddrCtrl.text);
    final fdl2Addr = _parseHex(_fdl2AddrCtrl.text);
    if (fdl1Addr == null || fdl2Addr == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(strings.chipsetsAddrInvalid)),
      );
      return;
    }
    final go = await _confirm(
      strings.chipsetsUnisocConfirmTitle,
      strings.chipsetsUnisocConfirmBody(
        _baseName(_unisocImagePath!),
        _partitionCtrl.text.trim(),
      ),
    );
    if (!go) return;
    setState(() {
      _unisocBusy = true;
      _unisocResult = null;
      _unisocError = null;
    });
    try {
      final message = await widget.repository.unisocFlashPartition(
        fdl1Path: _fdl1Path!,
        fdl2Path: _fdl2Path!,
        fdl1Addr: fdl1Addr,
        fdl2Addr: fdl2Addr,
        partition: _partitionCtrl.text.trim(),
        imagePath: _unisocImagePath!,
      );
      if (!mounted) return;
      setState(() {
        _unisocResult = message;
        _unisocBusy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _unisocError = e.toString();
        _unisocBusy = false;
      });
    }
  }

  Future<void> _parseXml() async {    setState(() {
      _qualBusy = true;
      _qualResult = null;
      _qualError = null;
    });
    try {
      final entries = await widget.repository.parseRawprogram(_xmlPath!);
      if (!mounted) return;
      setState(() {
        _entries = entries;
        _qualBusy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _qualError = e.toString();
        _qualBusy = false;
      });
    }
  }

  Future<void> _runQualcommFlash() async {
    final strings = AppLocalizations.of(context)!;
    final go = await _confirm(
      strings.chipsetsQualcommConfirmTitle,
      strings.chipsetsQualcommConfirmBody,
    );
    if (!go) return;
    setState(() {
      _qualBusy = true;
      _qualResult = null;
      _qualError = null;
    });
    try {
      final message = await widget.repository.qualcommEdlFlash(
        _programmerPath!,
        _entries!,
      );
      if (!mounted) return;
      setState(() {
        _qualResult = message;
        _qualBusy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _qualError = e.toString();
        _qualBusy = false;
      });
    }
  }

  Future<void> _runV6Boot(String mode) async {
    setState(() {
      _v6Busy = true;
      _v6Result = null;
      _v6Error = null;
    });
    try {
      final message = await widget.repository.mediatekV6Boot(
        _loaderPath!,
        mode,
      );
      if (!mounted) return;
      setState(() {
        _v6Result = message;
        _v6Busy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _v6Error = e.toString();
        _v6Busy = false;
      });
    }
  }

  /// The bootrom dump goes to a file the user picks.
  Future<void> _runBromDump() async {
    final strings = AppLocalizations.of(context)!;
    final location = await getSaveLocation(
      suggestedName: 'brom_dump.bin',
      acceptedTypeGroups: [
        XTypeGroup(label: strings.chipsetsPickType, extensions: const ['bin']),
      ],
    );
    if (location == null) return;
    setState(() {
      _exploitBusy = true;
      _exploitResult = null;
      _exploitError = null;
    });
    try {
      final message = await widget.repository
          .mediatekExploitDumpBrom(location.path);
      if (!mounted) return;
      setState(() {
        _exploitResult = message;
        _exploitBusy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _exploitError = e.toString();
        _exploitBusy = false;
      });
    }
  }

  Future<void> _runExploitUnlock(bool carbonara) async {
    final strings = AppLocalizations.of(context)!;
    final go = await _confirm(
      strings.chipsetsExploitConfirmTitle,
      strings.chipsetsExploitConfirmBody,
    );
    if (!go) return;
    setState(() {
      _exploitBusy = true;
      _exploitResult = null;
      _exploitError = null;
    });
    try {
      final message = carbonara
          ? await widget.repository.mediatekCarbonaraUnlock(_loaderPath!)
          : await widget.repository.mediatekExploitUnlock(_loaderPath!);
      if (!mounted) return;
      setState(() {
        _exploitResult = message;
        _exploitBusy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _exploitError = e.toString();
        _exploitBusy = false;
      });
    }
  }

  Future<void> _runRpmbRead() async {
    setState(() {
      _rpmbBusy = true;
      _rpmbResult = null;
      _rpmbError = null;
    });
    try {
      final data = await widget.repository
          .mediatekRpmbRead(_loaderPath!, 0, 256);
      if (!mounted) return;
      final hex = data
          .take(32)
          .map((b) => b.toRadixString(16).padLeft(2, '0'))
          .join(' ');
      setState(() {
        _rpmbResult = '$hex${data.length > 32 ? ' ...' : ''}';
        _rpmbBusy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _rpmbError = e.toString();
        _rpmbBusy = false;
      });
    }
  }

  Future<void> _runUnisocUnlock() async {
    final strings = AppLocalizations.of(context)!;
    final go = await _confirm(
      strings.chipsetsExploitConfirmTitle,
      strings.chipsetsExploitConfirmBody,
    );
    if (!go) return;
    setState(() {
      _unlockBusy = true;
      _unlockResult = null;
      _unlockError = null;
    });
    try {
      final message = await widget.repository.unisocUnlockBootloader();
      if (!mounted) return;
      setState(() {
        _unlockResult = message;
        _unlockBusy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _unlockError = e.toString();
        _unlockBusy = false;
      });
    }
  }

  Future<void> _loadTable() async {
    final strings = AppLocalizations.of(context)!;
    final fdl1Addr = _parseHex(_fdl1AddrCtrl.text);
    final fdl2Addr = _parseHex(_fdl2AddrCtrl.text);
    if (fdl1Addr == null || fdl2Addr == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(strings.chipsetsAddrInvalid)),
      );
      return;
    }
    setState(() {
      _tableBusy = true;
      _tableRows = null;
      _tableError = null;
    });
    try {
      final rows = await widget.repository.unisocPartitionList(
        _fdl1Path!,
        _fdl2Path!,
        fdl1Addr,
        fdl2Addr,
      );
      if (!mounted) return;
      setState(() {
        _tableRows = rows;
        _tableBusy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _tableError = e.toString();
        _tableBusy = false;
      });
    }
  }

  Future<void> _loadSerialPorts() async {
    setState(() {
      _serialBusy = true;
      _serialPorts = null;
      _selectedPort = null;
      _serialResult = null;
      _serialError = null;
    });
    try {
      final ports = await widget.repository.qualcommSerialPorts();
      if (!mounted) return;
      setState(() {
        _serialPorts = ports;
        _serialBusy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _serialError = e.toString();
        _serialBusy = false;
      });
    }
  }

  Future<void> _runSerialFlash() async {
    final strings = AppLocalizations.of(context)!;
    final go = await _confirm(
      strings.chipsetsQualcommConfirmTitle,
      strings.chipsetsQualcommConfirmBody,
    );
    if (!go) return;
    setState(() {
      _serialBusy = true;
      _serialResult = null;
      _serialError = null;
    });
    try {
      final message = await widget.repository.qualcommEdlFlashSerial(
        _programmerPath!,
        _entries!,
        _selectedPort!,
      );
      if (!mounted) return;
      setState(() {
        _serialResult = message;
        _serialBusy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _serialError = e.toString();
        _serialBusy = false;
      });
    }
  }

  Widget _resultLine(
    BuildContext context, {
    String? result,
    String? error,
    String? successText,
  }) {
    final scheme = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (result != null) ...[
          const SizedBox(height: 12),
          if (successText != null)
            Text(
              successText,
              style: TextStyle(
                color: scheme.primary,
                fontWeight: FontWeight.w600,
              ),
            ),
          if (successText != null) const SizedBox(height: 4),
          SelectableText(
            result,
            style: TextStyle(color: scheme.primary, fontSize: 12),
          ),
        ],
        if (error != null) ...[
          const SizedBox(height: 12),
          SelectableText(
            error,
            style: TextStyle(color: scheme.error, fontSize: 12),
          ),
        ],
      ],
    );
  }

  Widget _pathRow(
    BuildContext context, {
    required String noneText,
    required String pickText,
    required List<String> extensions,
    required ValueChanged<String> onPicked,
  }) {
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            noneText,
            style: Theme.of(context).textTheme.bodySmall,
            overflow: TextOverflow.ellipsis,
          ),
          const SizedBox(height: 4),
          OutlinedButton(
            onPressed: () async {
              final path = await _pickPath(extensions);
              if (path != null) onPicked(path);
            },
            child: Text(pickText),
          ),
        ],
      ),
    );
  }

  Widget _portsLine(
    BuildContext context,
    AppLocalizations strings,
    List<String> serials,
  ) {
    if (serials.isEmpty) {
      return Text(
        strings.chipsetsPortsNone,
        style: Theme.of(context).textTheme.bodySmall,
      );
    }
    return Wrap(
      spacing: 8,
      runSpacing: 4,
      children: [
        for (final serial in serials) Chip(label: Text(serial)),
      ],
    );
  }

  Widget _card({
    required String title,
    required String hint,
    required bool busy,
    required List<Widget> children,
  }) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 4),
            Text(hint, style: Theme.of(context).textTheme.bodySmall),
            ...children,
            if (busy) ...[
              const SizedBox(height: 12),
              const Center(child: CircularProgressIndicator()),
            ],
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(title: Text(strings.chipsetsTitle)),
      body: RefreshIndicator(
        onRefresh: _loadPorts,
        child: ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.all(16),
          children: [
            if (_loadError != null) ...[
              Text(
                strings.chipsetsLoadFailed,
                style: TextStyle(
                  color: Theme.of(context).colorScheme.error,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              SelectableText(
                _loadError!,
                style: TextStyle(
                  color: Theme.of(context).colorScheme.error,
                  fontSize: 12,
                ),
              ),
              const SizedBox(height: 8),
              OutlinedButton(
                onPressed: _loadPorts,
                child: Text(strings.retryLabel),
              ),
              const SizedBox(height: 16),
            ],
            _card(
              title: strings.chipsetsMtkTitle,
              hint: strings.chipsetsMtkHint,
              busy: _mtkBusy,
              children: [
                const SizedBox(height: 8),
                _portsLine(context, strings, _ports?.mediatek ?? const []),
                _pathRow(
                  context,
                  noneText: _loaderPath == null
                      ? strings.chipsetsMtkLoaderNone
                      : _baseName(_loaderPath!),
                  pickText: strings.chipsetsMtkLoaderPick,
                  extensions: const ['bin'],
                  onPicked: (path) => setState(() => _loaderPath = path),
                ),
                const SizedBox(height: 12),
                FilledButton.icon(
                  onPressed:
                      _loaderPath != null && !_mtkBusy ? _runMtkUnlock : null,
                  icon: const Icon(Icons.lock_open),
                  label: Text(strings.chipsetsMtkUnlock),
                ),
                _resultLine(
                  context,
                  result: _mtkResult,
                  error: _mtkError,
                  successText: strings.chipsetsMtkDone,
                ),
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: Text(
                    strings.chipsetsMtkV6Boot,
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                ),
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton(
                        onPressed: _loaderPath != null && !_v6Busy
                            ? () => _runV6Boot('fastboot')
                            : null,
                        child: Text(strings.chipsetsMtkV6Fastboot),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: OutlinedButton(
                        onPressed: _loaderPath != null && !_v6Busy
                            ? () => _runV6Boot('meta')
                            : null,
                        child: Text(strings.chipsetsMtkV6Meta),
                      ),
                    ),
                  ],
                ),
                _resultLine(
                  context,
                  result: _v6Result,
                  error: _v6Error,
                  successText: strings.chipsetsMtkV6Done,
                ),
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: Text(
                    strings.chipsetsExploitTitle,
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  strings.chipsetsExploitHint,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const SizedBox(height: 4),
                OutlinedButton(
                  onPressed: !_exploitBusy ? _runBromDump : null,
                  child: Text(strings.chipsetsExploitDump),
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton(
                        onPressed: _loaderPath != null && !_exploitBusy
                            ? () => _runExploitUnlock(false)
                            : null,
                        child: Text(strings.chipsetsExploitKamakiri),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: OutlinedButton(
                        onPressed: _loaderPath != null && !_exploitBusy
                            ? () => _runExploitUnlock(true)
                            : null,
                        child: Text(strings.chipsetsExploitCarbonara),
                      ),
                    ),
                  ],
                ),
                _resultLine(
                  context,
                  result: _exploitResult,
                  error: _exploitError,
                  successText: strings.chipsetsExploitDone,
                ),
                const SizedBox(height: 8),
                OutlinedButton(
                  onPressed: _loaderPath != null && !_rpmbBusy
                      ? _runRpmbRead
                      : null,
                  child: Text(strings.chipsetsRpmbRead),
                ),
                _resultLine(
                  context,
                  result: _rpmbResult,
                  error: _rpmbError,
                  successText: strings.chipsetsRpmbDone,
                ),
              ],
            ),
            const SizedBox(height: 12),
            _card(
              title: strings.chipsetsUnisocTitle,
              hint: strings.chipsetsUnisocHint,
              busy: _unisocBusy,
              children: [
                const SizedBox(height: 8),
                _portsLine(context, strings, _ports?.unisoc ?? const []),
                _pathRow(
                  context,
                  noneText: _fdl1Path == null
                      ? strings.chipsetsUnisocPickFdl1
                      : _baseName(_fdl1Path!),
                  pickText: strings.chipsetsUnisocPickFdl1,
                  extensions: const ['bin'],
                  onPicked: (path) => setState(() => _fdl1Path = path),
                ),
                _pathRow(
                  context,
                  noneText: _fdl2Path == null
                      ? strings.chipsetsUnisocPickFdl2
                      : _baseName(_fdl2Path!),
                  pickText: strings.chipsetsUnisocPickFdl2,
                  extensions: const ['bin'],
                  onPicked: (path) => setState(() => _fdl2Path = path),
                ),
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _fdl1AddrCtrl,
                          decoration: InputDecoration(
                            labelText: strings.chipsetsUnisocAddr1Label,
                            hintText: strings.chipsetsUnisocAddrHint,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: TextField(
                          controller: _fdl2AddrCtrl,
                          decoration: InputDecoration(
                            labelText: strings.chipsetsUnisocAddr2Label,
                            hintText: strings.chipsetsUnisocAddrHint,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: TextField(
                    controller: _partitionCtrl,
                    decoration: InputDecoration(
                      labelText: strings.chipsetsUnisocPartitionLabel,
                    ),
                  ),
                ),
                _pathRow(
                  context,
                  noneText: _unisocImagePath == null
                      ? strings.chipsetsUnisocImageNone
                      : _baseName(_unisocImagePath!),
                  pickText: strings.chipsetsUnisocImagePick,
                  extensions: const ['img'],
                  onPicked: (path) => setState(() => _unisocImagePath = path),
                ),
                const SizedBox(height: 12),
                FilledButton.icon(
                  onPressed: _unisocReady && !_unisocBusy
                      ? _runUnisocFlash
                      : null,
                  icon: const Icon(Icons.flash_on),
                  label: Text(strings.chipsetsUnisocFlash),
                ),
                _resultLine(
                  context,
                  result: _unisocResult,
                  error: _unisocError,
                  successText: strings.chipsetsUnisocDone,
                ),
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: OutlinedButton(
                    onPressed: _fdl1Path != null &&
                            _fdl2Path != null &&
                            !_tableBusy &&
                            !_unisocBusy
                        ? _loadTable
                        : null,
                    child: Text(strings.chipsetsUnisocTable),
                  ),
                ),
                if (_tableRows != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    strings.chipsetsUnisocTableDone,
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                  const SizedBox(height: 4),
                  for (final row in _tableRows!)
                    Text(
                      '${row.name} - ${strings.chipsetsUnisocSectors(row.sizeSectors.toInt())}',
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                ],
                _resultLine(context, error: _tableError),
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: Text(
                    strings.chipsetsUnisocUnlock,
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  strings.chipsetsUnisocUnlockHint,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const SizedBox(height: 4),
                FilledButton.icon(
                  onPressed: !_unlockBusy ? _runUnisocUnlock : null,
                  icon: const Icon(Icons.lock_open),
                  label: Text(strings.chipsetsUnisocUnlock),
                ),
                _resultLine(
                  context,
                  result: _unlockResult,
                  error: _unlockError,
                  successText: strings.chipsetsUnisocUnlockDone,
                ),
              ],
            ),
            const SizedBox(height: 12),
            _card(
              title: strings.chipsetsQualcommTitle,
              hint: strings.chipsetsQualcommHint,
              busy: _qualBusy,
              children: [
                const SizedBox(height: 8),
                _portsLine(context, strings, _ports?.qualcomm ?? const []),
                _pathRow(
                  context,
                  noneText: _programmerPath == null
                      ? strings.chipsetsQualcommProgrammerNone
                      : _baseName(_programmerPath!),
                  pickText: strings.chipsetsQualcommProgrammerPick,
                  extensions: const ['bin', 'elf', 'mbn'],
                  onPicked: (path) => setState(() => _programmerPath = path),
                ),
                _pathRow(
                  context,
                  noneText: _xmlPath == null
                      ? strings.chipsetsQualcommXmlPick
                      : _baseName(_xmlPath!),
                  pickText: strings.chipsetsQualcommXmlPick,
                  extensions: const ['xml'],
                  onPicked: (path) => setState(() {
                    _xmlPath = path;
                    _entries = null;
                  }),
                ),
                const SizedBox(height: 8),
                OutlinedButton(
                  onPressed: _xmlPath != null && !_qualBusy
                      ? _parseXml
                      : null,
                  child: Text(strings.chipsetsQualcommParse),
                ),
                if (_entries != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    strings.chipsetsQualcommEntriesLabel(_entries!.length),
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                  const SizedBox(height: 4),
                  for (final entry in _entries!)
                    Text(
                      strings.chipsetsQualcommEntryLine(
                        entry.label,
                        _baseName(entry.filePath),
                        entry.startSector,
                        entry.numSectors.toString(),
                      ),
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                ],
                const SizedBox(height: 12),
                FilledButton.icon(
                  onPressed: _programmerPath != null &&
                          _entries != null &&
                          _entries!.isNotEmpty &&
                          !_qualBusy
                      ? _runQualcommFlash
                      : null,
                  icon: const Icon(Icons.flash_on),
                  label: Text(strings.chipsetsQualcommFlash),
                ),
                _resultLine(
                  context,
                  result: _qualResult,
                  error: _qualError,
                  successText: strings.chipsetsQualcommDone,
                ),
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: Text(
                    strings.chipsetsQualcommSerial,
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                ),
                const SizedBox(height: 4),
                OutlinedButton(
                  onPressed: !_serialBusy ? _loadSerialPorts : null,
                  child: Text(strings.chipsetsQualcommSerialLoad),
                ),
                if (_serialPorts != null) ...[
                  const SizedBox(height: 4),
                  if (_serialPorts!.isEmpty)
                    Text(
                      strings.chipsetsQualcommSerialNone,
                      style: Theme.of(context).textTheme.bodySmall,
                    )
                  else
                    Wrap(
                      spacing: 8,
                      runSpacing: 4,
                      children: [
                        for (final port in _serialPorts!)
                          ChoiceChip(
                            label: Text(port),
                            selected: _selectedPort == port,
                            onSelected: (_) =>
                                setState(() => _selectedPort = port),
                          ),
                      ],
                    ),
                ],
                const SizedBox(height: 12),
                FilledButton.icon(
                  onPressed: _programmerPath != null &&
                          _entries != null &&
                          _entries!.isNotEmpty &&
                          _selectedPort != null &&
                          !_serialBusy
                      ? _runSerialFlash
                      : null,
                  icon: const Icon(Icons.settings_input_composite),
                  label: Text(strings.chipsetsQualcommSerialFlash),
                ),
                _resultLine(
                  context,
                  result: _serialResult,
                  error: _serialError,
                  successText: strings.chipsetsQualcommSerialDone,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  bool get _unisocReady =>
      _fdl1Path != null &&
      _fdl2Path != null &&
      _unisocImagePath != null &&
      _partitionCtrl.text.trim().isNotEmpty;
}
