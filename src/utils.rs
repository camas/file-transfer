use std::{fmt, future::Future, time::Duration};

use leptos::set_timeout;
use tokio::sync::oneshot;

macro_rules! jserror {
    ($message:expr, $js_error:expr) => {{
        let error = js_sys::Error::from($js_error);
        log::error!($message, &error.to_string());
        web_sys::console::log_1(&error);
    }};
}

pub(crate) use jserror;

pub(crate) async fn sleep(duration: Duration) {
    let (callback_tx, callback_rx) = oneshot::channel();

    set_timeout(
        move || {
            let _ = callback_tx.send(());
        },
        duration,
    );

    callback_rx.await.unwrap();
}

pub(crate) async fn timeout<T, F: Future<Output = T>>(
    duration: Duration,
    function: F,
) -> Result<T, Elapsed> {
    tokio::select! {
        v = function => Ok(v),
        _ = sleep(duration) => Err(Elapsed),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Elapsed;

impl std::error::Error for Elapsed {}

impl fmt::Display for Elapsed {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        "timeout has elapsed".fmt(fmt)
    }
}
