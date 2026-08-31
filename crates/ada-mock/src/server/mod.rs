//! `server` feature 模块 — 简单的 TcpListener 风格 mock 服务器.
//!
//! 4 能力层之第 2 层: HTTP / OTLP 拦截.
//!
//! ## 设计边界
//! - **不**使用 hyper / axum / tonic — 它们会拖入 tokio, 违反 crate "纯本地" 边界.
//! - 仅用 `std::net::TcpListener`, 在 `std::thread::spawn` 上同步服务.
//! - 仅识别 `Content-Length` 与 `Connection: close` — 这是 ada-m09-exporter
//!   push exporter 写出来的请求形状 (见 `crates/ada-m09-exporter/src/otlp.rs:566`).
//! - 适用: 单元/集成测试里"我要知道对方发过什么, 然后回复什么".
//!
//! ## 已知限制 (公开声明, sample 测试不依赖):
//! - 不处理 chunked transfer.
//! - 不解析 HTTP method/path (写啥回啥就行).
//! - 同一时间只服务一个连接 (mock 用, 不需要并发).

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, Default)]
pub struct CapturedRequest {
    pub raw: Vec<u8>,
    pub body: Vec<u8>,
}

/// 录制到的请求, FIFO 顺序.
#[derive(Debug, Default, Clone)]
pub struct Recorder {
    inner: Arc<Mutex<Vec<CapturedRequest>>>,
}

impl Recorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, req: CapturedRequest) {
        self.inner.lock().expect("poisoned").push(req);
    }

    pub fn len(&self) -> usize {
        self.inner.lock().expect("poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn drain(&self) -> Vec<CapturedRequest> {
        std::mem::take(&mut *self.inner.lock().expect("poisoned"))
    }
}

/// 启动一个本地 TCP mock 服务器, 用于拦截 HTTP/OTLP 请求.
#[derive(Debug)]
pub struct FakeOtlpServer {
    pub addr: std::net::SocketAddr,
    pub recorder: Recorder,
    join: Option<thread::JoinHandle<()>>,
    /// `true` 表示 drop 时 join 服务线程, 不再接受新连接.
    closed: Arc<std::sync::atomic::AtomicBool>,
}

impl FakeOtlpServer {
    /// 绑定 127.0.0.1 随机端口, 立刻开始服务.
    pub fn start() -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let recorder = Recorder::new();
        let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let rec = recorder.clone();
        let closed_cl = closed.clone();
        let join = thread::spawn(move || {
            // 单连接服务: accept 一次, 然后 exit. 适合"测试一次推送".
            while !closed_cl.load(std::sync::atomic::Ordering::Relaxed) {
                let (stream, _) = match listener.accept() {
                    Ok(p) => p,
                    Err(_) => return,
                };
                if let Some(resp) = handle_one(stream, &rec) {
                    let _ = resp.shutdown(Shutdown::Write);
                }
            }
        });
        Ok(Self {
            addr,
            recorder,
            join: Some(join),
            closed,
        })
    }

    /// 主动关闭 (drop 也会自动关).
    pub fn close(mut self) {
        self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
        // 主动断连以让 accept 循环退出
        let _ = TcpStream::connect(self.addr);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for FakeOtlpServer {
    fn drop(&mut self) {
        self.closed.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = TcpStream::connect(self.addr);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// 处理单次 HTTP 请求: 读取到 body 完, 记录 raw + body, 回复 200 OK + Content-Length: 0.
/// 返回已半关闭的 stream (Write 已 shutdown, 客户端可读到 EOF).
fn handle_one(mut stream: TcpStream, rec: &Recorder) -> Option<TcpStream> {
    let mut buf = [0u8; 4096];
    let mut raw = Vec::new();
    let mut body = Vec::new();
    let mut content_length: Option<usize> = None;

    // 简化解析: 一直读到 "\r\n\r\n", 解析 Content-Length 头, 再读 body.
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) | Err(_) => return None,
            Ok(n) => n,
        };
        raw.extend_from_slice(&buf[..n]);
        // 找 header 结束
        if let Some(idx) = find_double_crlf(&raw) {
            // 解析 Content-Length
            if content_length.is_none() {
                let head = std::str::from_utf8(&raw[..idx]).unwrap_or("");
                content_length = parse_content_length(head);
            }
            // 收集 body
            let body_start = idx + 4;
            let already = raw.len() - body_start;
            body.extend_from_slice(&raw[body_start..]);
            let need = match content_length {
                Some(c) => c,
                None => break,
            };
            if already >= need {
                break;
            }
            // 续读直到 need
            while body.len() < need {
                let n = match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                body.extend_from_slice(&buf[..n]);
            }
            break;
        }
        if raw.len() > 16 * 1024 {
            // 防御: header 太大, 直接退出
            break;
        }
    }

    rec.push(CapturedRequest { raw, body });

    let _ = stream.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    Some(stream)
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_content_length(headers: &str) -> Option<usize> {
    for line in headers.split("\r\n") {
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            return rest.trim().parse().ok();
        }
        if let Some(rest) = line.strip_prefix("content-length:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_captures_request_and_replies_200() {
        let srv = FakeOtlpServer::start().expect("start");
        // 用 std::net 起一个最小客户端
        let mut s = TcpStream::connect(srv.addr).expect("connect");
        let body = b"{\"hello\":\"world\"}";
        let req = format!(
            "POST /v1/metrics HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        s.write_all(req.as_bytes()).unwrap();
        s.write_all(body).unwrap();
        s.shutdown(Shutdown::Write).unwrap();

        // 读响应直到 EOF
        let mut resp = Vec::new();
        let _ = s.read_to_end(&mut resp);
        assert!(resp.starts_with(b"HTTP/1.1 200"));

        // 服务线程已经把这次请求录下
        // 给 server 一小段窗口把请求写进 recorder
        std::thread::sleep(std::time::Duration::from_millis(50));
        let captured = srv.recorder.drain();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].body, body);
        srv.close();
    }
}
