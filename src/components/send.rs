use std::rc::Rc;

use leptos::*;
use leptos_meta::Title;
use leptos_router::NavigateOptions;
use log::{error, info};
use tokio::select;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use web_sys::File;

use crate::{
    components::{
        app::{FileToSend, CONNECT_TIMEOUT},
        settings::Settings,
    },
    peerjs::{
        client::{Client, ClientError},
        dataconnection::{DataConnection, DataConnectionError},
        peerid::PeerID,
    },
    utils::timeout,
};

#[derive(Clone)]
struct PeerStatus {
    message: RwSignal<String>,
}

#[derive(Clone)]
struct Connection {
    id: Uuid,
    peer_id: String,
    status: RwSignal<String>,
}

#[derive(Debug, thiserror::Error)]
enum ReceiveConnectionsError {
    #[error("Error while connecting to PeerJS: {0}")]
    OpenError(ClientError),
    #[error("PeerJS open timed out")]
    OpenTimedOut,
    #[error("Error while receiving connection: {0}")]
    ReceiveConnectionError(ClientError),
}

#[derive(Debug, thiserror::Error)]
enum SendFileError {
    #[error("Error while opening data connection: {0}")]
    OpenDataConnectionError(DataConnectionError),
    #[error("Data connection open timed out")]
    OpenDataConnectionTimedOut,
    #[error("Error while waiting for close: {0}")]
    CloseError(DataConnectionError),
}

#[component]
pub(crate) fn SendFile() -> impl IntoView {
    let navigate = leptos_router::use_navigate();

    let file_to_send = use_context::<ReadSignal<FileToSend>>().unwrap();
    let set_file_to_send = use_context::<WriteSignal<FileToSend>>().unwrap();

    let Some(file) = file_to_send.get_untracked().0 else {
        info!("FileToSend not set. Redirecting to menu");
        navigate("/", NavigateOptions::default());
        return view! { <div></div> };
    };
    set_file_to_send.set_untracked(FileToSend(None));

    let status = PeerStatus {
        message: create_rw_signal("Initializing".to_string()),
    };
    provide_context(status.clone());

    let connections = Vec::<Connection>::new();
    let (connections, set_connections) = create_signal(connections);
    provide_context(connections);
    provide_context(set_connections);

    let client_id = PeerID::new_random_short_id();

    let title_text = format!("Sending {}", file.name());
    let client_id_string = client_id.base().to_string();
    let base_uri = document().base_uri().unwrap().unwrap();
    let sharing_link = format!("{base_uri}#{}", client_id.base());

    let cancel_token = CancellationToken::new();
    spawn_local_with_current_owner(receive_connections(client_id, file, cancel_token.clone()))
        .unwrap();
    on_cleanup(move || cancel_token.cancel());

    view! {
        <div>
            <Title text=title_text/>
            <div>
                <div>"Client id:"</div>
                <div>{client_id_string}</div>
            </div>
            <div>
                <div>"Share this link"</div>
                <a>{sharing_link}</a>
            </div>
            <div>
                <div>"Status"</div>
                <div>{move || status.message.get()}</div>
            </div>
            <div>"Connections"</div>
            <For
                each=move || connections.get()
                key=|connection| connection.id
                children=connection_view
            />
        </div>
    }
}

fn connection_view(connection: Connection) -> impl IntoView {
    view! {
        <div>
            <div>{&connection.peer_id}</div>
            <div>{move || connection.status.get()}</div>
        </div>
    }
}

async fn receive_connections(client_id: PeerID, file: File, cancel_token: CancellationToken) {
    let result = select! {
        v = receive_connections_inner(client_id, file, cancel_token.clone()) => v,
        _ = cancel_token.cancelled() => {
            return;
        },
    };

    if let Err(error) = result {
        update_peer_status(error.to_string());
        cancel_token.cancel();
    }
}

async fn receive_connections_inner(
    client_id: PeerID,
    file: File,
    cancel_token: CancellationToken,
) -> Result<(), ReceiveConnectionsError> {
    let servers = use_context::<ReadSignal<Rc<Settings>>>()
        .unwrap()
        .get_untracked()
        .servers
        .get_untracked()
        .iter()
        .map(|server| server.to_js())
        .collect();
    let mut client = Client::new(client_id, servers);

    timeout(CONNECT_TIMEOUT, client.wait_for_open())
        .await
        .map_err(|_| ReceiveConnectionsError::OpenTimedOut)?
        .map_err(ReceiveConnectionsError::OpenError)?;

    update_peer_status("Waiting for connections");

    loop {
        let connection = client
            .receive_connection()
            .await
            .map_err(ReceiveConnectionsError::ReceiveConnectionError)?;

        spawn_local_with_current_owner(send_file(connection, file.clone(), cancel_token.clone()))
            .unwrap();
    }
}

async fn send_file(
    data_connection: DataConnection,
    file: File,
    peer_cancel_token: CancellationToken,
) {
    let status = create_rw_signal("Accepting connection".to_string());
    let connection = Connection {
        id: Uuid::new_v4(),
        peer_id: data_connection.peer_id(),
        status,
    };

    let set_connections = use_context::<WriteSignal<Vec<Connection>>>().unwrap();
    set_connections.update(|connections| {
        connections.insert(0, connection);
    });

    let result = select! {
        v = send_file_inner(data_connection, file, status) => v,
        _ = peer_cancel_token.cancelled() => {
            return;
        },
    };

    if let Err(error) = result {
        update_connection_status(status, error.to_string());
    }
}

async fn send_file_inner(
    mut data_connection: DataConnection,
    file: File,
    status: RwSignal<String>,
) -> Result<(), SendFileError> {
    timeout(CONNECT_TIMEOUT, data_connection.wait_for_open())
        .await
        .map_err(|_| SendFileError::OpenDataConnectionTimedOut)?
        .map_err(SendFileError::OpenDataConnectionError)?;

    update_connection_status(
        status,
        format!(
            "Connection from {}. Sending file",
            data_connection.peer_id()
        ),
    );
    info!("Connection from {}", data_connection.peer_id());

    data_connection.send_string(&file.name());
    data_connection.send(&file.slice().unwrap());

    update_connection_status(status, "File sent. Waiting for confirmation");

    data_connection
        .wait_for_close()
        .await
        .map_err(SendFileError::CloseError)?;

    update_connection_status(status, "Done");

    Ok(())
}

fn update_peer_status<T: ToString>(message: T) {
    let message = message.to_string();

    info!("Status: {}", &message);

    let peer_status = use_context::<PeerStatus>().unwrap();
    peer_status.message.set(message);
}

fn update_connection_status<T: ToString>(status: RwSignal<String>, message: T) {
    let message = message.to_string();

    info!("Connection status: {}", &message);

    status.set(message);
}
