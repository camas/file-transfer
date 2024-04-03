use std::fmt::Debug;

use js_sys::{Array, Object, Reflect};
use log::debug;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tokio::{
    select,
    sync::{broadcast, mpsc},
};
use wasm_bindgen::prelude::*;

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const CHANNEL_BUFFER_SIZE: usize = 100;

mod ffi {

    use leptos::spawn_local;
    use tokio::sync::{broadcast, mpsc};
    use wasm_bindgen::prelude::*;

    use super::{PeerError, CHANNEL_BUFFER_SIZE};

    #[wasm_bindgen]
    extern "C" {
        pub type Peer;

        #[wasm_bindgen(constructor)]
        pub fn new(id: &str, options: &JsValue) -> Peer;

        #[wasm_bindgen(method)]
        pub fn connect(this: &Peer, id: &str, options: &JsValue) -> DataConnection;

        #[wasm_bindgen(method)]
        pub fn on(this: &Peer, method: &str, callback: &Closure<dyn Fn(JsValue)>);

        #[wasm_bindgen(method, js_name = "on")]
        pub fn on_open(this: &Peer, method: &str, callback: &Closure<dyn Fn()>);

        #[wasm_bindgen(method, js_name = "on")]
        pub fn on_error(this: &Peer, method: &str, callback: &Closure<dyn Fn(Error)>);

        #[wasm_bindgen(method, js_name = "on")]
        pub fn on_connection(this: &Peer, method: &str, callback: &Closure<dyn Fn(DataConnection)>);

        #[wasm_bindgen(method)]
        pub fn destroy(this: &Peer);

        pub type DataConnection;

        #[wasm_bindgen(method)]
        pub fn close(this: &DataConnection);

        #[wasm_bindgen(method)]
        pub fn send(this: &DataConnection, data: &JsValue);

        #[wasm_bindgen(method)]
        pub fn on(this: &DataConnection, method: &str, callback: &Closure<dyn Fn(JsValue)>);

        #[wasm_bindgen(method, js_name = "on")]
        pub fn on_data(this: &DataConnection, method: &str, callback: &Closure<dyn Fn(JsValue)>);

        #[wasm_bindgen(method, js_name = "on")]
        pub fn on_open(this: &DataConnection, method: &str, callback: &Closure<dyn Fn()>);

        #[wasm_bindgen(method, js_name = "on")]
        pub fn on_error(this: &DataConnection, method: &str, callback: &Closure<dyn Fn(Error)>);

        #[wasm_bindgen(method, js_name = "on")]
        pub fn on_close(this: &DataConnection, method: &str, callback: &Closure<dyn Fn()>);

        #[wasm_bindgen(method, getter)]
        pub fn peer(this: &DataConnection) -> String;

        pub type Error;

        #[wasm_bindgen(method, getter = type)]
        pub fn type_(this: &Error) -> String;
    }

    impl Peer {
        pub fn register_callback(&self, channel: &str) -> mpsc::Receiver<()> {
            let (callback_tx, callback_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
            let closure = Closure::new(move |_| {
                let callback_tx = callback_tx.clone();
                spawn_local(async move {
                    let _ = callback_tx.send(()).await;
                });
            });

            self.on(channel, &closure);

            closure.forget();

            callback_rx
        }

        pub fn register_arg_callback<T: From<JsValue> + 'static>(
            &self,
            channel: &str,
        ) -> mpsc::Receiver<T> {
            let (callback_tx, callback_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
            let closure = Closure::new(move |value: JsValue| {
                let callback_tx = callback_tx.clone();
                spawn_local(async move {
                    let _ = callback_tx.send(value.into()).await;
                });
            });

            self.on(channel, &closure);

            closure.forget();

            callback_rx
        }

        pub fn register_error_callback(
            &self,
        ) -> (broadcast::Sender<PeerError>, broadcast::Receiver<PeerError>) {
            let (callback_tx, callback_rx) = broadcast::channel(CHANNEL_BUFFER_SIZE);
            let callback_tx_ = callback_tx.clone();
            let closure = Closure::new(move |value: JsValue| {
                let callback_tx = callback_tx_.clone();
                spawn_local(async move {
                    let _ = callback_tx.send(value.dyn_into::<Error>().unwrap().into());
                });
            });

            self.on("error", &closure);

            closure.forget();

            (callback_tx, callback_rx)
        }
    }

    impl DataConnection {
        pub fn register_callback(&self, channel: &str) -> mpsc::Receiver<()> {
            let (callback_tx, callback_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
            let closure = Closure::new(move |_| {
                let callback_tx = callback_tx.clone();
                spawn_local(async move {
                    let _ = callback_tx.send(()).await;
                });
            });

            self.on(channel, &closure);

            closure.forget();

            callback_rx
        }

        pub fn register_arg_callback<T: From<JsValue> + 'static>(
            &self,
            channel: &str,
        ) -> mpsc::Receiver<T> {
            let (callback_tx, callback_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
            let closure = Closure::new(move |value: JsValue| {
                let callback_tx = callback_tx.clone();
                spawn_local(async move {
                    let _ = callback_tx.send(value.into()).await;
                });
            });

            self.on(channel, &closure);

            closure.forget();

            callback_rx
        }
    }
}

pub struct Peer {
    _id: PeerID,
    internal_peer: ffi::Peer,
    open_rx: mpsc::Receiver<()>,
    connection_rx: mpsc::Receiver<ffi::DataConnection>,
    error_tx: broadcast::Sender<PeerError>,
    error_handle: PeerErrorHandle,
}

pub struct PeerErrorHandle {
    error_rx: broadcast::Receiver<PeerError>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum PeerError {
    #[error("The client's browser does not support some or all WebRTC features that you are trying to use")]
    BrowserIncompatible,
    #[error("You've already disconnected this peer from the server and can no longer make any new connections on it")]
    Disconnected,
    #[error("The ID passed into the Peer constructor contains illegal characters")]
    InvalidID,
    #[error("The API key passed into the Peer constructor contains illegal characters or is not in the system (cloud server only)")]
    InvalidKey,
    #[error("Lost or cannot establish a connection to the signalling server")]
    Network,
    #[error("The peer you're trying to connect to does not exist")]
    PeerUnavailable,
    #[error("PeerJS is being used securely, but the cloud server does not support SSL. Use a custom PeerServer")]
    SSLUnavailable,
    #[error("Unable to reach the server")]
    ServerError,
    #[error("An error from the underlying socket")]
    SocketError,
    #[error("The underlying socket closed unexpectedly")]
    SocketClosed,
    #[error("The ID passed into the Peer constructor is already taken")]
    UnavailableID,
    #[error("Native WebRTC error")]
    WebRTC(JsValue),
    #[error("Open callback closed unexpectedly")]
    OpenCallbackClosed,
    #[error("Connection callback closed unexpectedly")]
    ConnectionCallbackClosed,
    #[error("Error callback closed unexpectedly")]
    ErrorCallbackClosed,
    #[error("Not an error. Should never be thrown")]
    NoError,
    #[error("Unknown error: {0:?}")]
    Unknown(JsValue),
}

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
    PeerError(PeerError),
    #[error("Unknown error: {0:?}")]
    Unknown(JsValue),
}

#[derive(Clone)]
pub struct PeerID {
    base_id: String,
    full_id: String,
}

#[wasm_bindgen]
#[derive(Debug, Serialize, Deserialize)]
pub struct ICEServer {
    urls: String,
    username: Option<String>,
    credential: Option<String>,
}

impl Peer {
    pub fn new(client_id: PeerID, servers: Vec<ICEServer>) -> Peer {
        debug!("Connecting to PeerJS as '{}'", client_id.full_id);

        let peer = ffi::Peer::new(&client_id.full_id, &create_options(servers));

        let open_rx = peer.register_callback("open");
        let connection_rx = peer.register_arg_callback("connection");
        let (error_tx, error_rx) = peer.register_error_callback();

        Peer {
            _id: client_id,
            internal_peer: peer,
            open_rx,
            connection_rx,
            error_tx,
            error_handle: PeerErrorHandle { error_rx },
        }
    }

    pub async fn wait_for_open(&mut self) -> Result<(), PeerError> {
        select! {
            v = self.open_rx.recv() => match v {
                Some(_) => Ok(()),
                None => Err(PeerError::OpenCallbackClosed),
            },
            v = self.error_handle.recv() => Err(v),
        }
    }

    fn create_error_handle(&self) -> PeerErrorHandle {
        PeerErrorHandle {
            error_rx: self.error_tx.subscribe(),
        }
    }

    pub fn connect(&self, peer_id: PeerID) -> DataConnection {
        debug!("Connecting to peer '{}'", peer_id.full_id);

        let options = Object::new();
        Reflect::set(&options, &"reliable".into(), &JsValue::from_bool(true)).unwrap();

        let internal_connection = self.internal_peer.connect(&peer_id.full_id, &options);

        DataConnection::new(internal_connection, self.create_error_handle())
    }

    pub async fn receive_connection(&mut self) -> Result<DataConnection, PeerError> {
        select! {
            v = self.connection_rx.recv() => match v {
                Some(v) => Ok(DataConnection::new(v, self.create_error_handle())),
                None => Err(PeerError::ConnectionCallbackClosed),
            },
            v = self.error_handle.recv() => Err(v),
        }
    }
}

fn create_options(servers: Vec<ICEServer>) -> Object {
    let js_servers = Array::new();
    for server in servers {
        js_servers.push(&server.into());
    }

    let config = Object::new();
    Reflect::set(&config, &"sdpSemantics".into(), &"unified-plan".into()).unwrap();
    Reflect::set(&config, &"iceServers".into(), &js_servers).unwrap();

    let options = Object::new();
    Reflect::set(&options, &"config".into(), &config).unwrap();

    options
}

impl PeerErrorHandle {
    async fn recv(&mut self) -> PeerError {
        match self.error_rx.recv().await {
            Ok(v) => v,
            Err(_) => PeerError::ErrorCallbackClosed,
        }
    }
}

impl From<ffi::Error> for PeerError {
    fn from(value: ffi::Error) -> Self {
        match value.type_().as_str() {
            "browser-incompatible" => PeerError::BrowserIncompatible,
            "disconnected" => PeerError::Disconnected,
            "invalid-id" => PeerError::InvalidID,
            "invalid-key" => PeerError::InvalidKey,
            "network" => PeerError::Network,
            "peer-unavailable" => PeerError::PeerUnavailable,
            "ssl-unavailable" => PeerError::SSLUnavailable,
            "server-error" => PeerError::ServerError,
            "socket-error" => PeerError::SocketError,
            "socket-closed" => PeerError::SocketClosed,
            "unavailable-id" => PeerError::UnavailableID,
            "webrtc" => PeerError::WebRTC(value.into()),
            _ => PeerError::Unknown(value.into()),
        }
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.internal_peer.destroy();
        debug!("Peer closed");
    }
}

impl DataConnection {
    fn new(
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

    pub async fn receive<T: TryFrom<JsValue, Error = impl Debug>>(
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

    pub fn peer(&self) -> String {
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

impl PeerID {
    pub fn new(base_id: String) -> Option<PeerID> {
        if !Self::valid_base(&base_id) {
            return None;
        }

        let full_id = format!("camas-file-transfer-{base_id}");

        Some(PeerID { base_id, full_id })
    }

    pub fn new_random_short_id() -> PeerID {
        let base_id = random_alphabet_string(4);
        PeerID::new(base_id).unwrap()
    }

    pub fn new_random_long_id() -> PeerID {
        let base_id = random_alphabet_string(10);
        PeerID::new(base_id).unwrap()
    }

    pub fn new_short_id(base_id: String) -> Option<PeerID> {
        if base_id.len() != 4 || !Self::valid_base(&base_id) {
            return None;
        }

        PeerID::new(base_id)
    }

    pub fn valid_base(base_id: &str) -> bool {
        base_id.as_bytes().iter().all(|c| ALPHABET.contains(c))
    }

    pub fn base(&self) -> &str {
        &self.base_id
    }
}

fn random_alphabet_string(len: usize) -> String {
    let mut rng = thread_rng();
    (0..len)
        .map(|_| {
            let index = rng.gen_range(0..ALPHABET.len());
            ALPHABET[index] as char
        })
        .collect::<String>()
}

#[wasm_bindgen]
impl ICEServer {
    pub fn new(urls: String, username: Option<String>, credential: Option<String>) -> ICEServer {
        ICEServer {
            urls,
            username,
            credential,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn urls(&self) -> String {
        self.urls.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn username(&self) -> Option<String> {
        self.username.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn credential(&self) -> Option<String> {
        self.credential.clone()
    }
}
