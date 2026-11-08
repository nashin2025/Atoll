#![warn(clippy::all)]
#![windows_subsystem = "windows"]

mod commands;
mod error;

use tauri::Builder;

fn main() {
    Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::wallet_unlock,
            commands::wallet_new,
            commands::wallet_import,
            commands::wallet_balance,
            commands::wallet_send,
            commands::wallet_history,
            commands::wallet_derive_address,
            commands::node_connect,
            commands::node_disconnect,
            commands::node_get_balance,
            commands::node_send_tx,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
