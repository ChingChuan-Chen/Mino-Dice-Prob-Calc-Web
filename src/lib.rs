pub mod api;
pub mod chart;
pub mod dice;
pub mod round;
pub mod trick;
pub mod ui;

use wasm_bindgen::prelude::*;

#[cfg(feature = "console_error_panic_hook")]
pub use console_error_panic_hook::set_once as set_panic_hook;

/// Called by the browser once the WASM module is loaded.
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(all(feature = "console_error_panic_hook", debug_assertions))]
    console_error_panic_hook::set_once();

    #[cfg(debug_assertions)]
    web_sys::console::log_1(&"Mino Dice Probability Calculator loaded!".into());

    if let Err(e) = ui::init_ui() {
        web_sys::console::error_1(&e);
    }
}

/// Returns the app version string.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
