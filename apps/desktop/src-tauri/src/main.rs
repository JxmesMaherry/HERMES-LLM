#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    if !hermes_lib::run_vault_cli() {
        hermes_lib::run();
    }
}
