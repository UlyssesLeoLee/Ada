import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/endpoints/pipelines_api.dart';
import '../../auth/data/auth_repository.dart';
import '../data/pipeline_model.dart';

final pipelinesApiProvider = Provider<PipelinesApi>((ref) {
  return PipelinesApi(ref.watch(apiClientProvider));
});

final pipelinesProvider = FutureProvider<List<Pipeline>>((ref) async {
  return ref.watch(pipelinesApiProvider).list();
});

final pipelineDetailProvider =
    FutureProvider.family<Pipeline, String>((ref, pipelineId) async {
  return ref.watch(pipelinesApiProvider).get(pipelineId);
});

final pipelineRetryControllerProvider =
    StateNotifierProvider<PipelineRetryController, AsyncValue<void>>((ref) {
  return PipelineRetryController(ref);
});

class PipelineRetryController extends StateNotifier<AsyncValue<void>> {
  PipelineRetryController(this._ref) : super(const AsyncValue.data(null));

  final Ref _ref;

  Future<bool> retry(String pipelineId) async {
    state = const AsyncValue.loading();
    try {
      await _ref.read(pipelinesApiProvider).retry(pipelineId);
      state = const AsyncValue.data(null);
      // Invalidate cached list/detail so callers see the new run.
      _ref.invalidate(pipelinesProvider);
      _ref.invalidate(pipelineDetailProvider(pipelineId));
      return true;
    } catch (e, st) {
      state = AsyncValue.error(e, st);
      return false;
    }
  }
}