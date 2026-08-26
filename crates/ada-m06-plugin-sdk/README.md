# ada-m06-plugin-sdk

M-06: Plugin SDK. 3 `PluginKind` (`Wasm` / `Native` / `Script`),
`PluginManifest`, `PluginHost` trait, `InMemoryHost` impl, and
a declarative capability-based `SandboxPolicy`.

See `docs/modules/M-06-plugin-sdk.md` (DOC-MOD-006) for the
full design.

## v0.1.0 status

Skeleton. Real WASM/native/script execution lands in B7+.

## v0.1.0 surface

- `PluginKind` — `Wasm | Native | Script`
- `PluginManifest` — id, name, version, kind, capabilities, entry_point, hash, signature
- `PluginHost` trait — `install / uninstall / invoke / list`
- `InMemoryHost` — process-local registry
- `SandboxPolicy` — allow-list of capability strings + `ResourceLimits`
- 5-variant `SdkError` — `PluginNotFound / ManifestInvalid / CapabilityDenied / HashMismatch / BackendError`
- 30 unit tests + 4 integration tests
