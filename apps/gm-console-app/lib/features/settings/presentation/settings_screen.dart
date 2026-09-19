import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../state/build_info_providers.dart';
import '../../../state/locale_controller.dart';
import '../../auth/application/auth_providers.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final buildInfoAsync = ref.watch(buildInfoProvider);
    final locale = ref.watch(localeControllerProvider);

    return Scaffold(
      appBar: AppBar(title: const Text('Settings')),
      body: ListView(
        children: [
          const _SectionHeader('Build'),
          buildInfoAsync.when(
            data: (info) => ListTile(
              leading: const Icon(Icons.info_outline),
              title: const Text('Version'),
              subtitle: Text(info.humanLabel),
            ),
            loading: () => const ListTile(
              leading: SizedBox(
                height: 18,
                width: 18,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
              title: Text('Loading build info…'),
            ),
            error: (e, _) => ListTile(
              leading: const Icon(Icons.error_outline),
              title: const Text('Build info unavailable'),
              subtitle: Text(e.toString()),
            ),
          ),
          const Divider(),
          const _SectionHeader('Language'),
          RadioListTile<Locale>(
            value: kLocaleEn,
            groupValue: locale,
            onChanged: (v) => _setLocale(ref, v),
            title: const Text('English'),
          ),
          RadioListTile<Locale>(
            value: kLocaleJa,
            groupValue: locale,
            onChanged: (v) => _setLocale(ref, v),
            title: const Text('日本語'),
          ),
          RadioListTile<Locale>(
            value: kLocaleZhCn,
            groupValue: locale,
            onChanged: (v) => _setLocale(ref, v),
            title: const Text('中文 (简体)'),
          ),
          const Divider(),
          const _SectionHeader('Security'),
          ListTile(
            key: const Key('settings.revoke'),
            leading: const Icon(Icons.logout),
            title: const Text('Revoke token & sign out'),
            subtitle: const Text('Clears the local session token.'),
            onTap: () async {
              await ref
                  .read(authControllerProvider.notifier)
                  .revoke();
              if (!context.mounted) return;
              ScaffoldMessenger.of(context).showSnackBar(
                const SnackBar(content: Text('Signed out.')),
              );
            },
          ),
        ],
      ),
    );
  }

  Future<void> _setLocale(WidgetRef ref, Locale? locale) async {
    if (locale == null) return;
    await ref.read(localeControllerProvider.notifier).setLocale(locale);
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader(this.text);
  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 20, 16, 8),
      child: Text(
        text,
        style: Theme.of(context)
            .textTheme
            .labelLarge
            ?.copyWith(color: Theme.of(context).colorScheme.primary),
      ),
    );
  }
}