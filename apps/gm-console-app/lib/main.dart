// gm-console Mobile — entry point.
//
// The root widget is `GmConsoleApp`, which wires together Riverpod state,
// go_router, secure-storage-backed auth, and the Material 3 theme.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app.dart';

void main() {
  runApp(const ProviderScope(child: GmConsoleApp()));
}