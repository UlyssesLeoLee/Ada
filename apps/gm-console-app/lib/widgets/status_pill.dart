import 'package:flutter/material.dart';

import '../features/pipelines/data/pipeline_model.dart';

class StatusPill extends StatelessWidget {
  const StatusPill({super.key, required this.status});

  final PipelineStatus status;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = _colorFor(status, theme);
    final label = _labelFor(status);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.12),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withOpacity(0.4)),
      ),
      child: Text(
        label,
        style: theme.textTheme.labelSmall?.copyWith(
          color: color,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }

  Color _colorFor(PipelineStatus s, ThemeData theme) {
    switch (s) {
      case PipelineStatus.idle:
        return Colors.blueGrey;
      case PipelineStatus.running:
        return Colors.indigo;
      case PipelineStatus.succeeded:
        return Colors.green.shade700;
      case PipelineStatus.failed:
        return Colors.red.shade700;
      case PipelineStatus.cancelled:
        return Colors.orange.shade800;
      case PipelineStatus.unknown:
        return theme.disabledColor;
    }
  }

  String _labelFor(PipelineStatus s) {
    switch (s) {
      case PipelineStatus.idle:
        return 'idle';
      case PipelineStatus.running:
        return 'running';
      case PipelineStatus.succeeded:
        return 'succeeded';
      case PipelineStatus.failed:
        return 'failed';
      case PipelineStatus.cancelled:
        return 'cancelled';
      case PipelineStatus.unknown:
        return 'unknown';
    }
  }
}