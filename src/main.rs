use sycamore::prelude::*;
use web_sys::{wasm_bindgen::prelude::*, window};

mod components;
mod routing;

#[component]
pub fn App() -> View {
    window()
        .unwrap_throw()
        .document()
        .unwrap_throw()
        .set_title("ebobo.dev");

    view! {
        routing::Root()
    }
}

fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();
    sycamore::render(App);
}
