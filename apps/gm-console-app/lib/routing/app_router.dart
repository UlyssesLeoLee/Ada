import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../features/auth/application/auth_providers.dart';
import '../features/auth/presentation/login_screen.dart';
import '../features/audit/presentation/audit_screen.dart';
import '../features/incidents/presentation/incidents_screen.dart';
import '../features/pipelines/presentation/pipeline_detail_screen.dart';
import '../features/pipelines/presentation/pipelines_screen.dart';
import '../features/settings/presentation/settings_screen.dart';
import '../features/tenants/presentation/tenants_screen.dart';
import '../widgets/error_view.dart';

/// App shell with bottom navigation. The detail screen intentionally lives
/// outside the shell to give it a dedicated AppBar with a back button.
class HomeShell extends ConsumerWidget {
  const HomeShell({super.key, required this.currentLocation});

  final String currentLocation;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selected = _selectedIndexFor(currentLocation);
    return Scaffold(
      body: IndexedStack(
        index: selected,
        children: const [
          TenantsScreen(),
          PipelinesScreen(),
          IncidentsScreen(),
          AuditScreen(),
          SettingsScreen(),
        ],
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: selected,
        onDestinationSelected: (i) => _onTab(context, i),
        destinations: const [
          NavigationDestination(
              icon: Icon(Icons.workspaces_outlined),
              selectedIcon: Icon(Icons.workspaces),
              label: 'Tenants'),
          NavigationDestination(
              icon: Icon(Icons.alt_route_outlined),
              selectedIcon: Icon(Icons.alt_route),
              label: 'Pipelines'),
          NavigationDestination(
              icon: Icon(Icons.warning_amber_outlined),
              selectedIcon: Icon(Icons.warning_amber),
              label: 'Incidents'),
          NavigationDestination(
              icon: Icon(Icons.history_outlined),
              selectedIcon: Icon(Icons.history),
              label: 'Audit'),
          NavigationDestination(
              icon: Icon(Icons.settings_outlined),
              selectedIcon: Icon(Icons.settings),
              label: 'Settings'),
        ],
      ),
    );
  }

  int _selectedIndexFor(String location) {
    if (location.startsWith('/pipelines')) return 1;
    if (location.startsWith('/incidents')) return 2;
    if (location.startsWith('/audit')) return 3;
    if (location.startsWith('/settings')) return 4;
    return 0;
  }

  void _onTab(BuildContext context, int index) {
    switch (index) {
      case 0:
        context.go('/tenants');
        break;
      case 1:
        context.go('/pipelines');
        break;
      case 2:
        context.go('/incidents');
        break;
      case 3:
        context.go('/audit');
        break;
      case 4:
        context.go('/settings');
        break;
    }
  }
}

final routerProvider = Provider<GoRouter>((ref) {
  final notifier = _AuthListenable(ref);
  return GoRouter(
    initialLocation: '/login',
    refreshListenable: notifier,
    redirect: (context, state) {
      final loggedIn = ref.read(authControllerProvider).isAuthenticated;
      final goingToLogin = state.matchedLocation == '/login';
      if (!loggedIn && !goingToLogin) return '/login';
      if (loggedIn && goingToLogin) return '/tenants';
      return null;
    },
    routes: [
      GoRoute(path: '/login', builder: (_, __) => const LoginScreen()),
      GoRoute(
        path: '/tenants',
        builder: (context, state) =>
            HomeShell(currentLocation: state.uri.toString()),
      ),
      GoRoute(
        path: '/pipelines',
        builder: (context, state) =>
            HomeShell(currentLocation: state.uri.toString()),
      ),
      GoRoute(
        path: '/pipelines/:id',
        builder: (context, state) {
          final id = state.pathParameters['id'] ?? '';
          return PipelineDetailScreen(pipelineId: id);
        },
      ),
      GoRoute(
        path: '/incidents',
        builder: (context, state) =>
            HomeShell(currentLocation: state.uri.toString()),
      ),
      GoRoute(
        path: '/audit',
        builder: (context, state) =>
            HomeShell(currentLocation: state.uri.toString()),
      ),
      GoRoute(
        path: '/settings',
        builder: (context, state) =>
            HomeShell(currentLocation: state.uri.toString()),
      ),
    ],
    errorBuilder: (context, state) => Scaffold(
      appBar: AppBar(title: const Text('Not found')),
      body: ErrorView(message: state.error?.toString() ?? 'Unknown route'),
    ),
  );
});

/// Bridges Riverpod auth changes to a [Listenable] for GoRouter refresh.
class _AuthListenable extends ChangeNotifier {
  _AuthListenable(this._ref) {
    _ref.listen(authControllerProvider, (_, __) => notifyListeners());
  }
  final Ref _ref;
}