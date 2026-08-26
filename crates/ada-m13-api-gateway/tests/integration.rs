//! Integration tests for the v0.1.0 gateway endpoints.
//!
//! These tests drive the router via `tower::ServiceExt::oneshot`, so
//! they exercise the full axum stack (routing, handler, response
//! shape) without binding a real TCP socket. See
//! [`DOC-MOD-013`](../docs/modules/M-13-api-gateway.md) §3 for the
//! endpoint contracts.

use std::sync::Arc;

use ada_m13_api_gateway::{AppState, MemoryHealthCheck};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn app() -> axum::Router {
    let state = AppState::new("ada-gateway-test", Arc::new(MemoryHealthCheck::new()));
    ada_m13_api_gateway::build_router(state)
}

#[tokio::test]
async fn get_health_returns_json_snapshot() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");

    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.starts_with("application/json"), "content-type was {ct}");

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["status"], "healthy");
    assert_eq!(v["name"], "ada-gateway-test");
    assert!(v["version"].is_string());
    assert!(v["timestamp"].is_number());
}

#[tokio::test]
async fn get_health_live_is_plain_ok() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), b"OK");
}

#[tokio::test]
async fn get_health_ready_is_200_when_healthy() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_api_v1_ping_returns_pong_true() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/ping")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["pong"], true);
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/no/such/path")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router call");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
