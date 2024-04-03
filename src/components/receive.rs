use std::rc::Rc;

use js_sys::{Array, ArrayBuffer};
use leptos::*;
use leptos_meta::Title;
use leptos_router::{use_params, NavigateOptions, Params};
use log::{error, info};
use tokio::select;
use tokio_util::sync::CancellationToken;
use wasm_bindgen::JsCast;
use web_sys::{Blob, HtmlAnchorElement, Url};

use crate::{
    components::{app::CONNECT_TIMEOUT, settings::Settings},
    peerjs::{
        client::{Client, ClientError},
        dataconnection::DataConnectionError,
        peerid::PeerID,
    },
    utils::timeout,
};

#[derive(Params, PartialEq, Clone, Debug)]
pub struct ReceiveFileParams {
    peer_id: String,
}

struct Status {
    message: String,
}

#[derive(Debug, thiserror::Error)]
enum ReceiveFileError {
    #[error("Error while connecting to PeerJS: {0}")]
    OpenPeerError(ClientError),
    #[error("Connecting to PeerJS server timed out")]
    OpenTimedOut,
    #[error("Error while opening data connection: {0}")]
    OpenDataConnectionError(DataConnectionError),
    #[error("Data connection open timed out")]
    OpenDataConnectionTimedOut,
    #[error("Error while receiving file info: {0}")]
    ReceiveFilenameDataConnectionError(DataConnectionError),
    #[error("Error while receiving file info: {0}")]
    ReceiveDataDataConnectionError(DataConnectionError),
}

#[component]
pub(crate) fn ReceiveFile() -> impl IntoView {
    let params = use_params::<ReceiveFileParams>();
    let navigate = leptos_router::use_navigate();

    let status = Status {
        message: "Initializing".to_string(),
    };
    let (status, set_status) = create_signal(Rc::new(status));
    provide_context(status);
    provide_context(set_status);

    let Ok(peer_id) = params.get_untracked().map(|v| v.peer_id) else {
        error!("No peer id in params");
        navigate("/", NavigateOptions::default());
        return view! { <div></div> };
    };
    let Some(peer_id) = PeerID::new(peer_id.clone()) else {
        error!("Invalid peer id: {peer_id}");
        navigate("/", NavigateOptions::default());
        return view! { <div></div> };
    };

    let title_text = format!("Receiving from {}", peer_id.base());

    let cancel_token = CancellationToken::new();
    spawn_local_with_current_owner(receive_file(peer_id, cancel_token.clone())).unwrap();
    on_cleanup(move || cancel_token.cancel());

    view! {
        <div>
            <Title text=title_text/>
            <div>{move || status.get().message.clone()}</div>
        </div>
    }
}

async fn receive_file(peer_id: PeerID, cancel_token: CancellationToken) {
    let result = select! {
        v = receive_file_inner(peer_id) => v,
        _ = cancel_token.cancelled() => {
            return;
        },
    };

    if let Err(error) = result {
        update_status(error.to_string());
        cancel_token.cancel();
    }
}

async fn receive_file_inner(peer_id: PeerID) -> Result<(), ReceiveFileError> {
    update_status("Connecting to peerjs");

    let servers = use_context::<ReadSignal<Rc<Settings>>>()
        .unwrap()
        .get_untracked()
        .servers
        .get_untracked()
        .iter()
        .map(|server| server.to_js())
        .collect();
    let mut client = Client::new(PeerID::new_random_long_id(), servers);

    timeout(CONNECT_TIMEOUT, client.wait_for_open())
        .await
        .map_err(|_| ReceiveFileError::OpenTimedOut)?
        .map_err(ReceiveFileError::OpenPeerError)?;

    update_status("Opening data connection to peer");

    let mut connection = client.connect(peer_id);

    timeout(CONNECT_TIMEOUT, connection.wait_for_open())
        .await
        .map_err(|_| ReceiveFileError::OpenDataConnectionTimedOut)?
        .map_err(ReceiveFileError::OpenDataConnectionError)?;

    update_status("Waiting for file info");

    let filename = connection
        .receive::<String>()
        .await
        .map_err(ReceiveFileError::ReceiveFilenameDataConnectionError)?;

    update_status(format!("Receiving {filename}"));

    let data = connection
        .receive::<ArrayBuffer>()
        .await
        .map_err(ReceiveFileError::ReceiveDataDataConnectionError)?;

    info!("Data size: {}", data.byte_length());

    save_file(&filename, data);

    update_status(format!("Saved {filename}"));

    Ok(())
}

fn update_status<T: ToString>(message: T) {
    let message = message.to_string();

    info!("Status: {}", &message);

    let set_status = use_context::<WriteSignal<Rc<Status>>>().unwrap();
    set_status(Rc::new(Status { message }));
}

fn save_file(filename: &str, data: ArrayBuffer) {
    let buffer_array = Array::new();
    buffer_array.push(&data);
    let blob = Blob::new_with_buffer_source_sequence(&buffer_array).unwrap();
    let url = Url::create_object_url_with_blob(&blob).unwrap();

    let anchor_element = document()
        .create_element("a")
        .unwrap()
        .dyn_into::<HtmlAnchorElement>()
        .unwrap();
    anchor_element.set_href(&url);
    anchor_element.set_attribute("download", filename).unwrap();

    document()
        .body()
        .unwrap()
        .append_child(&anchor_element)
        .unwrap();

    anchor_element.click();

    document()
        .body()
        .unwrap()
        .remove_child(&anchor_element)
        .unwrap();
}
