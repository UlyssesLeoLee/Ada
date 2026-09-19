import 'package:flutter/foundation.dart';

enum IncidentSeverity {
  info,
  warning,
  critical,
  unknown,
}

IncidentSeverity parseSeverity(Object? raw) {
  switch (raw) {
    case 'info':
    case 'low':
      return IncidentSeverity.info;
    case 'warning':
    case 'warn':
    case 'medium':
      return IncidentSeverity.warning;
    case 'critical':
    case 'high':
    case 'severe':
      return IncidentSeverity.critical;
    default:
      return IncidentSeverity.unknown;
  }
}

@immutable
class Incident {
  const Incident({
    required this.id,
    required this.title,
    required this.severity,
    required this.occurredAt,
    this.tenantId,
    this.status,
    this.summary,
  });

  final String id;
  final String title;
  final IncidentSeverity severity;
  final DateTime occurredAt;
  final String? tenantId;
  final String? status;
  final String? summary;

  factory Incident.fromJson(Map<String, dynamic> json) {
    final occurred = json['occurred_at'] ?? json['created_at'];
    return Incident(
      id: (json['id'] ?? json['incident_id'] ?? '').toString(),
      title: (json['title'] ?? json['message'] ?? '').toString(),
      severity: parseSeverity(json['severity']),
      occurredAt: occurred is String
          ? (DateTime.tryParse(occurred) ?? DateTime.fromMillisecondsSinceEpoch(0))
          : DateTime.fromMillisecondsSinceEpoch(0),
      tenantId: json['tenant_id'] as String?,
      status: json['status'] as String?,
      summary: json['summary'] as String?,
    );
  }
}