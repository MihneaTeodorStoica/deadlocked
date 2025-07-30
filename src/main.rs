use std::{
    io::Write,
    sync::{Arc, Mutex, mpsc},
};

use log::{error, info};

use crate::{app::App, data::Data};

mod aimbot;
mod app;
mod color;
mod config;
mod constants;
mod cs2;
mod data;
mod drag_range;
mod gui;
mod key_codes;
mod math;
mod message;
mod mouse;
mod process;
mod schema;
mod window_context;

#[cfg(not(target_os = "linux"))]
compile_error!("only linux is supported.");

fn main() {
    let env = env_logger::Env::new();
    env_logger::builder()
        .format(|buf, record| writeln!(buf, "[{}] {}", record.level(), record.args()))
        .filter_level(log::LevelFilter::Off)
        .filter_module("deadlocked", log::LevelFilter::Info)
        .parse_env(env)
        .init();

    // this runs as x11 for now, because wayland decorations for winit are not good
    // and don't support disabling the maximize button
    unsafe { std::env::remove_var("WAYLAND_DISPLAY") };

    if let Ok(username) = std::env::var("USER") {
        if username == "root" {
            error!("start without sudo, and add your user to the input group.");
            return;
        }
    }

    let (tx_aimbot, rx_gui) = mpsc::channel();
    let (tx_gui, rx_aimbot) = mpsc::channel();
    let data = Arc::new(Mutex::new(Data::default()));
    let data_aimbot = data.clone();

    std::thread::spawn(move || {
        aimbot::AimbotManager::new(tx_aimbot, rx_aimbot, data_aimbot).run();
    });
    info!("started game thread");

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App::new(tx_gui, rx_gui, data);
    event_loop.run_app(&mut app).unwrap();
    info!("exiting");
}
