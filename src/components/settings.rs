use std::{rc::Rc, time::Duration};

use gloo_utils::format::JsValueSerdeExt;
use js_sys::JSON;
use leptos::{
    component, create_memo, create_rw_signal, event_target_value, set_interval, use_context, view,
    window, For, IntoView, ReadSignal, RwSignal, SignalGet, SignalGetUntracked, SignalSet,
    SignalUpdate, WriteSignal,
};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wasm_bindgen::JsValue;

use crate::{peerjs::ICEServer, utils::jserror};

const SETTINGS_KEY: &str = "settings";

#[component]
pub(crate) fn SettingsEditor() -> impl IntoView {
    let settings = use_context::<ReadSignal<Rc<Settings>>>().unwrap();
    let set_settings = use_context::<WriteSignal<Rc<Settings>>>().unwrap();

    set_interval(
        move || settings.get_untracked().save(),
        Duration::from_secs(5),
    );

    leptos::on_cleanup(move || settings.get_untracked().save());

    let servers = create_memo(move |_| {
        let settings = settings.get();
        settings.servers.get().to_vec()
    });

    let on_add_click = move |_| {
        let settings = settings.get_untracked();
        settings.servers.update(|servers| {
            servers.push(Rc::new(SettingsServer {
                id: Uuid::new_v4(),
                url: create_rw_signal(String::new()),
                username: create_rw_signal(String::new()),
                credential: create_rw_signal(String::new()),
                editing: create_rw_signal(true),
            }))
        })
    };

    view! {
        <div class="settings">
            <div>"Settings"</div>
            <div on:click=move |_| {
                set_settings(Rc::new(Settings::default()));
                info!("Settings reset");
            }>"Reset"</div>
            <div>"Servers"</div>
            <div on:click=on_add_click>"Add"</div>
            <For
                each=move || servers.get()
                key=|server| server.id
                children=server_view
            />
        </div>
    }
}

fn server_view(server: Rc<SettingsServer>) -> impl IntoView {
    let SettingsServer {
        id,
        url,
        username,
        credential: credentials,
        editing,
    } = *server;

    let on_click = move |_| {
        editing.update(|v| {
            *v = !*v;
        })
    };

    let editing_view = move || {
        if !editing.get() {
            return None;
        }

        Some(view! {
            <div>"Url"</div>
            <input prop:value=url on:input=move |event| url.set(event_target_value(&event))/>
            <div>"Username"</div>
            <input prop:value=username on:input=move |event| username.set(event_target_value(&event)) />
            <div>"Credentials"</div>
            <input prop:value=credentials on:input=move |event| credentials.set(event_target_value(&event))/>
        })
    };

    let url_display_string = create_memo(move |_| {
        let url = url();
        let url = if url.is_empty() {
            "<missing url>".to_string()
        } else {
            url
        };
        let username = username();
        let credentials = credentials();
        match (username.is_empty(), credentials.is_empty()) {
            (true, true) => url,
            (false, true) => format!("{username}@{url}"),
            (false, false) => format!("{username}:****@{url}"),
            (true, false) => format!(":****@{url}"),
        }
    });

    let on_remove_click = move |_| {
        let settings = use_context::<ReadSignal<Rc<Settings>>>().unwrap();
        let settings = settings.get_untracked();
        settings.servers.update(|servers| {
            servers.retain(|server| server.id != id);
        });
    };

    view! {
        <div class="settings-server">
            <div on:click=on_click>{url_display_string}</div>
            {editing_view}
            <div on:click=on_remove_click>"Remove"</div>
        </div>
    }
}

pub struct Settings {
    pub servers: RwSignal<Vec<Rc<SettingsServer>>>,
}

#[derive(PartialEq)]
pub struct SettingsServer {
    id: Uuid,
    url: RwSignal<String>,
    username: RwSignal<String>,
    credential: RwSignal<String>,
    editing: RwSignal<bool>,
}

#[derive(Serialize, Deserialize)]
struct SavedSettings {
    servers: Vec<SavedSettingsServer>,
}

#[derive(Serialize, Deserialize)]
struct SavedSettingsServer {
    url: String,
    username: Option<String>,
    credential: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            servers: create_rw_signal(vec![Rc::new(SettingsServer {
                id: Uuid::new_v4(),
                url: RwSignal::new("stun:stun.l.google.com:19302".to_string()),
                username: RwSignal::new(String::new()),
                credential: RwSignal::new(String::new()),
                editing: RwSignal::new(false),
            })]),
        }
    }
}

impl Settings {
    pub fn load_or_default() -> Settings {
        let local_storage = match window().local_storage() {
            Ok(Some(v)) => v,
            Ok(None) => {
                warn!("localStorage not found. Browser up-to-date?");
                return Settings::default();
            }
            Err(error) => {
                jserror!("Error getting localStorage: {}", error);
                return Settings::default();
            }
        };

        let Some(settings_string) = local_storage.get_item(SETTINGS_KEY).unwrap() else {
            info!("No settings found in localStorage");
            return Settings::default();
        };

        let settings_js = match JSON::parse(&settings_string) {
            Ok(value) => value,
            Err(error) => {
                jserror!("Error decoding localStorage settings: {}", error);
                return Settings::default();
            }
        };

        settings_js.into_serde::<SavedSettings>().unwrap().into()
    }

    pub fn save(&self) {
        let settings_js = JsValue::from_serde(&self.to_saved()).unwrap();
        let settings_string = JSON::stringify(&settings_js).unwrap().as_string().unwrap();

        let local_storage = window().local_storage().unwrap().unwrap();
        local_storage
            .set_item(SETTINGS_KEY, &settings_string)
            .unwrap();
    }

    fn to_saved(&self) -> SavedSettings {
        SavedSettings {
            servers: self
                .servers
                .get_untracked()
                .iter()
                .map(|server| SavedSettingsServer {
                    url: server.url.get_untracked(),
                    username: string_to_option(server.username.get_untracked()),
                    credential: string_to_option(server.credential.get_untracked()),
                })
                .collect(),
        }
    }
}

impl From<SavedSettings> for Settings {
    fn from(value: SavedSettings) -> Self {
        Settings {
            servers: create_rw_signal(
                value
                    .servers
                    .into_iter()
                    .map(|saved_server| {
                        Rc::new(SettingsServer {
                            id: Uuid::new_v4(),
                            url: create_rw_signal(saved_server.url),
                            username: create_rw_signal(option_to_string(saved_server.username)),
                            credential: create_rw_signal(option_to_string(saved_server.credential)),
                            editing: create_rw_signal(false),
                        })
                    })
                    .collect(),
            ),
        }
    }
}

impl SettingsServer {
    pub fn to_js(&self) -> ICEServer {
        ICEServer::new(
            self.url.get_untracked(),
            string_to_option(self.username.get_untracked()),
            string_to_option(self.credential.get_untracked()),
        )
    }
}

fn string_to_option(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn option_to_string(value: Option<String>) -> String {
    match value {
        Some(v) => v,
        None => String::new(),
    }
}
