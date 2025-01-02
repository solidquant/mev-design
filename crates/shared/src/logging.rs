use std::path::Path;

use const_format::formatcp;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

const DEFAULT_FILE_DIRECTIVE: &str = formatcp!("info,{}=debug", env!("CARGO_PKG_NAME"));
const LOG_FILE_NAME: &str = formatcp!("{}.log", env!("CARGO_PKG_NAME"));

pub fn setup_tracing(
    log_directory: Option<&Path>,
    log_file_name: Option<&str>,
) -> Option<WorkerGuard> {
    let stdout_filter = EnvFilter::builder()
        .with_env_var("RUST_LOG")
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .unwrap();
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(stdout_filter);

    let (file_layer, file_guard) = log_directory
        .map(|directory| {
            let log_file_name = log_file_name.unwrap_or(LOG_FILE_NAME);
            let file_appender = tracing_appender::rolling::hourly(directory, log_file_name);
            let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);

            let file_filter = std::env::var("RUST_FILE_LOG").ok();
            let file_filter = file_filter.as_deref().unwrap_or(DEFAULT_FILE_DIRECTIVE);
            let file_filter: EnvFilter = file_filter.parse().unwrap();

            let file_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_writer(file_writer)
                .with_filter(file_filter);

            (Some(file_layer), Some(file_guard))
        })
        .unwrap_or((None, None));

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .init();

    file_guard
}
