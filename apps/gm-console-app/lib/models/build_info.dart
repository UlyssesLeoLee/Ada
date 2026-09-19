import 'package:flutter/foundation.dart';

@immutable
class BuildInfo {
  const BuildInfo({
    required this.flavor,
    required this.version,
    required this.buildNumber,
    this.packageName = '',
  });

  final String flavor;
  final String version;
  final String buildNumber;
  final String packageName;

  String get humanLabel => '$flavor $version+$buildNumber';

  @override
  String toString() => humanLabel;
}