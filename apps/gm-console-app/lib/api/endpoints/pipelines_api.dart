import '../../features/pipelines/data/pipeline_model.dart';
import '../api_client.dart';

class PipelinesApi {
  PipelinesApi(this._client);

  final ApiClient _client;

  Future<List<Pipeline>> list({String? tenantId}) async {
    final body = await _client.getJson(
      '/api/v1/pipelines',
      query: tenantId == null ? null : {'tenant_id': tenantId},
    );
    final raw = body['pipelines'] ?? body['items'] ?? body['data'];
    if (raw is! List) return const <Pipeline>[];
    return raw
        .whereType<Map<String, dynamic>>()
        .map(Pipeline.fromJson)
        .toList(growable: false);
  }

  Future<Pipeline> get(String pipelineId) async {
    final body = await _client.getJson('/api/v1/pipelines/$pipelineId');
    return Pipeline.fromJson(body);
  }

  Future<PipelineRun> retry(String pipelineId) async {
    final body =
        await _client.postJson('/api/v1/pipelines/$pipelineId/retry');
    return PipelineRun.fromJson(body);
  }
}