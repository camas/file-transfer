use leptos::*;
use leptos_router::*;

#[component]
pub(crate) fn Header() -> impl IntoView {
    let navigate = leptos_router::use_navigate();

    let on_header_click = move |_| {
        navigate("/", NavigateOptions::default());
    };

    view! {
        <div class="header">
            <div class="header-text" on:click=on_header_click>"File Transfer"</div>
            <div class="header-separator"/>
        </div>
    }
}
