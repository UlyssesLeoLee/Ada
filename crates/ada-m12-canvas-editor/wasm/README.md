# M-12 Canvas Editor — WASM build harness

> 适用范围: `crates/ada-m12-canvas-editor` 编译为 `wasm32-unknown-unknown` WebAssembly,供前端(`docs/modules/M-12-canvas-editor-frontend.md`)通过 JS 调用。

---

## 1. 工具链前置 (host prerequisites)

> ⚠️ **host 工具链需要本机未装时,先告知用户,不要自动 install。**

```bash
# 1) Rust target
rustup target add wasm32-unknown-unknown

# 2) wasm-pack (官方安装脚本)
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
# 或 cargo install wasm-pack (无 cargo binstall 时)

# 3) 验证
wasm-pack --version
rustup target list --installed | grep wasm32
```

如果 host 是 Windows,优先用 `cargo install wasm-pack` 或 `winget install RustFoundation.WasmPack`(后者需要用户授权)。

---

## 2. 构建命令

### 2.1 标准 Web target (browser ESM)

```bash
cd crates/ada-m12-canvas-editor
wasm-pack build --target web --release --features wasm
# 产出: ./pkg/
#   ada_m12_canvas_editor.js     ESM JS 胶水层
#   ada_m12_canvas_editor_bg.wasm WASM 二进制 (D-05 8 MB 上限)
#   ada_m12_canvas_editor.d.ts   TS 类型声明
#   package.json                 wasm-pack 提供的 package metadata
```

### 2.2 浏览器内单元测试 (wasm-bindgen-test)

```bash
wasm-pack test --headless --chrome -p ada-m12-canvas-editor \
    --features wasm-test
# 也可用 --firefox / --safari,需要 host 装对应浏览器
```

### 2.3 Node.js target (SSR / 集成测试)

```bash
wasm-pack build --target nodejs --release --features wasm
```

---

## 3. JS 侧接入示例

```js
// app.js (ESM)
import init, { WasmCanvas } from "@ada/m12-canvas-editor";

await init();
const canvas = new WasmCanvas("my-flow-1");

canvas.addNodeJson(JSON.stringify({
    id: crypto.randomUUID(),
    kind: "Block",
    position: { x: 100, y: 200 },
    label: "data-source",
    ports: [{ name: "out" }],
}));

console.log(canvas.version); // 1
console.log(canvas.toJson()); // full snapshot
```

---

## 4. 尺寸验证 (D-05: WASM 8 MB / gzip 3 MB)

`build.sh` 在 `wasm-pack build` 之后跑尺寸检查,失败 exit 1。

```bash
cd crates/ada-m12-canvas-editor
./wasm/build.sh
```

阈值(per `docs/decisions/02-design-adrs.md` D-05):

| 指标 | 上限 |
|---|---|
| Raw `.wasm` | 8 MB |
| Gzip `.wasm` | 3 MB |

实际 baseline 在 `wasm-pack 0.13` + Rust 1.98 + Bevy feature 关闭时,
`ada_m12_canvas_editor_bg.wasm` 约 1.0–1.5 MB (raw) / 0.4–0.6 MB (gzip)。
启用 `--features bevy` 时会显著增大(bevy_ecs + bevy_app 单独约 +6 MB),
实际前端 bundle 通过 `wasm-opt -O3` + feature 裁剪控制在 8 MB 内。

---

## 5. 已知缺口 (per DDD Review checklist, "缺标比错标安全")

- 当前 `pkg/` 不在 `.gitignore` 内的精确路径 — 用户应在
  `crates/ada-m12-canvas-editor/.gitignore` 加 `pkg/`(本目录
  README 仅描述构建,不动 `.gitignore`)。
- `bevy_ecs` / `bevy_app` 没有禁用 internal feature list — 若
  发现 `pkg/*.wasm` 突破 8 MB,需收紧 `default-features = false`
  并按需开启 `bevy_ecs` 的 `parallel` / `trace` 等子 feature。
- `wasm-opt` 默认未跑 — `wasm-pack build` 默认调用
  `wasm-opt -O3` (binaryen),host 需装 binaryen。

---

## 6. 文件清单 (本子目录)

| 文件 | 作用 |
|---|---|
| `build.sh` | 一键 `wasm-pack build` + 尺寸校验 |
| `test.sh` | 一键 `wasm-pack test --headless --chrome` |
| `size-check.sh` | 单独跑尺寸校验(可在 CI 用) |
| `README.md` | 本文件 |
| `package.json.tmpl` | 模板,给 host 项目复制后改 name |
