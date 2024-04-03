use js_sys::{Array, Object, Reflect};
use log::debug;
use tokio::{
    select,
    sync::{broadcast, mpsc},
};
use wasm_bindgen::JsValue;

use crate::peerjs::dataconnection::DataConnection;

use super::{ffi, peerid::PeerID, ICEServer};

pub struct Client {
    _id: PeerID,
    internal_peer: ffi::Peer,
    open_rx: mpsc::Receiver<()>,
    connection_rx: mpsc::Receiver<ffi::DataConnection>,
    error_tx: broadcast::Sender<ClientError>,
    error_handle: PeerErrorHandle,
}

pub struct PeerErrorHandle {
    error_rx: broadcast::Receiver<ClientError>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ClientError {
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

impl Client {
    pub fn new(client_id: PeerID, servers: Vec<ICEServer>) -> Client {
        debug!("Connecting to PeerJS as '{}'", client_id.full());

        let peer = ffi::Peer::new(client_id.full(), &create_options(servers));

        let open_rx = peer.register_callback("open");
        let connection_rx = peer.register_arg_callback("connection");
        let (error_tx, error_rx) = peer.register_error_callback();

        Client {
            _id: client_id,
            internal_peer: peer,
            open_rx,
            connection_rx,
            error_tx,
            error_handle: PeerErrorHandle { error_rx },
        }
    }

    pub async fn wait_for_open(&mut self) -> Result<(), ClientError> {
        select! {
            v = self.open_rx.recv() => match v {
                Some(_) => Ok(()),
                None => Err(ClientError::OpenCallbackClosed),
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
        debug!("Connecting to peer '{}'", peer_id.full());

        let options = Object::new();
        Reflect::set(&options, &"reliable".into(), &JsValue::from_bool(true)).unwrap();

        let internal_connection = self.internal_peer.connect(peer_id.full(), &options);

        DataConnection::new(internal_connection, self.create_error_handle())
    }

    pub async fn receive_connection(&mut self) -> Result<DataConnection, ClientError> {
        select! {
            v = self.connection_rx.recv() => match v {
                Some(v) => Ok(DataConnection::new(v, self.create_error_handle())),
                None => Err(ClientError::ConnectionCallbackClosed),
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
    pub async fn recv(&mut self) -> ClientError {
        match self.error_rx.recv().await {
            Ok(v) => v,
            Err(_) => ClientError::ErrorCallbackClosed,
        }
    }
}

impl From<ffi::Error> for ClientError {
    fn from(value: ffi::Error) -> Self {
        match value.type_().as_str() {
            "browser-incompatible" => ClientError::BrowserIncompatible,
            "disconnected" => ClientError::Disconnected,
            "invalid-id" => ClientError::InvalidID,
            "invalid-key" => ClientError::InvalidKey,
            "network" => ClientError::Network,
            "peer-unavailable" => ClientError::PeerUnavailable,
            "ssl-unavailable" => ClientError::SSLUnavailable,
            "server-error" => ClientError::ServerError,
            "socket-error" => ClientError::SocketError,
            "socket-closed" => ClientError::SocketClosed,
            "unavailable-id" => ClientError::UnavailableID,
            "webrtc" => ClientError::WebRTC(value.into()),
            _ => ClientError::Unknown(value.into()),
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.internal_peer.destroy();
        debug!("Peer closed");
    }
}
