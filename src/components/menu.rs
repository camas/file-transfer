use leptos::{html::Input, *};
use leptos_meta::Title;
use leptos_router::NavigateOptions;
use log::error;
use web_sys::{Event, MouseEvent};

use crate::{
    components::{app::FileToSend, settings::SettingsEditor},
    peerjs::PeerID,
};

#[component]
pub(crate) fn Menu() -> impl IntoView {
    let navigate = leptos_router::use_navigate();

    let set_file_to_send = use_context::<WriteSignal<FileToSend>>().unwrap();

    let file_input_ref = create_node_ref::<Input>();
    let receive_input_ref = create_node_ref::<Input>();

    let send_click = move |_: MouseEvent| {
        if let Some(e) = file_input_ref() {
            // Spawn new thread to prevent event being fired while handling an event
            spawn_local(async move {
                e.click();
            });
        }
    };

    let navigate_ = navigate.clone();
    let on_hidden_input_change = move |_: Event| {
        let Some(file) = file_input_ref()
            .and_then(|e| e.files())
            .filter(|fl| fl.length() >= 1)
            .and_then(|fl| fl.item(0))
        else {
            return;
        };

        set_file_to_send(FileToSend(Some(file)));
        navigate_("/send", NavigateOptions::default());
    };

    let receive_change = move |_| {
        let Some(peer_id_string) = receive_input_ref().map(|e| e.value()) else {
            error!("No input node ref");
            return;
        };

        let Some(peer_id) = PeerID::new_short_id(peer_id_string) else {
            error!("Invalid peer id");
            return;
        };

        navigate(
            &format!("/receive/{}", peer_id.base()),
            NavigateOptions::default(),
        );
    };

    view! {
        <div class="menu-container">
            <Title text="Menu"/>
            <div class="menu">
                <div>"Peer-to-peer file transfer. Select a file to send, or enter another user's code to receive. All data is sent encrypted thanks to WebRTC. Connections brokered via PeerJS's Cloud PeerServer."</div>
                <div on:click=send_click>"Send file"</div>
                <div>
                    <div>"Receive from:"</div>
                    <input type="text" on:change=receive_change node_ref=receive_input_ref></input>
                </div>
                <div class="menu-separator"/>
                <SettingsEditor/>
            </div>
            <input type="file" class="menu-hidden-file-input" node_ref=file_input_ref on:change=on_hidden_input_change/>
        </div>
    }
}
