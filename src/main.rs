use std::process::ExitCode;

use datuma_k::cli;

#[tokio::main]
async fn main() -> ExitCode {
  match cli::dispatch(std::env::args()).await {
    Ok(()) => ExitCode::SUCCESS,
    Err(err) => {
      if !err.is_reported() {
        eprintln!("{err}");
      }
      ExitCode::FAILURE
    }
  }
}
