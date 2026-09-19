import 'package:flutter/foundation.dart';

@immutable
class AuditEvent {
  const AuditEvent({
    required this.id,
    required this.action,
    required this.actor,
    required this.occurredAt,
    this.target,
    this.tenantId,
  });

  final String id;
  final String action;
  final String actor;
  final DateTime occurredAt;
  final String? target;
  final String? tenantId;

  factory AuditEvent.fromJson(Map<String, dynamic> json) {
    final occurred = json['occurred_at'] ?? json['created_at'];
    return AuditEvent(
      id: (json['id'] ?? json['event_id'] ?? '').toString(),
      action: (json['action'] ?? json['event'] ?? '').toString(),
      actor: (json['actor'] ?? json['user'] ?? '').toString(),
      occurredAt: occurred is String
          ? (DateTime.tryParse(occurred) ?? DateTime.fromMillisecondsSinceEpoch(0))
          : DateTime.fromMillisecondsSinceEpoch(0),
      target: json['target'] as String?,
      tenantId: json['tenant_id'] as String?,
    );
  }
}