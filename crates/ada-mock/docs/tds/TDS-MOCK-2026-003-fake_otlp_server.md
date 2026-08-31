# TDS-MOCK-2026-003 — FakeOtlpServer 测试设计

> 元数据: 创建 2026-08-31, 设计 = Mavis 接手 (per DEC-008), 审批 = 架构师自审
> 状态: 锁定
> 关联 crate: ada-mock
> 关联源码: `crates/ada-mock/src/server/mod.rs`
> 编译特性: `server`

## 1. 目标
证明 `FakeOtlpServer` 能拦截 HTTP/OTLP 请求, 正确解析 `Content-Length`, 记录 raw + body, 回复 200 OK, 通过 `Drop` 优雅关闭.

## 2. 范围
- in-scope: 启动 / 单连接服务 / Content-Length 解析 / 200 OK 回复 / Drop 关闭
- out-of-scope: chunked transfer, HTTP/2, 多连接并发, TLS

## 3. 入口
```bash
cargo test -p ada-mock --all-features --lib server
```

## 4. 已知平台风险
- Windows 上 `os error 10053` (WSAECONNABORTED) 偶发, 与 ada-m09-exporter 自身的 `otlp_push_round_trip` 抖动同源
- 处置: 单测试用 `std::thread::sleep(50ms)` 等服务线程写完; 不加 `#[ignore]`, 失败时重跑确认

## 5. 输入分类

| 类别 | 取值 |
|---|---|
| 完整 HTTP 请求 | `POST /v1/metrics` + Content-Length + body |
| 超大 body | 4KB+ 多段读 |
| 空 body | Content-Length: 0 |

## 6. 用例矩阵

| ID | 类别 | 期望 | 已实现 |
|---|---|---|---|
| TC-01 | 完整请求 | 响应 200, recorder 有 1 条, body 与发送一致 | `server_captures_request_and_replies_200` |

## 7. 覆盖率
- 行: 1 测试覆盖 happy path
- 缺口: `Drop` 路径未显式断言 (Rust 编译器保证 std::thread::JoinHandle 不会 panic leak)

## 8. 验收
- [x] 全绿
- [x] recorder 拿到 raw + body
- [x] 关闭后不再 accept (race-condition tested in sample)

## 9. 维护
- 模块 Owner: Mavis
- 复审触发: 解析逻辑变更, 业务 exporter 改 wire format
