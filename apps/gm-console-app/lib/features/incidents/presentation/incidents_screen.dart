import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../widgets/empty_state.dart';
import '../../../widgets/error_view.dart';
import '../application/incident_providers.dart';
import '../data/incident_model.dart';

class IncidentsScreen extends ConsumerWidget {
  const IncidentsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final incidentsAsync = ref.watch(incidentsProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('Incidents')),
      body: RefreshIndicator(
        onRefresh: () async => ref.invalidate(incidentsProvider),
        child: incidentsAsync.when(
          data: (incidents) {
            if (incidents.isEmpty) {
              return const EmptyStateView(
                title: 'All clear',
                message: 'No incidents in this window.',
                icon: Icons.check_circle_outline,
              );
            }
            return ListView.separated(
              itemCount: incidents.length,
              separatorBuilder: (_, __) => const Divider(height: 1),
              itemBuilder: (context, index) {
                final i = incidents[index];
                return ListTile(
                  leading: _severityDot(i.severity),
                  title: Text(i.title),
                  subtitle: Text(
                    '${i.occurredAt.toLocal()}'
                    '${i.status != null ? ' · ${i.status}' : ''}',
                  ),
                  trailing: i.tenantId == null
                      ? null
                      : Chip(label: Text(i.tenantId!)),
                );
              },
            );
          },
          loading: () => const Center(child: CircularProgressIndicator()),
          error: (e, _) => ErrorView(
            message: e.toString(),
            onRetry: () => ref.invalidate(incidentsProvider),
          ),
        ),
      ),
    );
  }

  Widget _severityDot(IncidentSeverity s) {
    Color color;
    switch (s) {
      case IncidentSeverity.info:
        color = Colors.blue;
        break;
      case IncidentSeverity.warning:
        color = Colors.orange;
        break;
      case IncidentSeverity.critical:
        color = Colors.red;
        break;
      case IncidentSeverity.unknown:
        color = Colors.grey;
        break;
    }
    return Container(
      width: 12,
      height: 12,
      decoration: BoxDecoration(color: color, shape: BoxShape.circle),
    );
  }
}