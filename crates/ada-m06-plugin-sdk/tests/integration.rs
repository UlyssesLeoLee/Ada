//! End-to-end integration tests for the M-06 plugin SDK.

use ada_m06_plugin_sdk::{InMemoryHost, PluginHost, PluginKind, PluginManifest, SandboxPolicy};
use serde_json::json;

#[tokio::test]
async fn install_list_uninstall_round_trip() {
    let host = InMemoryHost::default();
    let m = PluginManifest::new("foo", "0.1.0", PluginKind::Wasm).with_entry_point("foo:run");
    let id = host.install(m).await.expect("install");

    let list = host.list().await.expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, id);
    assert_eq!(list[0].name, "foo");

    host.uninstall(id).await.expect("uninstall");
    let list = host.list().await.expect("list");
    assert!(list.is_empty(), "expected empty after uninstall");
}

#[tokio::test]
async fn capability_policy_blocks_unauthorized_invoke() {
    let host = InMemoryHost::new(SandboxPolicy {
        allowed_capabilities: vec!["fs:read:/tmp".into()],
        resource_limits: ada_m06_plugin_sdk::ResourceLimits::default(),
    });
    let m = PluginManifest::new("bar", "0.1.0", PluginKind::Native)
        .with_entry_point("bar:run")
        .with_capability("fs:write:/etc/passwd");
    let id = host.install(m).await.expect("install");
    let err = host.invoke(id, json!({})).await.unwrap_err();
    assert!(
        matches!(err, ada_m06_plugin_sdk::SdkError::CapabilityDenied { .. }),
        "expected CapabilityDenied, got {err:?}"
    );
}

#[tokio::test]
async fn capability_policy_permits_authorized_invoke() {
    let host = InMemoryHost::new(SandboxPolicy {
        allowed_capabilities: vec!["net:connect:api.example.com".into()],
        resource_limits: ada_m06_plugin_sdk::ResourceLimits::default(),
    });
    let m = PluginManifest::new("baz", "0.1.0", PluginKind::Script)
        .with_entry_point("baz:run")
        .with_capability("net:connect:api.example.com");
    let id = host.install(m).await.expect("install");
    let out = host.invoke(id, json!({"x": 1})).await.expect("invoke");
    assert_eq!(out, serde_json::Value::Null);
}

#[tokio::test]
async fn multiple_plugins_with_overlapping_capabilities() {
    let host = InMemoryHost::new(SandboxPolicy {
        allowed_capabilities: vec!["net:connect".into()],
        resource_limits: ada_m06_plugin_sdk::ResourceLimits::default(),
    });
    let m1 = PluginManifest::new("a", "0.1.0", PluginKind::Wasm)
        .with_entry_point("a:r")
        .with_capability("net:connect");
    let m2 = PluginManifest::new("b", "0.2.0", PluginKind::Native)
        .with_entry_point("b:r")
        .with_capability("net:connect");
    let id1 = host.install(m1).await.expect("install 1");
    let id2 = host.install(m2).await.expect("install 2");
    assert_ne!(id1, id2);
    let list = host.list().await.expect("list");
    assert_eq!(list.len(), 2);
    host.invoke(id1, json!(null)).await.expect("invoke 1");
    host.invoke(id2, json!(null)).await.expect("invoke 2");
}
