use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

use egui::{FontData, FontDefinitions, Stroke, Style};
use egui_glow::glow;
use log::info;
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
};

use crate::{
    color::Colors,
    config::{
        AimbotStatus, Config, DEFAULT_CONFIG_NAME, available_configs, get_config_path,
        parse_config, write_config,
    },
    cs2::weapon::Weapon,
    data::Data,
    gui::{AimbotTab, Tab},
    message::Message,
    mouse::DeviceStatus,
    window_context::WindowContext,
};

const FRAME_RATE: u64 = 120;
const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / FRAME_RATE);

pub struct App {
    pub gui_window: Option<WindowContext>,
    pub gui_gl: Option<Arc<glow::Context>>,
    pub gui_glow: Option<egui_glow::EguiGlow>,
    pub overlay_window: Option<WindowContext>,
    pub overlay_gl: Option<Arc<glow::Context>>,
    pub overlay_glow: Option<egui_glow::EguiGlow>,
    pub next_frame_time: Instant,

    pub tx: mpsc::Sender<Message>,
    pub rx: mpsc::Receiver<Message>,
    pub data: Arc<Mutex<Data>>,

    pub status: AimbotStatus,
    pub mouse_status: DeviceStatus,

    pub config: Config,
    pub current_config: PathBuf,
    pub available_configs: Vec<PathBuf>,
    pub new_config_name: String,

    pub current_tab: Tab,
    pub aimbot_tab: AimbotTab,
    pub aimbot_weapon: Weapon,
}

impl App {
    pub fn new(
        tx: mpsc::Sender<Message>,
        rx: mpsc::Receiver<Message>,
        data: Arc<Mutex<Data>>,
    ) -> Self {
        // read config
        let config = parse_config(&get_config_path().join(DEFAULT_CONFIG_NAME));
        // override config if invalid
        write_config(&config, &get_config_path().join(DEFAULT_CONFIG_NAME));

        Self {
            gui_window: None,
            gui_gl: None,
            gui_glow: None,

            overlay_window: None,
            overlay_gl: None,
            overlay_glow: None,
            next_frame_time: Instant::now() + FRAME_DURATION,

            tx,
            rx,
            data,
            config,
            current_config: get_config_path().join(DEFAULT_CONFIG_NAME),
            available_configs: available_configs(),
            new_config_name: String::new(),

            status: AimbotStatus::GameNotStarted,
            mouse_status: DeviceStatus::Disconnected,
            current_tab: Tab::Aimbot,
            aimbot_tab: AimbotTab::Global,
            aimbot_weapon: Weapon::Ak47,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let (gui_window, gui_gl) = create_display(event_loop, false);
        let gui_gl = Arc::new(gui_gl);
        let mut gui_glow = egui_glow::EguiGlow::new(event_loop, gui_gl.clone(), None, None, true);
        prep_ctx(&mut gui_glow.egui_ctx);
        let display_scale = gui_window.window().scale_factor() as f32;
        info!("detected display scale: {display_scale}");
        gui_glow.egui_ctx.set_pixels_per_point(1.2);

        let (overlay_window, overlay_gl) = create_display(event_loop, true);
        let overlay_gl = Arc::new(overlay_gl);
        let mut overlay_glow =
            egui_glow::EguiGlow::new(event_loop, overlay_gl.clone(), None, None, true);
        prep_ctx(&mut overlay_glow.egui_ctx);
        overlay_glow.egui_ctx.set_pixels_per_point(1.0);

        self.gui_window = Some(gui_window);
        self.gui_gl = Some(gui_gl);
        self.gui_glow = Some(gui_glow);

        self.overlay_window = Some(overlay_window);
        self.overlay_gl = Some(overlay_gl);
        self.overlay_glow = Some(overlay_glow);

        self.next_frame_time = Instant::now() + FRAME_DURATION;
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
            self.next_frame_time,
        ));
    }

    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            if let Some(window) = &self.gui_window {
                window.window().request_redraw();
            }
            if let Some(window) = &self.overlay_window {
                window.window().request_redraw();
            }
            self.next_frame_time += FRAME_DURATION;
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(gui_window) = &self.gui_window else {
            return;
        };
        let Some(overlay_window) = &self.overlay_window else {
            return;
        };

        let window = if gui_window.window().id() == window_id {
            gui_window
        } else if overlay_window.window().id() == window_id {
            overlay_window
        } else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                window.resize(new_size);
            }
            WindowEvent::RedrawRequested => {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                    self.next_frame_time,
                ));
                self.render();
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.logical_key
                    == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) =>
            {
                event_loop.exit();
            }
            _ => {
                let event_response = self
                    .gui_glow
                    .as_mut()
                    .unwrap()
                    .on_window_event(self.gui_window.as_mut().unwrap().window(), &event);

                if event_response.repaint {
                    self.gui_window.as_mut().unwrap().window().request_redraw();
                    self.overlay_window
                        .as_mut()
                        .unwrap()
                        .window()
                        .request_redraw();
                }
            }
        }
    }
}

fn create_display(
    event_loop: &winit::event_loop::ActiveEventLoop,
    overlay: bool,
) -> (WindowContext, glow::Context) {
    let glutin_window_context = WindowContext::new(event_loop, overlay);
    let gl = unsafe {
        glow::Context::from_loader_function(|s| {
            let s = std::ffi::CString::new(s)
                .expect("failed to construct C string from string for gl proc address");

            glutin_window_context.get_proc_address(&s)
        })
    };

    (glutin_window_context, gl)
}

fn prep_ctx(ctx: &mut egui::Context) {
    // add font
    let fira_sans = include_bytes!("../resources/FiraSansIcons.ttf");
    let mut font_definitions = FontDefinitions::default();
    font_definitions.font_data.insert(
        String::from("fira_sans"),
        Arc::new(FontData::from_static(fira_sans)),
    );

    // insert into font definitions, so it gets chosen as default
    font_definitions
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, String::from("fira_sans"));

    ctx.set_fonts(font_definitions);

    ctx.style_mut_of(egui::Theme::Dark, gui_style);
}

fn gui_style(style: &mut Style) {
    style.interaction.selectable_labels = false;
    //style.visuals.override_text_color = Some(Color32::WHITE);

    style.visuals.window_fill = Colors::BASE;
    style.visuals.panel_fill = Colors::BASE;
    style.visuals.extreme_bg_color = Colors::BACKDROP;

    let bg_stroke = Stroke::new(1.0, Colors::SUBTEXT);
    let fg_stroke = Stroke::new(1.0, Colors::TEXT);
    let dark_stroke = Stroke::new(1.0, Colors::BASE);

    style.visuals.selection.bg_fill = Colors::BLUE;
    style.visuals.selection.stroke = dark_stroke;

    style.visuals.widgets.active.bg_fill = Colors::HIGHLIGHT;
    style.visuals.widgets.active.bg_stroke = bg_stroke;
    style.visuals.widgets.active.fg_stroke = fg_stroke;
    style.visuals.widgets.active.weak_bg_fill = Colors::HIGHLIGHT;

    style.visuals.widgets.hovered.bg_fill = Colors::HIGHLIGHT;
    style.visuals.widgets.hovered.bg_stroke = bg_stroke;
    style.visuals.widgets.hovered.fg_stroke = fg_stroke;
    style.visuals.widgets.hovered.weak_bg_fill = Colors::HIGHLIGHT;

    style.visuals.widgets.inactive.bg_fill = Colors::HIGHLIGHT;
    style.visuals.widgets.inactive.fg_stroke = fg_stroke;
    style.visuals.widgets.inactive.weak_bg_fill = Colors::HIGHLIGHT;

    style.visuals.widgets.noninteractive.bg_fill = Colors::HIGHLIGHT;
    style.visuals.widgets.noninteractive.fg_stroke = fg_stroke;
    style.visuals.widgets.noninteractive.weak_bg_fill = Colors::HIGHLIGHT;

    style.visuals.widgets.open.bg_fill = Colors::HIGHLIGHT;
    style.visuals.widgets.open.bg_stroke = bg_stroke;
    style.visuals.widgets.open.fg_stroke = fg_stroke;
    style.visuals.widgets.open.weak_bg_fill = Colors::HIGHLIGHT;
}
