import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/endpoints/incidents_api.dart';
import '../../auth/data/auth_repository.dart';
import '../data/incident_model.dart';

final incidentsApiProvider = Provider<IncidentsApi>((ref) {
  return IncidentsApi(ref.watch(apiClientProvider));
});

final incidentsProvider = FutureProvider<List<Incident>>((ref) async {
  return ref.watch(incidentsApiProvider).list(limit: 100);
});