//! 黄金集文件加载器.
//!
//! 路径策略:
//! - `FixturePath::Relative("foo/bar.json")` — 解析为 `CARGO_MANIFEST_DIR/tests/fixtures/foo/bar.json`,
//!   即 crate 自己的 `tests/fixtures/` 目录, 跟业务 crate 的 `tests/fixtures` 约定一致.
//! - `FixturePath::Absolute(...)` — 直接用, 用于跨 fixture 引用 (sample 内演示).

use std::fs;
use std::path::PathBuf;

use crate::builders::GoldenEnvelope;
use crate::error::{MockError, Result};

#[derive(Debug, Clone)]
pub enum FixturePath {
    Relative(String),
    Absolute(String),
}

impl FixturePath {
    pub fn relative(p: impl Into<String>) -> Self {
        Self::Relative(p.into())
    }
    pub fn absolute(p: impl Into<String>) -> Self {
        Self::Absolute(p.into())
    }

    pub fn resolve(&self) -> PathBuf {
        match self {
            Self::Relative(p) => {
                let manifest = env!("CARGO_MANIFEST_DIR");
                PathBuf::from(manifest).join("tests").join("fixtures").join(p)
            }
            Self::Absolute(p) => PathBuf::from(p),
        }
    }
}

/// 加载 GoldenEnvelope (含 schema_version 校验).
pub fn load_envelope(path: &FixturePath) -> Result<GoldenEnvelope> {
    let p = path.resolve();
    let raw = fs::read_to_string(&p).map_err(|_| {
        MockError::FixtureNotFound(p.display().to_string())
    })?;
    let env: GoldenEnvelope = serde_json::from_str(&raw)
        .map_err(|e| MockError::FixtureParse(e.to_string()))?;
    env.validate()?;
    Ok(env)
}

/// 加载 NDJSON (每行一条 JSON, 跳过空行与 `//` 注释).
pub fn load_ndjson(path: &FixturePath) -> Result<Vec<serde_json::Value>> {
    let p = path.resolve();
    let raw = fs::read_to_string(&p).map_err(|_| {
        MockError::FixtureNotFound(p.display().to_string())
    })?;
    let mut out = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|e| MockError::FixtureParse(format!("line {}: {}", i + 1, e)))?;
        out.push(v);
    }
    Ok(out)
}
