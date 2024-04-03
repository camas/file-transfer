use log::debug;
use tokio::{select, sync::mpsc};
use wasm_bindgen::JsValue;

use super::{
    client::{ClientError, PeerErrorHandle},
    ffi,
};

pub struct DataConnection {
    internal_connection: ffi::DataConnection,
    data_rx: mpsc::Receiver<JsValue>,
    open_rx: mpsc::Receiver<()>,
    close_rx: mpsc::Receiver<()>,
    error_rx: mpsc::Receiver<ffi::Error>,
    peer_error_handle: PeerErrorHandle,
}

#[derive(Debug, thiserror::Error)]
pub enum DataConnectionError {
    #[error("Couldn't cast data to expected type: {0}")]
    InvalidCast(String),
    #[error("Data callback closed unexpectedly")]
    DataCallbackClosed,
    #[error("Open callback closed unexpectedly")]
    OpenCallbackClosed,
    #[error("Close callback closed unexpectedly")]
    CloseCallbackClosed,
    #[error("Error callback closed unexpectedly")]
    ErrorCallbackClosed,
    #[error("Peer error: {0}")]
    PeerError(ClientError),
    #[error("Unknown error: {0:?}")]
    Unknown(JsValue),
}

impl DataConnection {
    pub fn new(
        internal_connection: ffi::DataConnection,
        peer_error_handle: PeerErrorHandle,
    ) -> DataConnection {
        let data_rx = internal_connection.register_arg_callback("data");
        let open_rx = internal_connection.register_callback("open");
        let close_rx = internal_connection.register_callback("close");
        let error_rx = internal_connection.register_arg_callback("error");

        DataConnection {
            internal_connection,
            data_rx,
            open_rx,
            close_rx,
            error_rx,
            peer_error_handle,
        }
    }

    pub async fn wait_for_open(&mut self) -> Result<(), DataConnectionError> {
        select! {
            v = self.open_rx.recv() => match v {
                Some(_) => Ok(()),
                None => Err(DataConnectionError::OpenCallbackClosed),
            },
            v = recv_data_error(&mut self.error_rx) => Err(v),
            v = self.peer_error_handle.recv() => Err(DataConnectionError::PeerError(v)),
        }
    }

    pub fn send_string(&self, value: &str) {
        let js_value = JsValue::from_str(value);
        self.internal_connection.send(&js_value);
    }

    pub fn send(&self, value: &JsValue) {
        self.internal_connection.send(value);
    }

    pub async fn receive<T: TryFrom<JsValue, Error = impl std::fmt::Debug>>(
        &mut self,
    ) -> Result<T, DataConnectionError> {
        select! {
            v = self.data_rx.recv() => match v {
                Some(v) => match v.try_into() {
                    Ok(s) => Ok(s),
                    Err(error) => Err(DataConnectionError::InvalidCast(format!("{error:?}"))),
                },
                None => Err(DataConnectionError::DataCallbackClosed),
            },
            v = recv_data_error(&mut self.error_rx) => Err(v),
            v = self.peer_error_handle.recv() => Err(DataConnectionError::PeerError(v)),
        }
    }

    pub async fn wait_for_close(&mut self) -> Result<(), DataConnectionError> {
        select! {
            v = self.close_rx.recv() => match v {
                Some(_) => Ok(()),
                None => Err(DataConnectionError::CloseCallbackClosed),
            },
            v = recv_data_error(&mut self.error_rx) => Err(v),
            v = self.peer_error_handle.recv() => Err(DataConnectionError::PeerError(v)),
        }
    }

    pub fn peer_id(&self) -> String {
        self.internal_connection.peer()
    }
}

async fn recv_data_error(error_rx: &mut mpsc::Receiver<ffi::Error>) -> DataConnectionError {
    match error_rx.recv().await {
        Some(v) => v.into(),
        None => DataConnectionError::ErrorCallbackClosed,
    }
}

impl Drop for DataConnection {
    fn drop(&mut self) {
        self.internal_connection.close();
        debug!("DataConnection closed");
    }
}

impl From<ffi::Error> for DataConnectionError {
    fn from(value: ffi::Error) -> Self {
        // TODO: Can it return anything else? Docs are sparse https://peerjs.com/docs/#dataconnection-on-error
        #[allow(clippy::match_single_binding)]
        match value.type_().as_str() {
            _ => DataConnectionError::Unknown(value.into()),
        }
    }
}
