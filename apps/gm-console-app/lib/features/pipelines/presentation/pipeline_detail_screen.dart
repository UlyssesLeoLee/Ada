import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../widgets/error_view.dart';
import '../../../widgets/status_pill.dart';
import '../application/pipeline_providers.dart';

class PipelineDetailScreen extends ConsumerWidget {
  const PipelineDetailScreen({super.key, required this.pipelineId});

  final String pipelineId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final async = ref.watch(pipelineDetailProvider(pipelineId));
    final retry = ref.watch(pipelineRetryControllerProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Pipeline'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.go('/pipelines'),
        ),
      ),
      body: async.when(
        data: (pipeline) {
          final last = pipeline.lastRun;
          return ListView(
            padding: const EdgeInsets.all(20),
            children: [
              Text(pipeline.name,
                  style: Theme.of(context).textTheme.headlineSmall),
              const SizedBox(height: 12),
              StatusPill(status: pipeline.status),
              const SizedBox(height: 24),
              Text('Last run',
                  style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Run id: ${last?.id ?? '—'}'),
                      const SizedBox(height: 4),
                      Text(
                          'Started: ${last?.startedAt?.toLocal() ?? '—'}'),
                      const SizedBox(height: 4),
                      Text(
                          'Finished: ${last?.finishedAt?.toLocal() ?? '—'}'),
                      const SizedBox(height: 4),
                      Text('Commit: ${last?.commitSha ?? '—'}'),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 24),
              FilledButton.icon(
                key: const Key('pipeline.retry'),
                onPressed: retry.isLoading
                    ? null
                    : () async {
                        final ok = await ref
                            .read(pipelineRetryControllerProvider.notifier)
                            .retry(pipelineId);
                        if (!context.mounted) return;
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(
                            content: Text(ok
                                ? 'Retry requested'
                                : 'Retry failed'),
                          ),
                        );
                      },
                icon: retry.isLoading
                    ? const SizedBox(
                        height: 16,
                        width: 16,
                        child: CircularProgressIndicator(
                            strokeWidth: 2, color: Colors.white))
                    : const Icon(Icons.refresh),
                label: const Text('Retry pipeline'),
              ),
              if (retry.hasError) ...[
                const SizedBox(height: 12),
                Text(
                  retry.error.toString(),
                  style: TextStyle(
                      color: Theme.of(context).colorScheme.error),
                ),
              ],
            ],
          );
        },
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => ErrorView(
          message: e.toString(),
          onRetry: () =>
              ref.invalidate(pipelineDetailProvider(pipelineId)),
        ),
      ),
    );
  }
}