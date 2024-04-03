use std::{rc::Rc, time::Duration};

use leptos::*;
use leptos_meta::{provide_meta_context, Title};
use leptos_router::*;
use log::info;
use web_sys::File;

use crate::{
    components::{
        footer::Footer, header::Header, menu::Menu, receive::ReceiveFile, send::SendFile,
        settings::Settings,
    },
    peerjs::PeerID,
};

pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[component]
pub(crate) fn App() -> impl IntoView {
    info!("App started");

    provide_meta_context();

    let settings = Settings::load_or_default();
    let (settings, set_settings) = create_signal(Rc::new(settings));
    provide_context(settings);
    provide_context(set_settings);

    let (file_to_send, set_file_to_send) = create_signal::<FileToSend>(FileToSend(None));
    provide_context(file_to_send);
    provide_context(set_file_to_send);

    let location = web_sys::window().unwrap().location();
    let location_hash = location.hash().unwrap();
    if hash_is_peer_id(&location_hash) {
        let receive_endpoint = format!("#/receive/{}", location_hash.trim_start_matches('#'));
        location.set_hash(&receive_endpoint).unwrap();
    }

    provide_context(RouterIntegrationContext::new(BrowserHashIntegration {}));

    view! {
        <div class="app">
            <Title formatter=|text| format!("{text} - File Transfer")/>
            <Router fallback=|| view! { <Redirect path="/"/> }>
                <Header/>
                <div class="page">
                    <Routes>
                        <Route path="/" view=Menu/>
                        <Route path="/send" view=SendFile/>
                        <Route path="/receive/:peer_id" view=ReceiveFile/>
                    </Routes>
                </div>
                <Footer/>
            </Router>
        </div>
    }
}

#[derive(Clone)]
pub struct FileToSend(pub Option<File>);

fn hash_is_peer_id(hash: &str) -> bool {
    let trimmed = hash.strip_prefix('#').unwrap_or(hash);
    trimmed.len() == 4 && PeerID::valid_base(trimmed)
}
