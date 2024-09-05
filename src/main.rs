use anyhow::Result;
use lwd_warp::cli::cli_main;

pub fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    cli_main()?;
    Ok(())
}
