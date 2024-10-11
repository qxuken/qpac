use argon2::{
    password_hash::SaltString, Algorithm, Argon2, ParamsBuilder, PasswordHasher, Version,
};
use clap::Parser;

use args::Args;
use ring::rand::{SecureRandom, SystemRandom};
use tracing::{debug, trace};

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
    args.instrumentation.setup(&["sqlx"])?;

    tracing::trace!("{:?}", args);

    match args.command {
        args::Command::Serve {
            bind,
            token,
            database,
        } => {
            web::run_web_server(bind, token, database).await?;
        }
        args::Command::Hash { token } => {
            let hash = generate_hash(token.as_bytes());
            println!("{hash}");
        }
        args::Command::Add => {
            unimplemented!();
        }
    }

    Ok(())
}

fn generate_hash(token: &[u8]) -> String {
    let mut params = ParamsBuilder::new();
    params.m_cost(65540).t_cost(3).p_cost(4);
    debug!("Generating with params {:?}", &params);

    let argon = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        params.build().expect("Should build params"),
    );
    trace!("Argon2 {:?}", argon);

    let mut buf = [0; 32];
    SystemRandom::new()
        .fill(&mut buf)
        .expect("Error generating random values");
    let salt = SaltString::encode_b64(&buf).unwrap();
    trace!("Generated salt");

    argon
        .hash_password(token, &salt)
        .expect("Hashed password")
        .to_string()
}
