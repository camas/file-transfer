use leptos::spawn_local;
use tokio::sync::{broadcast, mpsc};
use wasm_bindgen::prelude::*;

use super::{client::ClientError, CHANNEL_BUFFER_SIZE};

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
    ) -> (
        broadcast::Sender<ClientError>,
        broadcast::Receiver<ClientError>,
    ) {
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
