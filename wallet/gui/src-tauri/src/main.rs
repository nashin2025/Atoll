#![warn(clippy::all)]
#![windows_subsystem = "windows"]

mod commands;
mod error;
mod node;

use std::sync::RwLock;

use tauri::Builder;

use crate::node::NodeManager;

fn main() {
    Builder::default()
        // Embedded node manager: holds atoll-core state in-process.
        // All Tauri commands read/write through this via State<'_, RwLock<NodeManager>>.
        .manage(RwLock::new(NodeManager::new()))
        .invoke_handler(tauri::generate_handler![
            commands::wallet_unlock,
            commands::wallet_new,
            commands::wallet_import,
            commands::wallet_balance,
            commands::wallet_send,
            commands::wallet_history,
            commands::wallet_derive_address,
            commands::node_start,
            commands::node_stop,
            commands::node_status,
            commands::node_get_balance,
            commands::node_get_history,
            commands::node_send_tx,
            commands::node_sync,
            commands::node_add_peer,
            commands::node_mine,
            commands::node_auto_mine,
            commands::node_auto_mine_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
