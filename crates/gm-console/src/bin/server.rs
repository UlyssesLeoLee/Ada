use gm_console::{serve, Config};

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let cfg = Config::from_env();
    match serve(cfg).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gm-console fatal: {e:?}");
            std::process::ExitCode::FAILURE
        }
    }
}
