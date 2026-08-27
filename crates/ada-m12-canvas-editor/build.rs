//! Build script for `ada-m12-canvas-editor`.
//!
//! 仅在编译时给 Cargo 打 `cfg` 标志,不执行任何编译期副作用。
//! 真正的 WASM / Bevy 编译路径由 Cargo features 控制。
//!
//! 提示用户在尝试 `wasm-pack build` 之前先安装
//! `wasm32-unknown-unknown` target(否则 wasm-pack 报
//! "target not installed" 错误)。此 build script 不会
//! 自动安装,避免静默网络操作。

use std::env;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // When the user tries to build for wasm32-* without the target
    // installed, rustc will already complain loudly. We add a more
    // actionable hint to the build log.
    if target.starts_with("wasm32") && target_os != "emscripten" {
        println!("cargo:warning=building ada-m12-canvas-editor for {target}");
        println!("cargo:warning=if this is the first time, run:");
        println!("cargo:warning=    rustup target add {target}");
    }

    // Re-run only if the build script itself changes (no inputs).
    println!("cargo:rerun-if-changed=build.rs");
}
