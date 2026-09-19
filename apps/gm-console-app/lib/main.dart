// gm-console Mobile — entry point. Real implementation (auth, routing, screens, theming)
// is owned by worker-B. Mavis scaffold ensures the runnable shell compiles.

import 'package:flutter/material.dart';

void main() {
  runApp(const GmConsoleApp());
}

class GmConsoleApp extends StatelessWidget {
  const GmConsoleApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'gm-console',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF1E88E5)),
        useMaterial3: true,
      ),
      home: const _Bootstrap(),
    );
  }
}

class _Bootstrap extends StatelessWidget {
  const _Bootstrap();

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('gm-console')),
      body: const Center(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Text(
            'gm-console mobile scaffold ready.\n'
            'worker-B will replace this with auth → dashboard → monitor flows.',
            textAlign: TextAlign.center,
          ),
        ),
      ),
    );
  }
}
