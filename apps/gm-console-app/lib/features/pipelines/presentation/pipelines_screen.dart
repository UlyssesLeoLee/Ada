import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../widgets/empty_state.dart';
import '../../../widgets/error_view.dart';
import '../../../widgets/status_pill.dart';
import '../application/pipeline_providers.dart';
import '../data/pipeline_model.dart';

class PipelinesScreen extends ConsumerWidget {
  const PipelinesScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final pipelinesAsync = ref.watch(pipelinesProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('Pipelines')),
      body: RefreshIndicator(
        onRefresh: () async => ref.invalidate(pipelinesProvider),
        child: pipelinesAsync.when(
          data: (pipelines) {
            if (pipelines.isEmpty) {
              return const EmptyStateView(
                title: 'No pipelines yet',
                message:
                    'When your team adds a pipeline, it will show up here.',
              );
            }
            return ListView.separated(
              itemCount: pipelines.length,
              separatorBuilder: (_, __) => const Divider(height: 1),
              itemBuilder: (context, index) {
                final p = pipelines[index];
                return ListTile(
                  title: Text(p.name),
                  subtitle: _lastRunLabel(p.lastRun),
                  trailing: StatusPill(status: p.status),
                  onTap: () => context.go('/pipelines/${p.id}'),
                );
              },
            );
          },
          loading: () => const Center(child: CircularProgressIndicator()),
          error: (e, _) => ErrorView(
            message: e.toString(),
            onRetry: () => ref.invalidate(pipelinesProvider),
          ),
        ),
      ),
    );
  }

  Widget? _lastRunLabel(PipelineRun? run) {
    if (run == null) return const Text('No runs yet');
    final stamp = run.startedAt ?? run.finishedAt;
    final prefix = stamp == null ? '' : '${stamp.toLocal()} · ';
    return Text('$prefix${run.id}');
  }
}