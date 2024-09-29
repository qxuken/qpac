use crate::constants::PACKAGE_NAME;
use std::io::IsTerminal;
use tracing::Subscriber;
use tracing_subscriber::{
    filter::Directive,
    layer::{Layer, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    EnvFilter,
};

use super::logger::Logger;

#[derive(clap::Args, Debug, Default, Clone)]
pub struct Instrumentation {
    /// Enable debug logs, -vv for trace
    #[clap(
        short = 'v',
        long,
        env = "QPAC_VERBOSITY",
        action = clap::ArgAction::Count,
        global = true,
    )]
    pub verbose: u8,

    /// Which tracing-setup to use
    #[clap(
        long,
        env = "QPAC_LOGGER",
        default_value_t = Default::default(),
        global = true
    )]
    pub logger: Logger,

    /// Tracing directives
    ///
    /// See <https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives>
    #[clap(
        long = "log-directive",
        env = "QPAC_LOG_DIRECTIVES",
        value_delimiter = ',',
        num_args = 0..,
        global = true
    )]
    pub log_directives: Vec<Directive>,
}

impl Instrumentation {
    pub fn log_level(&self) -> String {
        match self.verbose {
            0 => "info",
            1 => "debug",
            _ => "trace",
        }
        .to_string()
    }

    pub fn setup(&self, packages: &[&str]) -> color_eyre::Result<()> {
        let filter_layer = self.filter_layer(packages)?;

        let registry = tracing_subscriber::registry()
            .with(filter_layer)
            .with(tracing_error::ErrorLayer::default());

        // `try_init` called inside `match` since `with` changes the type
        match self.logger {
            Logger::Compact => registry.with(self.fmt_layer_compact()).try_init()?,
            Logger::Full => registry.with(self.fmt_layer_full()).try_init()?,
            Logger::Pretty => registry.with(self.fmt_layer_pretty()).try_init()?,
            Logger::Json => registry.with(self.fmt_layer_json()).try_init()?,
        }

        Ok(())
    }

    pub fn filter_layer(&self, packages: &[&str]) -> color_eyre::Result<EnvFilter> {
        let mut filter_layer = {
            if self.log_directives.is_empty() {
                let log_level = self.log_level();
                let default = packages
                    .iter()
                    .map(|p| p.replace('-', "_"))
                    .map(|p| format!("{p}={log_level}"))
                    .fold(format!("{}={log_level}", PACKAGE_NAME), |mut acc, p| {
                        acc.push_str(&format!(",{p}"));
                        acc
                    });
                EnvFilter::try_new(default)?
            } else {
                EnvFilter::try_new("")?
            }
        };

        for directive in &self.log_directives {
            let directive_clone = directive.clone();
            filter_layer = filter_layer.add_directive(directive_clone);
        }

        Ok(filter_layer)
    }

    pub fn fmt_layer_full<S>(&self) -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        tracing_subscriber::fmt::Layer::new()
            .with_ansi(std::io::stderr().is_terminal())
            .with_writer(std::io::stderr)
    }

    pub fn fmt_layer_pretty<S>(&self) -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        tracing_subscriber::fmt::Layer::new()
            .with_ansi(std::io::stderr().is_terminal())
            .with_writer(std::io::stderr)
            .pretty()
    }

    pub fn fmt_layer_json<S>(&self) -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        tracing_subscriber::fmt::Layer::new()
            .with_ansi(std::io::stderr().is_terminal())
            .with_writer(std::io::stderr)
            .json()
    }

    pub fn fmt_layer_compact<S>(&self) -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        tracing_subscriber::fmt::Layer::new()
            .with_ansi(std::io::stderr().is_terminal())
            .with_writer(std::io::stderr)
            .compact()
            .without_time()
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false)
    }
}
