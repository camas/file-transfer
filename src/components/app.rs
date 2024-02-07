use leptos::*;
use leptos_router::*;
use log::info;

use crate::components::{footer::Footer, header::Header};

#[component]
pub(crate) fn App() -> impl IntoView {
    info!("App started");

    let state = AppState::Menu;
    let (state, set_state) = create_signal(state);

    provide_context(RouterIntegrationContext::new(BrowserHashIntegration {}));

    view! {
        <div class="app">
            <Router>
                <Header/>
                <Footer/>
            </Router>
        </div>
    }
}

enum AppState {
    Menu,
}
