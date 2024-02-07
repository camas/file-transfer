use components::app::App;

mod components;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

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
