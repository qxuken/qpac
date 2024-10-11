use crate::instrument::instrumentation::Instrumentation;
use clap::{Parser, Subcommand};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[clap(flatten)]
    pub instrumentation: Instrumentation,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Start http server
    Serve {
        /// Bind ip address
        #[arg(short, long, env = "QPAC_BIND", default_value_t = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080)
        )]
        bind: SocketAddr,

        /// Argon2 PHC or string token for auth puproses
        #[arg(short, long, env = "QPAC_TOKEN")]
        token: Option<String>,
    },

    /// Generate Argon2 PHC token
    Hash { token: String },

    /// Test connection to server
    Add,
}
