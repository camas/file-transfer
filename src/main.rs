use components::app::App;

mod components;
mod peerjs;
mod utils;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    remove_loading_div();

    leptos::mount_to_body(App);
}

fn remove_loading_div() {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("loading-screen")
        .unwrap()
        .remove();
}
