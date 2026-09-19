import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../widgets/empty_state.dart';
import '../../../widgets/error_view.dart';
import '../application/audit_providers.dart';

class AuditScreen extends ConsumerWidget {
  const AuditScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final auditAsync = ref.watch(auditEventsProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('Audit')),
      body: RefreshIndicator(
        onRefresh: () async => ref.invalidate(auditEventsProvider),
        child: auditAsync.when(
          data: (events) {
            if (events.isEmpty) {
              return const EmptyStateView(
                title: 'No recent events',
                message: 'Audit activity will appear here as it occurs.',
              );
            }
            return ListView.separated(
              itemCount: events.length,
              separatorBuilder: (_, __) => const Divider(height: 1),
              itemBuilder: (context, index) {
                final e = events[index];
                return ListTile(
                  leading: const Icon(Icons.history),
                  title: Text(e.action),
                  subtitle: Text(
                    '${e.actor}'
                    '${e.target != null ? ' → ${e.target}' : ''}'
                    '\n${e.occurredAt.toLocal()}',
                  ),
                  isThreeLine: e.target != null,
                );
              },
            );
          },
          loading: () => const Center(child: CircularProgressIndicator()),
          error: (e, _) => ErrorView(
            message: e.toString(),
            onRetry: () => ref.invalidate(auditEventsProvider),
          ),
        ),
      ),
    );
  }
}