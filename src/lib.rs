pub mod aliases;
#[cfg(feature = "ssr")]
pub mod api;
pub mod app;
pub mod auth;
pub mod database;
pub mod domains;
pub mod error_template;
#[cfg(feature = "ssr")]
pub mod fileserv;
pub mod mailboxes;
#[cfg(feature = "ssr")]
pub mod provision;
#[cfg(feature = "ssr")]
pub mod state;
pub mod users;
pub mod utils;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount_to_body(App);
}
