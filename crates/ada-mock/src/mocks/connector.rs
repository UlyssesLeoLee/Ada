//! StubConnector — 业务 ada-m01 acquisition 的"形状相似"克隆.
//!
//! 业务版本有 `FileConnector / StdinConnector / HttpConnector` 三个具体
//! 实现, 这里合并为单一 `StubConnector` + `StubKind` 枚举, 便于测试
//! 构造不同数据源.
//!
//! ## 与业务版接口差异
//! - 业务版 `Connector` 是 trait + `async fn read()`. 本 mock 提供**同步**
//!   `read_all() -> Vec<Record>`, 因为我们不允许 tokio 依赖.
//! - "HTTP" 模式: 从内存里的 JSON 列表按行返回, 模拟分批.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StubKind {
    /// 模拟 `cat file.ndjson | connector`
    Stdin,
    /// 模拟 `FileConnector::open(path)`
    File,
    /// 模拟 `HttpConnector` (从内置 JSON 列表逐行返回)
    Http,
}

impl StubKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stdin => "stdin",
            Self::File => "file",
            Self::Http => "http",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Record {
    pub id: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct StubConnector {
    kind: StubKind,
    records: Vec<Record>,
    cursor: usize,
    /// 故意注入的失败次数 (用于测试错误路径), 0 = 不注入.
    fail_times: usize,
}

impl StubConnector {
    /// 新建: 按 `kind` 选数据形状.
    pub fn new(kind: StubKind) -> Self {
        Self {
            kind,
            records: Vec::new(),
            cursor: 0,
            fail_times: 0,
        }
    }

    /// 链式: 预置一批 record.
    pub fn with_records(mut self, records: Vec<Record>) -> Self {
        self.records = records;
        self
    }

    /// 链式: 让前 N 次 `read_all` 报 `Err`, 之后正常 — 用于重试/退避测试.
    pub fn with_transient_failures(mut self, n: usize) -> Self {
        self.fail_times = n;
        self
    }

    pub fn kind(&self) -> StubKind {
        self.kind
    }

    /// 一次性读完全部. 同步, 无 I/O.
    pub fn read_all(&mut self) -> Result<Vec<Record>, String> {
        if self.fail_times > 0 {
            self.fail_times -= 1;
            return Err(format!("simulated transient failure (remaining={})", self.fail_times));
        }
        // cursor 仅用于"已读"指示; mock 不真正消费, 但提供 count 便于断言.
        let batch = self.records[self.cursor..].to_vec();
        self.cursor = self.records.len();
        Ok(batch)
    }

    /// 已读 record 数.
    pub fn read_count(&self) -> usize {
        self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, val: i64) -> Record {
        Record {
            id: id.into(),
            payload: serde_json::json!({ "v": val }),
        }
    }

    #[test]
    fn kind_as_str_matches_variant() {
        assert_eq!(StubKind::Stdin.as_str(), "stdin");
        assert_eq!(StubKind::File.as_str(), "file");
        assert_eq!(StubKind::Http.as_str(), "http");
    }

    #[test]
    fn read_all_returns_preset_records() {
        let mut c = StubConnector::new(StubKind::Http)
            .with_records(vec![rec("a", 1), rec("b", 2)]);
        let got = c.read_all().expect("ok");
        assert_eq!(got.len(), 2);
        assert_eq!(c.read_count(), 2);
    }

    #[test]
    fn transient_failures_injected_before_success() {
        let mut c = StubConnector::new(StubKind::File)
            .with_records(vec![rec("a", 1)])
            .with_transient_failures(2);
        assert!(c.read_all().is_err());
        assert!(c.read_all().is_err());
        assert!(c.read_all().is_ok());
    }

    #[test]
    fn read_count_increments_only_on_success() {
        let mut c = StubConnector::new(StubKind::Stdin)
            .with_records(vec![rec("a", 1)])
            .with_transient_failures(1);
        let _ = c.read_all(); // 失败, 不增
        assert_eq!(c.read_count(), 0);
        let _ = c.read_all(); // 成功
        assert_eq!(c.read_count(), 1);
    }
}
