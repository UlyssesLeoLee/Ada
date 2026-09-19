import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:package_info_plus/package_info_plus.dart';

import '../api/api_config.dart';
import '../models/build_info.dart';

final apiConfigProvider = Provider<ApiConfig>((ref) {
  return ApiConfig.fromEnvironment();
});

final buildInfoProvider = FutureProvider<BuildInfo>((ref) async {
  final info = await PackageInfo.fromPlatform();
  return BuildInfo(
    flavor: ref.watch(apiConfigProvider).flavor,
    version: info.version.isEmpty ? '0.0.0' : info.version,
    buildNumber: info.buildNumber.isEmpty ? '0' : info.buildNumber,
    packageName: info.packageName,
  );
});