// #![allow(unused)]
#![feature(duration_millis_float)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
extern crate sp_log2;
#[macro_use]
extern crate tacky_borders_logger;

use tacky_borders::initialize_logger;
use tacky_borders::start_application;
use tacky_borders::windows_api::WindowsApi;

fn main() -> anyhow::Result<()> {
    if let Err(e) = &initialize_logger() {
        error!("logger initialization failed: {e}");
    };

    debug!("Application: Starting");
    let res = start_application();

    if let Err(err) = &res {
        error!("{err:?}");
        WindowsApi::show_error_dialog("Fatal error", &err.to_string());
    } else {
        debug!("Application: Exit");
    }

    res
}
