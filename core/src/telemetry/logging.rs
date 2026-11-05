//! Structured logging for the Atoll Protocol node.
//!
//! JSON-formatted structured logs via [`tracing`] and [`tracing-subscriber`].
//! Log levels are controlled via the `RUST_LOG` environment variable
//! (default `info`). Logs are written to stdout by default
//! (container-friendly) or to a rotating file via [`tracing-appender`].

use std::io;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Output destination for log records.
#[derive(Debug, Clone)]
pub enum LogOutput {
    /// Write JSON logs to stdout.
    Stdout,
    /// Write JSON logs to a rolling file in `directory` with `file_stem`.
    File {
        directory: String,
        file_stem: String,
    },
}

/// Keeps the non-blocking file writer guard alive for the process lifetime.
///
/// Dropping this guard stops the background flush thread, so callers must hold
/// it for the duration of the program.
pub struct LogGuard {
    #[allow(dead_code)]
    guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}

impl LogGuard {
    /// No-op guard used for stdout output; nothing to keep alive.
    fn none() -> Self {
        Self { guard: None }
    }
}

/// Builds the [`EnvFilter`] from the `RUST_LOG` environment variable.
///
/// Falls back to `info` when `RUST_LOG` is unset or unparseable.
fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

/// Initialises the global tracing subscriber with JSON formatting.
///
/// # Errors
///
/// Returns an [`std::io::Error`] if the log directory cannot be created or a
/// file appender cannot be opened.
pub fn init(output: LogOutput) -> io::Result<LogGuard> {
    match output {
        LogOutput::Stdout => {
            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .json()
                        .with_ansi(false)
                        .with_filter(env_filter()),
                )
                .init();
            Ok(LogGuard::none())
        }
        LogOutput::File {
            directory,
            file_stem,
        } => {
            std::fs::create_dir_all(&directory)?;
            let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix(file_stem)
                .build(directory)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .json()
                        .with_ansi(false)
                        .with_writer(non_blocking)
                        .with_filter(env_filter()),
                )
                .init();
            Ok(LogGuard { guard: Some(guard) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_output_stdout_variant() {
        assert!(matches!(LogOutput::Stdout, LogOutput::Stdout));
    }

    #[test]
    fn log_output_file_variant_holds_fields() {
        let output = LogOutput::File {
            directory: "/var/log/atoll".to_string(),
            file_stem: "node".to_string(),
        };
        match output {
            LogOutput::File {
                directory,
                file_stem,
            } => {
                assert_eq!(directory, "/var/log/atoll");
                assert_eq!(file_stem, "node");
            }
            LogOutput::Stdout => panic!("expected File variant"),
        }
    }
}
