use clap::Parser;

use args::Args;

mod args;
mod constants;
mod error;
mod instrument;
mod pac;
mod storage;
mod trace_layer;
mod utils;
mod web;

#[tokio::main]
async fn main() -> error::Result<()> {
    utils::color_eyre::setup()?;

    let args = Args::parse();
    args.instrumentation.setup(&[])?;

    tracing::trace!("{:?}", args);

    match args.command {
        args::Command::Serve { bind } => {
            web::run_web_server(bind).await?;
        }
        args::Command::Add => {
            unimplemented!();
        }
    }

    Ok(())
}
