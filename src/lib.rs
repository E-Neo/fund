//! Fund backtesting web application.
//!
//! The library contains the SSR+hydration Leptos app and, under the `ssr`
//! feature, the server-side modules (database, data fetching, simulation).

#[cfg(feature = "ssr")]
pub mod db;
#[cfg(feature = "ssr")]
pub mod eastmoney;
#[cfg(feature = "ssr")]
pub mod error;
#[cfg(feature = "ssr")]
pub mod fees;
#[cfg(feature = "ssr")]
pub mod report;
#[cfg(feature = "ssr")]
pub mod rules;
#[cfg(feature = "ssr")]
pub mod sim;

pub mod web;

/// Entry point for the client-side hydration.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use web::app::App;

    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
