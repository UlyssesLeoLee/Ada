//! Integration smoke — 同时演示 ada-mock 的 4 能力层.
//!
//! 这是**示例测试** (sample), 展示: 拿到一个 mock crate 后, 怎么用
//! builder / fixture / mock / server 写一个 5 步场景. 后续测试可
//! 直接 copy 改写.
//!
//! 场景: "scheduler 入队 3 个 job, 中途通过 event bus 收到 cancel 事件,
//! 第 2 个 job 转 Cancelled, 剩余两个进入 Running 直到 Succeeded, 同时
//! 把指标通过 FakeOtlpServer 推出去, 由测试断言推送 body 形状."

use ada_mock::builders::{EventBuilder, GoldenEnvelope, JobBuilder};
use ada_mock::fixtures::{golden_event, load_envelope, load_ndjson, FixturePath};
use ada_mock::mocks::{InMemoryEventBus, InMemoryScheduler, JobState, StubConnector, StubKind};

#[test]
fn four_layer_smoke() {
    // ----- 1) fixture 加载黄金集 (能力层 3) -----
    let envelope: GoldenEnvelope = load_envelope(&FixturePath::relative(
        "events_basic.envelope.json",
    ))
    .expect("load envelope");
    assert_eq!(envelope.schema_version, 1);
    assert_eq!(envelope.events.len(), 3);

    // NDJSON 路径
    let records = load_ndjson(&FixturePath::relative("acquire_records.ndjson")).expect("ndjson");
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["id"], "rec-001");

    // ----- 2) Mock 资源 (能力层 1) -----
    let mut sched = InMemoryScheduler::with_capacity(4);
    let bus = InMemoryEventBus::new();

    // 入队 3 个 job
    let j1 = JobBuilder::new("ingest").enqueue(&mut sched).expect("j1");
    let j2 = JobBuilder::new("transform").enqueue(&mut sched).expect("j2");
    let j3 = JobBuilder::new("export").enqueue(&mut sched).expect("j3");
    assert_eq!(sched.in_flight(), 3);

    // ----- 3) 通过 bus 发布 cancel 事件, 业务方会消费 (这里直接验证录下) -----
    let sub = bus.subscribe("ada.job.cancel").expect("sub");
    let _ev = EventBuilder::new("ada.job.cancel")
        .with_payload(serde_json::json!({ "job_id": j2.id }))
        .with_trace_id("trace-cancel-1")
        .publish(&bus)
        .expect("publish");
    let got = bus.try_recv(sub).expect("recv").expect("one event");
    assert_eq!(got.payload["job_id"], serde_json::json!(j2.id.to_string()));

    // ----- 4) 推进 job 状态 -----
    sched.transition(j1.id, JobState::Queued).unwrap();
    sched.transition(j1.id, JobState::Running).unwrap();
    sched.transition(j1.id, JobState::Succeeded).unwrap();

    sched.transition(j2.id, JobState::Cancelled).unwrap(); // 模拟 cancel 生效
    assert_eq!(sched.state_of(j2.id).unwrap(), JobState::Cancelled);

    sched.transition(j3.id, JobState::Queued).unwrap();
    sched.transition(j3.id, JobState::Running).unwrap();

    // in_flight 应当: j1 终态释放, j2 终态释放, j3 仍 Running = 1
    assert_eq!(sched.in_flight(), 1);

    // ----- 5) StubConnector 一次性读全部 -----
    let mut c = StubConnector::new(StubKind::Http)
        .with_records(
            records
                .iter()
                .map(|v| ada_mock::mocks::Record {
                    id: v["id"].as_str().unwrap().to_string(),
                    payload: v["payload"].clone(),
                })
                .collect(),
        );
    let got = c.read_all().expect("read");
    assert_eq!(got.len(), 3);
    assert_eq!(c.read_count(), 3);

    // ----- 6) 黄金事件工厂对齐 -----
    let g = golden_event("ada.job.scheduled", 1);
    assert_eq!(g["topic"], "ada.job.scheduled");
    assert_eq!(g["payload"]["seq"], 1);
}

#[cfg(feature = "server")]
#[test]
fn four_layer_with_otlp_capture() {
    use ada_mock::server::FakeOtlpServer;

    let srv = FakeOtlpServer::start().expect("start otlp mock");
    let addr = srv.addr;

    // 同步 TCP 客户端, 模拟 OTLP/HTTP push
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpStream};

    let mut s = TcpStream::connect(addr).expect("connect");
    let body = serde_json::to_vec(&golden_event("ada.metric.tick", 7)).unwrap();
    let head = format!(
        "POST /v1/metrics HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    s.write_all(head.as_bytes()).unwrap();
    s.write_all(&body).unwrap();
    s.shutdown(Shutdown::Write).unwrap();

    let mut resp = Vec::new();
    let _ = s.read_to_end(&mut resp);
    assert!(resp.starts_with(b"HTTP/1.1 200"));

    // 等服务线程写完
    std::thread::sleep(std::time::Duration::from_millis(50));
    let captured = srv.recorder.drain();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].body, body);
    srv.close();
}
