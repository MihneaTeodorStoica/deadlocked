use egui::{Align, Align2, Color32, Context, DragValue, Painter, Sense, Stroke, Ui, pos2};
use egui_glow::glow;
use log::info;
use strum::IntoEnumIterator;

use crate::{
    app::App,
    color::Colors,
    config::{AimbotStatus, Config, VERSION, WeaponConfig, write_config},
    constants::cs2,
    cs2::{bones::Bones, weapon::Weapon},
    data::{Data, PlayerData},
    key_codes::KeyCode,
    math::world_to_screen,
    message::Message,
    mouse::DeviceStatus,
};

#[derive(PartialEq)]
pub enum Tab {
    Aimbot,
    Player,
    Hud,
    Unsafe,
    Config,
    Misc,
}

#[derive(PartialEq)]
pub enum AimbotTab {
    Global,
    Weapon,
}

impl App {
    pub fn send_config(&self) {
        self.send_message(Message::Config(self.config.clone()));
        write_config(&self.config);
    }

    pub fn send_message(&self, message: Message) {
        self.tx.send(message).expect("aimbot thread died");
    }

    fn gui(&mut self, ctx: &Context) {
        egui::SidePanel::new(egui::containers::panel::Side::Left, "sidebar")
            .resizable(false)
            .show(ctx, |ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Aimbot, "\u{f04fe} Aimbot");
                ui.selectable_value(&mut self.current_tab, Tab::Player, "\u{f0013} Player");
                ui.selectable_value(&mut self.current_tab, Tab::Hud, "\u{f0379} Hud");
                ui.selectable_value(&mut self.current_tab, Tab::Unsafe, "\u{f0ce6} Unsafe");
                ui.selectable_value(&mut self.current_tab, Tab::Config, "\u{f168b} Config");
                ui.selectable_value(&mut self.current_tab, Tab::Misc, "\u{f01d8} Misc");

                ui.with_layout(egui::Layout::bottom_up(Align::Min), |ui| {
                    ui.label(VERSION);
                    if ui.button("Report Issue").clicked() {
                        ctx.open_url(egui::OpenUrl {
                            url: String::from("https://github.com/avitran0/deadlocked/issues"),
                            new_tab: false,
                        });
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.add_game_status(ui);
            ui.separator();

            match self.current_tab {
                Tab::Aimbot => self.aimbot_settings(ui),
                Tab::Player => self.player_settings(ui),
                Tab::Hud => self.hud_settings(ui),
                Tab::Unsafe => self.unsafe_settings(ui),
                Tab::Config => self.config_settings(ui),
                Tab::Misc => self.misc_settings(ui),
            }
        });
    }

    fn held_weapon_config(&mut self) -> &mut WeaponConfig {
        let data = self.data.lock().unwrap();
        if self
            .config
            .aimbot
            .weapons
            .get(&data.weapon)
            .unwrap()
            .enabled
        {
            self.config.aimbot.weapons.get_mut(&data.weapon).unwrap()
        } else {
            &mut self.config.aimbot.global
        }
    }

    fn weapon_config(&mut self) -> &mut WeaponConfig {
        if self.aimbot_tab == AimbotTab::Weapon {
            self.config
                .aimbot
                .weapons
                .get_mut(&self.aimbot_weapon)
                .unwrap()
        } else {
            &mut self.config.aimbot.global
        }
    }

    fn section_title(&self, ui: &mut Ui, title: &str) {
        ui.add_space(8.0);
        ui.label(title);
        ui.separator();
    }

    fn aimbot_settings(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.aimbot_tab, AimbotTab::Global, "Global");
            ui.selectable_value(&mut self.aimbot_tab, AimbotTab::Weapon, "Weapon");
            if self.aimbot_tab == AimbotTab::Weapon {
                egui::ComboBox::new("aimbot_weapon", "Weapon")
                    .selected_text(format!("{:?}", self.aimbot_weapon))
                    .show_ui(ui, |ui| {
                        for weapon in Weapon::iter() {
                            let text = format!("{:?}", weapon);
                            ui.selectable_value(&mut self.aimbot_weapon, weapon, text);
                        }
                    });
            }
        });
        ui.separator();
        ui.columns(2, |cols| {
            let left = &mut cols[0];
            egui::ScrollArea::vertical()
                .id_salt("aimbot_left")
                .show(left, |left| {
                    self.aimbot_left(left);
                });

            let right = &mut cols[1];
            egui::ScrollArea::vertical()
                .id_salt("aimbot_right")
                .show(right, |right| {
                    self.aimbot_right(right);
                });
        });
    }

    fn aimbot_left(&mut self, ui: &mut Ui) {
        ui.label("Aimbot");
        ui.separator();

        egui::ComboBox::new("aimbot_hotkey", "Hotkey")
            .selected_text(format!("{:?}", self.config.aimbot.hotkey))
            .show_ui(ui, |ui| {
                for key_code in KeyCode::iter() {
                    let text = format!("{:?}", &key_code);
                    if ui
                        .selectable_value(&mut self.config.aimbot.hotkey, key_code, text)
                        .clicked()
                    {
                        self.send_config();
                    }
                }
            });

        let enable_label = if self.aimbot_tab == AimbotTab::Global {
            "Enable Aimbot"
        } else {
            "Enable Override"
        };
        if ui
            .checkbox(&mut self.weapon_config().enabled, enable_label)
            .changed()
        {
            self.send_config();
        }

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.weapon_config().fov)
                        .range(0.1..=360.0)
                        .suffix("°")
                        .speed(0.02)
                        .max_decimals(1),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("FOV");
        });

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.weapon_config().smooth)
                        .range(0.0..=10.0)
                        .speed(0.02)
                        .max_decimals(1),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("Smooth");
        });

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.weapon_config().start_bullet)
                        .range(0..=10)
                        .speed(0.05),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("Start Bullet");
        });

        if ui
            .checkbox(&mut self.weapon_config().multibone, "Multibone")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.weapon_config().aim_lock, "Aim Lock")
            .changed()
        {
            self.send_config();
        }

        self.section_title(ui, "Checks");

        if ui
            .checkbox(
                &mut self.weapon_config().visibility_check,
                "Visibility Check",
            )
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.weapon_config().flash_check, "Flash Check")
            .changed()
        {
            self.send_config();
        }

        self.section_title(ui, "RCS");

        if ui
            .checkbox(&mut self.weapon_config().rcs, "Enable RCS")
            .changed()
        {
            self.send_config();
        }
        ui.end_row();
    }

    fn aimbot_right(&mut self, ui: &mut Ui) {
        ui.label("Triggerbot");
        ui.separator();

        let enable_label = if self.aimbot_tab == AimbotTab::Global {
            "Enable Triggerbot"
        } else {
            "Enable Override"
        };
        if ui
            .checkbox(&mut self.weapon_config().triggerbot.enabled, enable_label)
            .changed()
        {
            self.send_config();
        }

        egui::ComboBox::new("triggerbot_hotkey", "Hotkey")
            .selected_text(format!("{:?}", self.config.aimbot.triggerbot_hotkey))
            .show_ui(ui, |ui| {
                for key_code in KeyCode::iter() {
                    let text = format!("{:?}", &key_code);
                    if ui
                        .selectable_value(&mut self.config.aimbot.triggerbot_hotkey, key_code, text)
                        .clicked()
                    {
                        self.send_config();
                    }
                }
            });

        ui.horizontal(|ui| {
            let mut start = *self.weapon_config().triggerbot.delay.start();
            if ui
                .add(
                    egui::DragValue::new(&mut start)
                        .speed(0.2)
                        .range(0..=*self.weapon_config().triggerbot.delay.end()),
                )
                .changed()
            {
                self.send_config();
            }
            let mut end = *self.weapon_config().triggerbot.delay.end();
        });
    }

    fn player_settings(&mut self, ui: &mut Ui) {
        ui.columns(2, |cols| {
            let left = &mut cols[0];
            egui::ScrollArea::vertical()
                .id_salt("player_left")
                .show(left, |left| {
                    self.player_left(left);
                });

            let right = &mut cols[1];
            egui::ScrollArea::vertical()
                .id_salt("player_right")
                .show(right, |right| {
                    self.player_right(right);
                });
        });
    }

    fn player_left(&mut self, ui: &mut Ui) {}

    fn player_right(&mut self, ui: &mut Ui) {}

    fn hud_settings(&mut self, ui: &mut Ui) {
        ui.columns(2, |cols| {
            let left = &mut cols[0];
            egui::ScrollArea::vertical()
                .id_salt("hud_left")
                .show(left, |left| {
                    self.hud_left(left);
                });

            let right = &mut cols[1];
            egui::ScrollArea::vertical()
                .id_salt("hud_right")
                .show(right, |right| {
                    self.hud_right(right);
                });
        });
    }

    fn hud_left(&mut self, ui: &mut Ui) {}

    fn hud_right(&mut self, ui: &mut Ui) {}

    fn unsafe_settings(&mut self, ui: &mut Ui) {
        egui::Grid::new("unsafe").num_columns(4).show(ui, |ui| {
            ui.label("No Flash");
            if ui.checkbox(&mut self.config.misc.no_flash, "").changed() {
                self.send_config();
            }

            ui.label("Max Flash Alpha");
            if ui
                .add(
                    DragValue::new(&mut self.config.misc.max_flash_alpha)
                        .range(0.0..=1.0)
                        .speed(0.002)
                        .max_decimals(2),
                )
                .changed()
            {
                self.send_config();
            }
            ui.end_row();

            ui.label("FOV Changer");
            if ui.checkbox(&mut self.config.misc.fov_changer, "").changed() {
                self.send_config();
            }

            ui.label("Desired FOV");
            if ui
                .add(
                    DragValue::new(&mut self.config.misc.desired_fov)
                        .speed(0.1)
                        .range(1..=179),
                )
                .changed()
            {
                self.send_config();
            }

            if self.config.misc.fov_changer && ui.button("Reset").clicked() {
                self.config.misc.desired_fov = cs2::DEFAULT_FOV;
                self.send_config();
            }
            ui.end_row();
        });
    }

    fn unsafe_left(&mut self, ui: &mut Ui) {}

    fn unsafe_right(&mut self, ui: &mut Ui) {}

    fn config_settings(&mut self, ui: &mut Ui) {
        ui.columns(2, |cols| {
            let left = &mut cols[0];
            egui::ScrollArea::vertical()
                .id_salt("config_left")
                .show(left, |left| {
                    self.config_left(left);
                });

            let right = &mut cols[1];
            egui::ScrollArea::vertical()
                .id_salt("config_right")
                .show(right, |right| {
                    self.config_right(right);
                });
        });
    }

    fn config_left(&mut self, ui: &mut Ui) {
        if ui.button("Reset").clicked() {
            self.config = Config::default();
            self.send_config();
            info!("loaded default config");
        }
    }

    fn config_right(&mut self, ui: &mut Ui) {}

    fn misc_settings(&mut self, ui: &mut Ui) {
        ui.columns(2, |cols| {
            let left = &mut cols[0];
            egui::ScrollArea::vertical()
                .id_salt("misc_left")
                .show(left, |left| {
                    self.misc_left(left);
                });

            let right = &mut cols[1];
            egui::ScrollArea::vertical()
                .id_salt("misc_right")
                .show(right, |right| {
                    self.misc_right(right);
                });
        });
    }

    fn misc_left(&mut self, ui: &mut Ui) {}

    fn misc_right(&mut self, ui: &mut Ui) {}

    fn add_game_status(&self, ui: &mut Ui) {
        ui.horizontal_top(|ui| {
            ui.label(
                egui::RichText::new(self.status.string())
                    .line_height(Some(8.0))
                    .color(match self.status {
                        AimbotStatus::Working => Colors::GREEN,
                        AimbotStatus::GameNotStarted => Colors::YELLOW,
                    }),
            );

            let mouse_text = match &self.mouse_status {
                DeviceStatus::Working(name) => name,
                DeviceStatus::PermissionsRequired => {
                    "mouse input only works when user is in input group"
                }
                DeviceStatus::Disconnected => "mouse was disconnected",
                DeviceStatus::NotFound => "no mouse was found",
            };
            let color = match &self.mouse_status {
                DeviceStatus::Working(_) => Colors::SUBTEXT,
                _ => Colors::YELLOW,
            };
            ui.label(
                egui::RichText::new(mouse_text)
                    .line_height(Some(8.0))
                    .color(color),
            );
        });
    }

    fn color_picker(&self, ui: &mut Ui, color: &mut Color32) {
        let [mut r, mut g, mut b, _] = color.to_array();
        if ui.add(DragValue::new(&mut r).prefix("r: ")).changed() {
            *color = Color32::from_rgb(r, g, b);
        }
        if ui.add(DragValue::new(&mut g).prefix("g: ")).changed() {
            *color = Color32::from_rgb(r, g, b);
        }
        if ui.add(DragValue::new(&mut b).prefix("b: ")).changed() {
            *color = Color32::from_rgb(r, g, b);
        };
        let (response, painter) = ui.allocate_painter(ui.spacing().interact_size, Sense::hover());
        painter.rect_filled(
            response.rect,
            ui.style().visuals.widgets.inactive.corner_radius,
            *color,
        );
    }

    fn overlay(&self, ctx: &Context) {
        ctx.set_pixels_per_point(1.0);
        let painter = ctx.debug_painter();
        let font = egui::FontId::proportional(16.0);

        let data = &self.data.lock().unwrap();
        if let Some(overlay) = &self.overlay_window {
            overlay
                .window()
                .set_outer_position(winit::dpi::PhysicalPosition::new(
                    data.window_position.x,
                    data.window_position.y,
                ));
            let _ = overlay
                .window()
                .request_inner_size(winit::dpi::PhysicalSize::new(
                    data.window_size.x,
                    data.window_size.y,
                ));
        }

        painter.text(
            pos2(50.0, 50.0),
            Align2::CENTER_CENTER,
            "cock",
            font,
            Color32::WHITE,
        );

        painter.line(
            vec![
                pos2(0.0, 0.0),
                pos2(data.window_size.x as f32, data.window_size.y as f32),
            ],
            egui::Stroke::new(2.0, Colors::TEXT),
        );

        painter.circle(pos2(2560.0, 1440.0), 4.0, Colors::TEXT, Stroke::NONE);

        for player in &data.players {
            self.player_box(&painter, player, data);
            self.skeleton(&painter, player, data);
        }
    }

    fn player_box(&self, painter: &Painter, player: &PlayerData, data: &Data) {
        let midpoint = (player.position + player.head) / 2.0;
        let height = player.head.z - player.position.z + 8.0;
    }

    fn skeleton(&self, painter: &Painter, player: &PlayerData, data: &Data) {
        for (a, b) in &Bones::CONNECTIONS {
            let a = player.bones.get(a).unwrap();
            let b = player.bones.get(b).unwrap();

            let Some(a) = world_to_screen(a, data) else {
                continue;
            };
            let Some(b) = world_to_screen(b, data) else {
                continue;
            };

            painter.line(vec![a, b], Stroke::new(2.0, Colors::TEXT));
        }
    }

    pub fn render(&mut self) {
        use glow::HasContext as _;

        while let Ok(message) = self.rx.try_recv() {
            match message {
                Message::Status(status) => self.status = status,
                Message::MouseStatus(status) => self.mouse_status = status,
                _ => {}
            }
        }

        let self_ptr = self as *mut Self;
        self.gui_window.as_mut().unwrap().make_current().unwrap();
        self.gui_glow
            .as_mut()
            .unwrap()
            .run(self.gui_window.as_mut().unwrap().window(), |ctx| {
                (unsafe { &mut *self_ptr }).gui(ctx)
            });

        unsafe {
            self.gui_gl
                .as_mut()
                .unwrap()
                .clear_color(0.0, 0.0, 0.0, 1.0);
            self.gui_gl.as_mut().unwrap().clear(glow::COLOR_BUFFER_BIT);
        }

        self.gui_glow
            .as_mut()
            .unwrap()
            .paint(self.gui_window.as_mut().unwrap().window());

        self.gui_window.as_mut().unwrap().swap_buffers().unwrap();
        self.gui_window.as_mut().unwrap().window().request_redraw();

        self.overlay_window
            .as_mut()
            .unwrap()
            .make_current()
            .unwrap();
        self.overlay_glow.as_mut().unwrap().run(
            self.overlay_window.as_mut().unwrap().window(),
            move |egui_ctx| {
                (unsafe { &mut *self_ptr }).overlay(egui_ctx);
            },
        );

        unsafe {
            self.overlay_gl
                .as_mut()
                .unwrap()
                .clear_color(0.0, 0.0, 0.0, 0.0);
            self.overlay_gl
                .as_mut()
                .unwrap()
                .clear(glow::COLOR_BUFFER_BIT);
        }

        self.overlay_glow
            .as_mut()
            .unwrap()
            .paint(self.overlay_window.as_mut().unwrap().window());

        self.overlay_window
            .as_mut()
            .unwrap()
            .swap_buffers()
            .unwrap();
        self.overlay_window
            .as_mut()
            .unwrap()
            .window()
            .request_redraw();
    }
}
