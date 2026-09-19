import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gm_console_app/features/pipelines/application/pipeline_providers.dart';
import 'package:gm_console_app/features/pipelines/data/pipeline_model.dart';
import 'package:gm_console_app/features/pipelines/presentation/pipelines_screen.dart';

void main() {
  testWidgets('PipelinesScreen shows empty state when no pipelines',
      (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          pipelinesProvider.overrideWith(
              (ref) async => const <Pipeline>[]),
        ],
        child: const MaterialApp(home: PipelinesScreen()),
      ),
    );

    // Initial frame triggers the future; pump until it settles.
    await tester.pumpAndSettle();

    expect(find.text('Pipelines'), findsOneWidget);
    expect(find.text('No pipelines yet'), findsOneWidget);
    expect(
        find.text(
            'When your team adds a pipeline, it will show up here.'),
        findsOneWidget);
  });
}