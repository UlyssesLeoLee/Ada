import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../widgets/empty_state.dart';
import '../../widgets/error_view.dart';
import '../application/tenant_providers.dart';

class TenantsScreen extends ConsumerWidget {
  const TenantsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tenantsAsync = ref.watch(tenantsProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('Workspaces')),
      body: RefreshIndicator(
        onRefresh: () async => ref.invalidate(tenantsProvider),
        child: tenantsAsync.when(
          data: (tenants) {
            if (tenants.isEmpty) {
              return const EmptyStateView(
                title: 'No workspaces yet',
                message:
                    'Ask your admin to invite you to a workspace, then pull to refresh.',
              );
            }
            return ListView.separated(
              itemCount: tenants.length,
              separatorBuilder: (_, __) => const Divider(height: 1),
              itemBuilder: (context, index) {
                final tenant = tenants[index];
                return ListTile(
                  leading: CircleAvatar(child: Text(tenant.name.isNotEmpty
                      ? tenant.name[0].toUpperCase()
                      : '?')),
                  title: Text(tenant.name),
                  subtitle: Text('${tenant.slug} · ${tenant.role}'
                      '${tenant.region != null ? ' · ${tenant.region}' : ''}'),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(content: Text('Switching to ${tenant.name}…')),
                    );
                  },
                );
              },
            );
          },
          loading: () => const Center(child: CircularProgressIndicator()),
          error: (e, _) => ErrorView(
            message: e.toString(),
            onRetry: () => ref.invalidate(tenantsProvider),
          ),
        ),
      ),
    );
  }
}