//! Static file serving and the SPA fallback.
//!
//! ## Why this file exists
//!
//! `try_disk` and `static_fallback` had no coverage at all — the only
//! test file in this crate exercised the `/api` proxy. So the guard
//! that the source described as "Reject any path-traversal attempts"
//! was never executed, and neither was the much more important thing it
//! was not doing: `GM_CONSOLE_STATIC_DIR` is a *directory*, and every
//! readable file inside it was served by URL, dotfiles included.
//!
//! Confirmed against a running `gm-console-server` before the fix, with
//! `GM_CONSOLE_STATIC_DIR` pointed at a directory containing a `.env`:
//!
//! ```text
//! GET /.env              -> 200  SECRET_KEY=super-secret-value
//! GET /config.json       -> 200  {"db_password":"hunter2"}
//! GET /secrets/prod.yaml -> 200  db_password: hunter2
//! ```
//!
//! `GM_CONSAOLE_STATIC_DIR` is set nowhere in `deploy/`, so this was
//! dormant rather than live. It is one env var away from not being, and
//! the variable exists to be set during development, which is exactly
//! where a `.env` sits next to the build output.
//!
//! The traversal half is tested here too, and the tests document which
//! spellings are inert *and why* — the answer is "because nothing
//! percent-decodes the path", which is a property someone could break
//! while trying to be helpful.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

use gm_console::config::Config;
use gm_console::routes::router;

fn app() -> axum::Router {
    router(std::sync::Arc::new(Config::default()))
}

async fn get(path: &str) -> (axum::http::StatusCode, String, Option<String>) {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("router call");
    let status = resp.status();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .expect("body");
    (status, String::from_utf8_lossy(&body).into_owned(), ct)
}

/// Build a static directory that looks like a real one: an index, an
/// asset, and the secrets that end up beside them.
fn fixture() -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "gm-static-test-{}-{}",
        std::process::id(),
        unique()
    ));
    let pub_dir = base.join("public");
    std::fs::create_dir_all(pub_dir.join("assets")).expect("mkdir");
    std::fs::create_dir_all(pub_dir.join("secrets")).expect("mkdir");
    std::fs::write(pub_dir.join("index.html"), b"PUBLIC-INDEX").expect("write");
    std::fs::write(pub_dir.join("app.css"), b"PUBLIC-CSS").expect("write");
    std::fs::write(pub_dir.join("assets/app.js"), b"PUBLIC-JS").expect("write");
    std::fs::write(pub_dir.join("secrets/prod.yaml"), b"db_password: hunter2").expect("write");
    std::fs::write(pub_dir.join(".env"), b"SECRET_KEY=super-secret-value").expect("write");
    std::fs::write(base.join("OUTSIDE.txt"), b"CANARY-OUTSIDE-THE-STATIC-DIR").expect("write");
    pub_dir
}

/// Cheap uniqueness without pulling in a dependency: the tests in this
/// file share a process and `static_dir` is process-global state.
fn unique() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0)
}

/// `GM_CONSOLE_STATIC_DIR` is read from the environment on every
/// request, so a test that sets it affects the process. These run in one
/// binary; each sets it immediately before its own call and clears it
/// after, which is why they must not be run with `--test-threads`
/// assumptions. Kept in one test to make that explicit.
#[tokio::test]
async fn static_serving_rules() {
    let dir = fixture();
    std::env::set_var("GM_CONSOLE_STATIC_DIR", &dir);
    let dir_str = dir.to_string_lossy().into_owned();

    // ---- the files that should be served -----------------------------
    let (status, body, _) = get("/app.css").await;
    assert_eq!(status, 200);
    assert_eq!(body, "PUBLIC-CSS");

    let (status, body, _) = get("/assets/app.js").await;
    assert_eq!(status, 200);
    assert_eq!(body, "PUBLIC-JS", "a subdirectory asset must still serve");

    // A directory request resolves to its index document.
    let (status, body, _) = get("/").await;
    assert_eq!(status, 200);
    assert_eq!(body, "PUBLIC-INDEX");

    // ---- the regression: dotfiles must not be served ------------------
    // Verified against a running server before the fix: with a `.env`
    // in the static directory, `GET /.env` returned its contents.
    for path in ["/.env", "/.git/config", "/secrets/.aws/credentials"] {
        let (_, body, _) = get(path).await;
        assert!(
            !body.contains("SECRET_KEY") && !body.contains("hunter2"),
            "{path} leaked a secret to the client: {body}"
        );
        // Falls through to the SPA shell, which is the documented
        // behaviour for a path with no file behind it.
        assert!(
            body.contains("<!doctype html") || body.contains("<html"),
            "{path} should have fallen back to the SPA shell, got: {body}"
        );
    }

    // ---- what is NOT a defect, pinned so nobody "fixes" it ------------
    // `GM_CONSOLE_STATIC_DIR` is the public document root. Serving an
    // ordinary file from it — including one in a subdirectory, including
    // a directory that happens to be called `secrets` — is correct
    // behaviour for a static server, and this assertion is here to stop
    // a future reader from blacklisting subdirectory names to make a
    // test pass. The mitigation is not "refuse more paths"; it is that
    // the directory must only ever contain built assets, which is
    // stated on `Config::static_dir`.
    let (status, body, _) = get("/secrets/prod.yaml").await;
    assert_eq!(status, 200);
    assert_eq!(
        body, "db_password: hunter2",
        "a normal file in the document root is served; that is what a static server is for"
    );

    // ---- traversal: refused, and for the stated reason ----------------
    // A `..` segment is rejected. It is inert today because nothing
    // percent-decodes; it must stay inert.
    for path in [
        "/../OUTSIDE.txt",
        "/assets/../../OUTSIDE.txt",
        "/..%2fOUTSIDE.txt",
        "/%2e%2e/OUTSIDE.txt",
        "/%2e%2e%2fOUTSIDE.txt",
    ] {
        let (_, body, _) = get(path).await;
        assert!(
            !body.contains("CANARY-OUTSIDE"),
            "{path} escaped the static directory: {body}"
        );
    }

    // ---- /api never answers with the SPA shell ------------------------
    // `/api/*path` needs a segment, so bare `/api` used to fall through
    // to the fallback and return 200 text/html — an API client would
    // read that as a working endpoint.
    for path in ["/api", "/api/"] {
        let (status, body, ct) = get(path).await;
        assert_eq!(
            status, 404,
            "{path} matched no route and must not report success"
        );
        assert!(
            !body.contains("<!doctype html") && !body.contains("<html"),
            "{path} returned the SPA shell to an API client: {body}"
        );
        assert!(
            ct.as_deref()
                .is_some_and(|c| c.starts_with("application/json")),
            "{path} should answer with the JSON error envelope, got {ct:?}"
        );
    }

    // A real /api path still reaches the proxy, not the fallback.
    let (_, body, _) = get("/api/v1/ping").await;
    assert!(
        !body.contains("<!doctype html"),
        "/api/v1/ping must be proxied, never answered with the SPA"
    );

    std::env::remove_var("GM_CONSOLE_STATIC_DIR");
    let _ = dir_str;
}
