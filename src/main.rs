use std::{
    collections::HashMap,
    io::Write,
    sync::{Arc, Mutex, mpsc},
};

use log::{error, info};

use crate::{app::App, data::Data, parser::parse_maps};

mod app;
mod bvh;
mod color;
mod config;
mod constants;
mod cs2;
mod data;
mod drag_range;
mod game;
mod gui;
mod key_codes;
mod math;
mod message;
mod mouse;
mod parser;
mod process;
mod schema;
mod script;
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

    let args: Vec<String> = std::env::args().collect();

    // this runs as x11 for now, because wayland decorations for winit are not good
    // and don't support disabling the maximize button
    unsafe { std::env::remove_var("WAYLAND_DISPLAY") };

    if let Ok(username) = std::env::var("USER")
        && username == "root"
    {
        error!("start without sudo, and add your user to the input group.");
        return;
    }

    let bvh = Arc::new(Mutex::new(HashMap::new()));
    let bvh_game = bvh.clone();
    let bvh_gui = bvh.clone();

    let force_reparse = args.iter().any(|arg| arg == "--force-reparse");
    std::thread::spawn(move || parse_maps(bvh, force_reparse));

    let (tx_game, rx_gui) = mpsc::channel();
    let (tx_gui, rx_game) = mpsc::channel();
    let data = Arc::new(Mutex::new(Data::default()));
    let data_game = data.clone();

    std::thread::spawn(move || {
        game::GameManager::new(tx_game, rx_game, data_game, bvh_game).run();
    });
    info!("started game thread");

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App::new(tx_gui, rx_gui, data, bvh_gui);
    event_loop.run_app(&mut app).unwrap();
    info!("exiting");
}
