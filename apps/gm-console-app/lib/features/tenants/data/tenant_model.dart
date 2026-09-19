import 'package:flutter/foundation.dart';

@immutable
class Tenant {
  const Tenant({
    required this.id,
    required this.name,
    required this.slug,
    required this.role,
    this.region,
  });

  final String id;
  final String name;
  final String slug;
  final String role;
  final String? region;

  factory Tenant.fromJson(Map<String, dynamic> json) {
    return Tenant(
      id: (json['id'] ?? json['tenant_id'] ?? '').toString(),
      name: (json['name'] ?? json['display_name'] ?? '').toString(),
      slug: (json['slug'] ?? '').toString(),
      role: (json['role'] ?? json['user_role'] ?? 'member').toString(),
      region: json['region'] as String?,
    );
  }
}