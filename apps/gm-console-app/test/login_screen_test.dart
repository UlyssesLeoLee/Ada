import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gm_console_app/features/auth/presentation/login_screen.dart';

void main() {
  testWidgets('login form rejects invalid email and short password',
      (tester) async {
    await tester.pumpWidget(
      const ProviderScope(
        child: MaterialApp(home: LoginScreen()),
      ),
    );

    final emailField = find.byKey(const Key('login.email'));
    final passwordField = find.byKey(const Key('login.password'));
    final submit = find.byKey(const Key('login.submit'));

    expect(emailField, findsOneWidget);
    expect(passwordField, findsOneWidget);
    expect(submit, findsOneWidget);

    // Empty submit → both required errors appear.
    await tester.tap(submit);
    await tester.pump();
    expect(find.text('Email is required'), findsOneWidget);
    expect(find.text('Password is required'), findsOneWidget);

    // Malformed email → "valid email" error.
    await tester.enterText(emailField, 'not-an-email');
    await tester.pump();
    expect(find.text('Enter a valid email'), findsOneWidget);

    // Reset email and enter a short password → length error.
    await tester.enterText(emailField, 'user@example.com');
    await tester.enterText(passwordField, 'short');
    await tester.tap(submit);
    await tester.pump();
    expect(find.text('Password must be at least 6 characters'),
        findsOneWidget);

    // Fix password → form passes validation (no error texts remain).
    await tester.enterText(passwordField, 'correct-horse-battery-staple');
    await tester.tap(submit);
    await tester.pump();
    expect(find.text('Email is required'), findsNothing);
    expect(find.text('Password is required'), findsNothing);
    expect(find.text('Enter a valid email'), findsNothing);
    expect(find.text('Password must be at least 6 characters'),
        findsNothing);
  });
}