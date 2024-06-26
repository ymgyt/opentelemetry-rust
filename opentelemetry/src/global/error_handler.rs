use std::sync::PoisonError;
use std::sync::RwLock;

#[cfg(feature = "logs")]
use crate::logs::LogError;
#[cfg(feature = "metrics")]
use crate::metrics::MetricsError;
#[cfg(feature = "trace")]
use crate::trace::TraceError;
use once_cell::sync::Lazy;

static GLOBAL_ERROR_HANDLER: Lazy<RwLock<Option<ErrorHandler>>> = Lazy::new(|| RwLock::new(None));

/// Wrapper for error from both tracing and metrics part of open telemetry.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[cfg(feature = "trace")]
    #[cfg_attr(docsrs, doc(cfg(feature = "trace")))]
    #[error(transparent)]
    /// Failed to export traces.
    Trace(#[from] TraceError),
    #[cfg(feature = "metrics")]
    #[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
    #[error(transparent)]
    /// An issue raised by the metrics module.
    Metric(#[from] MetricsError),

    #[cfg(feature = "logs")]
    #[cfg_attr(docsrs, doc(cfg(feature = "logs")))]
    #[error(transparent)]
    /// Failed to export logs.
    Log(#[from] LogError),

    #[error("{0}")]
    /// Other types of failures not covered by the variants above.
    Other(String),
}

impl<T> From<PoisonError<T>> for Error {
    fn from(err: PoisonError<T>) -> Self {
        Error::Other(err.to_string())
    }
}

struct ErrorHandler(Box<dyn Fn(Error) + Send + Sync>);

/// Handle error using the globally configured error handler.
///
/// Writes to stderr if unset.
pub fn handle_error<T: Into<Error>>(err: T) {
    match GLOBAL_ERROR_HANDLER.read() {
        Ok(handler) if handler.is_some() => (handler.as_ref().unwrap().0)(err.into()),
        _ => match err.into() {
            #[cfg(feature = "metrics")]
            #[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
            Error::Metric(err) => eprintln!("OpenTelemetry metrics error occurred. {}", err),
            #[cfg(feature = "trace")]
            #[cfg_attr(docsrs, doc(cfg(feature = "trace")))]
            Error::Trace(err) => eprintln!("OpenTelemetry trace error occurred. {}", err),
            #[cfg(feature = "logs")]
            #[cfg_attr(docsrs, doc(cfg(feature = "logs")))]
            Error::Log(err) => eprintln!("OpenTelemetry log error occurred. {}", err),
            Error::Other(err_msg) => eprintln!("OpenTelemetry error occurred. {}", err_msg),
        },
    }
}

/// Set global error handler.
pub fn set_error_handler<F>(f: F) -> std::result::Result<(), Error>
where
    F: Fn(Error) + Send + Sync + 'static,
{
    GLOBAL_ERROR_HANDLER
        .write()
        .map(|mut handler| *handler = Some(ErrorHandler(Box::new(f))))
        .map_err(Into::into)
}
