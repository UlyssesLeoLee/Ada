import '../../features/audit/data/audit_event_model.dart';
import '../api_client.dart';

class AuditApi {
  AuditApi(this._client);

  final ApiClient _client;

  Future<List<AuditEvent>> list({String? tenantId, int? limit}) async {
    final query = <String, String>{};
    if (tenantId != null) query['tenant_id'] = tenantId;
    if (limit != null) query['limit'] = limit.toString();
    final body = await _client.getJson('/api/v1/audit', query: query);
    final raw = body['events'] ?? body['items'] ?? body['data'];
    if (raw is! List) return const <AuditEvent>[];
    return raw
        .whereType<Map<String, dynamic>>()
        .map(AuditEvent.fromJson)
        .toList(growable: false);
  }
}