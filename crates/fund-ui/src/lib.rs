//! Fund backtesting web UI (client-rendered).
//!
//! Compiled to `wasm32-unknown-unknown` by the server's `build.rs` and
//! embedded into the server binary; data is loaded from the REST API exposed
//! by `fund-api` via `/api` endpoints.

#![recursion_limit = "256"]

pub mod api;
pub mod app;
pub mod chart;
pub mod pages;

/// Entry point invoked from `index.html` after the wasm module initializes.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn main() {
    use app::App;

    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
