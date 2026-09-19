import 'package:flutter/foundation.dart';

enum PipelineStatus {
  idle,
  running,
  succeeded,
  failed,
  cancelled,
  unknown,
}

PipelineStatus parsePipelineStatus(Object? raw) {
  switch (raw) {
    case 'idle':
    case 'ready':
      return PipelineStatus.idle;
    case 'running':
    case 'in_progress':
      return PipelineStatus.running;
    case 'succeeded':
    case 'success':
    case 'passed':
      return PipelineStatus.succeeded;
    case 'failed':
    case 'failure':
    case 'errored':
      return PipelineStatus.failed;
    case 'cancelled':
    case 'canceled':
      return PipelineStatus.cancelled;
    default:
      return PipelineStatus.unknown;
  }
}

@immutable
class PipelineRun {
  const PipelineRun({
    required this.id,
    required this.status,
    required this.startedAt,
    this.finishedAt,
    this.commitSha,
  });

  final String id;
  final PipelineStatus status;
  final DateTime? startedAt;
  final DateTime? finishedAt;
  final String? commitSha;

  factory PipelineRun.fromJson(Map<String, dynamic> json) {
    DateTime? parse(Object? v) =>
        v is String ? DateTime.tryParse(v) : null;

    return PipelineRun(
      id: (json['id'] ?? json['run_id'] ?? '').toString(),
      status: parsePipelineStatus(json['status']),
      startedAt: parse(json['started_at']),
      finishedAt: parse(json['finished_at']),
      commitSha: json['commit_sha'] as String?,
    );
  }
}

@immutable
class Pipeline {
  const Pipeline({
    required this.id,
    required this.name,
    required this.status,
    this.lastRun,
    this.tenantId,
  });

  final String id;
  final String name;
  final PipelineStatus status;
  final PipelineRun? lastRun;
  final String? tenantId;

  factory Pipeline.fromJson(Map<String, dynamic> json) {
    final lastRunRaw = json['last_run'];
    return Pipeline(
      id: (json['id'] ?? json['pipeline_id'] ?? '').toString(),
      name: (json['name'] ?? json['display_name'] ?? '').toString(),
      status: parsePipelineStatus(json['status']),
      lastRun: lastRunRaw is Map<String, dynamic>
          ? PipelineRun.fromJson(lastRunRaw)
          : null,
      tenantId: json['tenant_id'] as String?,
    );
  }
}