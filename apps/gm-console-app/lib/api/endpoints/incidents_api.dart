import '../../features/incidents/data/incident_model.dart';
import '../api_client.dart';

class IncidentsApi {
  IncidentsApi(this._client);

  final ApiClient _client;

  Future<List<Incident>> list({String? tenantId, int? limit}) async {
    final query = <String, String>{};
    if (tenantId != null) query['tenant_id'] = tenantId;
    if (limit != null) query['limit'] = limit.toString();
    final body = await _client.getJson('/api/v1/incidents', query: query);
    final raw = body['incidents'] ?? body['items'] ?? body['data'];
    if (raw is! List) return const <Incident>[];
    return raw
        .whereType<Map<String, dynamic>>()
        .map(Incident.fromJson)
        .toList(growable: false);
  }
}