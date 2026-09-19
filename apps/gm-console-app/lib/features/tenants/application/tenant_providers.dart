import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/endpoints/tenants_api.dart';
import '../data/tenant_model.dart';
import '../auth/data/auth_repository.dart';

final tenantsApiProvider = Provider<TenantsApi>((ref) {
  return TenantsApi(ref.watch(apiClientProvider));
});

final tenantsProvider = FutureProvider<List<Tenant>>((ref) async {
  return ref.watch(tenantsApiProvider).list();
});