import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/endpoints/audit_api.dart';
import '../../auth/data/auth_repository.dart';
import '../data/audit_event_model.dart';

final auditApiProvider = Provider<AuditApi>((ref) {
  return AuditApi(ref.watch(apiClientProvider));
});

final auditEventsProvider = FutureProvider<List<AuditEvent>>((ref) async {
  return ref.watch(auditApiProvider).list(limit: 100);
});