import '../../features/tenants/data/tenant_model.dart';
import '../api_client.dart';

class TenantsApi {
  TenantsApi(this._client);

  final ApiClient _client;

  Future<List<Tenant>> list() async {
    final body = await _client.getJson('/api/v1/tenants');
    final raw = body['tenants'] ?? body['items'] ?? body['data'];
    if (raw is! List) return const <Tenant>[];
    return raw
        .whereType<Map<String, dynamic>>()
        .map(Tenant.fromJson)
        .toList(growable: false);
  }

  Future<Tenant> get(String tenantId) async {
    final body = await _client.getJson('/api/v1/tenants/$tenantId');
    return Tenant.fromJson(body);
  }
}