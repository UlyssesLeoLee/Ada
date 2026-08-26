# `ada-m13-api-gateway` (M-13 API Gateway)

> M-13 入口网关 crate — v0.1.0 骨架实现(B2)
> 設計書:`docs/modules/M-13-api-gateway.md` (DOC-MOD-013)

## v0.1.0 提供的能力

| 端点 | 方法 | 响应 | 说明 |
|---|---|---|---|
| `/health` | GET | 200 JSON | 仪表盘快照,包含 `status` / `name` / `version` / `timestamp` |
| `/health/live` | GET | 200 `OK` | 存活探针,纯文本 |
| `/health/ready` | GET | 200 / 503 | 就绪探针,委托给 [`HealthCheck`] |
| `/api/v1/ping` | GET | 200 JSON `{ "pong": true }` | 部署流水线烟测 |

B2 范围**不**包含的生产中间件(CORS / HSTS / JWT / 租户 / RBAC)见
`docs/modules/M-13-api-gateway.md` §3.1,会在 B3+ 加入。

## 公共 API

```rust
use std::sync::Arc;
use ada_m13_api_gateway::{AppState, MemoryHealthCheck, build_router};

let state = AppState::new("ada-gateway", Arc::new(MemoryHealthCheck::new()));
let app = build_router(state);
```

在自己的 axum 主程序里挂载:

```rust
use ada_m13_api_gateway::{build_router, AppState, MemoryHealthCheck};
use std::sync::Arc;
use axum::Router;

#[tokio::main]
async fn main() {
    let state = AppState::new("ada-gateway", Arc::new(MemoryHealthCheck::new()));
    let gateway = build_router(state);

    // 在主程序里把 M-13 作为子路由挂到根路径
    let app = Router::new().nest("/", gateway);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## 错误映射

[`ApiError`] 直接实现 `IntoResponse`,各变体映射的 HTTP 状态码:

| 变体 | HTTP |
|---|---|
| `NotFound` | 404 |
| `BadRequest` | 400 |
| `Unauthorized` | 401 |
| `ServiceUnavailable` | 503 |
| `Internal` | 500 |

完整错误码表见 `docs/api/error-codes.md` §2。

## 测试

```bash
cargo test -p ada-m13-api-gateway
```

- 12 个单元测试(error / health / state / router)
- 5 个集成测试(用 `tower::ServiceExt::oneshot` 跑四个端点 + 未知路径)
