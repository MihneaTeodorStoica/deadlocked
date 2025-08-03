use egui::{
    Align, Align2, Button, Color32, Context, DragValue, FontId, Painter, Sense, Stroke, Ui, pos2,
};
use egui_glow::glow;
use glam::vec3;
use log::info;
use strum::IntoEnumIterator;

use crate::{
    app::App,
    color::Colors,
    config::{
        AimbotConfig, Config, DrawMode, GameStatus, VERSION, WeaponConfig, available_configs,
        delete_config, get_config_path, parse_config, write_config,
    },
    constants::cs2,
    cs2::{bones::Bones, weapon::Weapon, weapon_class::WeaponClass},
    data::{Data, PlayerData},
    drag_range::DragRange,
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
}

#[derive(PartialEq)]
pub enum AimbotTab {
    Global,
    Weapon,
}

impl App {
    pub fn send_config(&self) {
        self.send_message(Message::Config(self.config.clone()));
        write_config(&self.config, &self.current_config);
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
                Tab::Config => self.config_settings(ui, ctx),
            }
        });
    }

    fn aimbot_config(&self, weapon: &Weapon) -> &AimbotConfig {
        if let Some(weapon_config) = self.config.aim.weapons.get(weapon) {
            if weapon_config.aimbot.enabled {
                return &weapon_config.aimbot;
            }
        }
        &self.config.aim.global.aimbot
    }

    fn weapon_config(&mut self) -> &mut WeaponConfig {
        if self.aimbot_tab == AimbotTab::Weapon {
            self.config
                .aim
                .weapons
                .get_mut(&self.aimbot_weapon)
                .unwrap()
        } else {
            &mut self.config.aim.global
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
                            if weapon == Weapon::Unknown {
                                continue;
                            }
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
            .selected_text(format!("{:?}", self.config.aim.hotkey))
            .show_ui(ui, |ui| {
                for key_code in KeyCode::iter() {
                    let text = format!("{:?}", &key_code);
                    if ui
                        .selectable_value(&mut self.config.aim.hotkey, key_code, text)
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
            .checkbox(&mut self.weapon_config().aimbot.enabled, enable_label)
            .changed()
        {
            self.send_config();
        }

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.weapon_config().aimbot.fov)
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
                    DragValue::new(&mut self.weapon_config().aimbot.smooth)
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
                    DragValue::new(&mut self.weapon_config().aimbot.start_bullet)
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
            .checkbox(&mut self.weapon_config().aimbot.multibone, "Multibone")
            .changed()
        {
            self.send_config();
        }

        self.section_title(ui, "Checks");

        if ui
            .checkbox(
                &mut self.weapon_config().aimbot.visibility_check,
                "Visibility Check",
            )
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.weapon_config().aimbot.flash_check, "Flash Check")
            .changed()
        {
            self.send_config();
        }
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
            .selected_text(format!("{:?}", self.config.aim.triggerbot_hotkey))
            .show_ui(ui, |ui| {
                for key_code in KeyCode::iter() {
                    let text = format!("{:?}", &key_code);
                    if ui
                        .selectable_value(&mut self.config.aim.triggerbot_hotkey, key_code, text)
                        .clicked()
                    {
                        self.send_config();
                    }
                }
            });

        ui.horizontal(|ui| {
            if ui
                .add(DragRange::new(
                    &mut self.weapon_config().triggerbot.delay,
                    0..=999,
                ))
                .changed()
            {
                self.send_config();
            }
            ui.label("Delay (ms)");
        });

        if ui
            .checkbox(&mut self.weapon_config().triggerbot.head_only, "Head Only")
            .changed()
        {
            self.send_config();
        }

        self.section_title(ui, "Checks");

        if ui
            .checkbox(
                &mut self.weapon_config().triggerbot.flash_check,
                "Flash Check",
            )
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(
                &mut self.weapon_config().triggerbot.scope_check,
                "Scope Check",
            )
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(
                &mut self.weapon_config().triggerbot.velocity_check,
                "Velocity Check",
            )
            .changed()
        {
            self.send_config();
        }

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.weapon_config().triggerbot.velocity_threshold)
                        .range(0..=5000),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("Velocity Threshold");
        });

        self.section_title(ui, "RCS");

        let enable_label = if self.aimbot_tab == AimbotTab::Global {
            "Enable RCS"
        } else {
            "Enable Override"
        };
        if ui
            .checkbox(&mut self.weapon_config().rcs.enabled, enable_label)
            .changed()
        {
            self.send_config();
        }

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.weapon_config().rcs.smooth)
                        .range(0.0..=1.0)
                        .speed(0.02),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("RCS Smooth");
        });
    }

    fn player_settings(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical()
            .id_salt("player")
            .show(ui, |ui| {
                ui.columns(2, |cols| {
                    let left = &mut cols[0];
                    self.player_left(left);
                    let right = &mut cols[1];
                    self.player_right(right);
                });

                self.section_title(ui, "Colors");

                if let Some(color) = self.color_picker(ui, &self.config.player.box_color, "Box") {
                    self.config.player.box_color = color;
                    self.send_config();
                }

                if let Some(color) =
                    self.color_picker(ui, &self.config.player.skeleton_color, "Skeleton")
                {
                    self.config.player.skeleton_color = color;
                    self.send_config();
                }
            });
    }

    fn player_left(&mut self, ui: &mut Ui) {
        ui.label("Player");
        ui.separator();

        if ui
            .checkbox(&mut self.config.player.enabled, "Enable")
            .changed()
        {
            self.send_config();
        }

        egui::ComboBox::new("draw_box", "Box")
            .selected_text(format!("{:?}", self.config.player.draw_box))
            .show_ui(ui, |ui| {
                for mode in DrawMode::iter() {
                    let text = format!("{:?}", &mode);
                    if ui
                        .selectable_value(&mut self.config.player.draw_box, mode, text)
                        .clicked()
                    {
                        self.send_config();
                    }
                }
            });

        egui::ComboBox::new("draw_skeleton", "Skeleton")
            .selected_text(format!("{:?}", self.config.player.draw_skeleton))
            .show_ui(ui, |ui| {
                for mode in DrawMode::iter() {
                    let text = format!("{:?}", &mode);
                    if ui
                        .selectable_value(&mut self.config.player.draw_skeleton, mode, text)
                        .clicked()
                    {
                        self.send_config();
                    }
                }
            });
    }

    fn player_right(&mut self, ui: &mut Ui) {
        ui.label("Info");
        ui.separator();

        if ui
            .checkbox(&mut self.config.player.health_bar, "Health Bar")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.config.player.armor_bar, "Armor Bar")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.config.player.player_name, "Player Name")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.config.player.weapon_name, "Weapon Name")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.config.player.tags, "Show Tags")
            .changed()
        {
            self.send_config();
        }
    }

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

    fn hud_left(&mut self, ui: &mut Ui) {
        ui.label("HUD");
        ui.separator();

        if ui
            .checkbox(&mut self.config.hud.bomb_timer, "Bomb Timer")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.config.hud.fov_circle, "FOV Circle")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(&mut self.config.hud.dropped_weapons, "Dropped Weapons")
            .changed()
        {
            self.send_config();
        }
    }

    fn hud_right(&mut self, ui: &mut Ui) {
        ui.label("Advanced");
        ui.separator();

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.config.hud.line_width)
                        .range(0.1..=8.0)
                        .speed(0.02)
                        .max_decimals(1),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("Line Width");
        });

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.config.hud.font_size)
                        .range(1.0..=99.0)
                        .speed(0.2)
                        .max_decimals(1),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("Font Size");
        });

        if ui
            .checkbox(&mut self.config.hud.debug, "Debug Overlay")
            .changed()
        {
            self.send_config();
        }
    }

    fn unsafe_settings(&mut self, ui: &mut Ui) {
        ui.columns(2, |cols| {
            let left = &mut cols[0];
            egui::ScrollArea::vertical()
                .id_salt("unsafe_left")
                .show(left, |left| {
                    self.unsafe_left(left);
                });

            let right = &mut cols[1];
            egui::ScrollArea::vertical()
                .id_salt("unsafe_right")
                .show(right, |right| {
                    self.unsafe_right(right);
                });
        });

        self.section_title(ui, "Smokes");

        if ui
            .checkbox(&mut self.config.misc.no_smoke, "No Smoke")
            .changed()
        {
            self.send_config();
        }

        if ui
            .checkbox(
                &mut self.config.misc.change_smoke_color,
                "Change Smoke Color",
            )
            .changed()
        {
            self.send_config();
        }

        if let Some(color) = self.color_picker(ui, &self.config.misc.smoke_color, "Smoke Color") {
            self.config.misc.smoke_color = color;
            self.send_config();
        }
    }

    fn unsafe_left(&mut self, ui: &mut Ui) {
        ui.label("No Flash");
        ui.separator();

        if ui
            .checkbox(&mut self.config.misc.no_flash, "No Flash")
            .changed()
        {
            self.send_config();
        }

        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.config.misc.max_flash_alpha)
                        .range(0.0..=255.0)
                        .speed(0.5),
                )
                .changed()
            {
                self.send_config();
            }
            ui.label("Max Flash Alpha");
        });
    }

    fn unsafe_right(&mut self, ui: &mut Ui) {
        ui.label("FOV Changer");
        ui.separator();

        if ui
            .checkbox(&mut self.config.misc.fov_changer, "FOV Changer")
            .changed()
        {
            self.send_config();
        }

        ui.horizontal(|ui| {
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
            ui.label("Desired FOV");

            if ui.button("Reset").clicked() {
                self.config.misc.desired_fov = cs2::DEFAULT_FOV;
                self.send_config();
            }
        });
    }

    fn config_settings(&mut self, ui: &mut Ui, ctx: &Context) {
        ui.columns(2, |cols| {
            let left = &mut cols[0];
            egui::ScrollArea::vertical()
                .id_salt("config_left")
                .show(left, |left| {
                    self.config_left(left, ctx);
                });

            let right = &mut cols[1];

            right.label("Configs");
            right.separator();

            right.horizontal(|right| {
                if right.button("+").clicked() && !self.new_config_name.is_empty() {
                    if !self.new_config_name.ends_with(".toml") {
                        self.new_config_name.push_str(".toml");
                    }
                    self.config = Config::default();
                    let path = get_config_path().join(&self.new_config_name);
                    write_config(&self.config, &path);
                    self.new_config_name.clear();
                    self.current_config = path;
                    self.available_configs = available_configs();
                }
                right.text_edit_singleline(&mut self.new_config_name);
            });

            egui::ScrollArea::vertical()
                .id_salt("config_right")
                .show(right, |right| {
                    self.config_right(right);
                });
        });
    }

    fn config_left(&mut self, ui: &mut Ui, ctx: &Context) {
        ui.label("Config");
        ui.separator();

        if ui.button("Reset").clicked() {
            self.config = Config::default();
            self.send_config();
            info!("loaded default config");
        }

        self.section_title(ui, "Accent Color");

        egui::ComboBox::new("accent_color", "Accent Color")
            .selected_text(
                Colors::ACCENT_COLORS
                    .iter()
                    .find(|c| c.1 == self.config.accent_color)
                    .unwrap_or(&Colors::ACCENT_COLORS[5])
                    .0,
            )
            .show_ui(ui, |ui| {
                for (name, color) in Colors::ACCENT_COLORS {
                    if ui
                        .add(
                            egui::Button::selectable(color == self.config.accent_color, name)
                                .fill(color),
                        )
                        .clicked()
                    {
                        self.config.accent_color = color;
                        ctx.style_mut(|style| style.visuals.selection.bg_fill = color);
                    }
                }
            });
    }

    fn config_right(&mut self, ui: &mut Ui) {
        let mut clicked_config = None;
        let mut delete = None;

        for config in &self.available_configs {
            ui.horizontal(|ui| {
                if ui
                    .add(Button::selectable(
                        *config == self.current_config,
                        config.file_name().unwrap().to_str().unwrap(),
                    ))
                    .clicked()
                {
                    clicked_config = Some(config.clone());
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("\u{f0a7a}").clicked() {
                        delete = Some(config.clone());
                    }
                });
            });
        }

        if let Some(config_path) = clicked_config {
            self.config = parse_config(&config_path);
            self.current_config = config_path;
            self.send_config();
        }

        if let Some(config) = delete {
            delete_config(&config);
            self.available_configs = available_configs();
            self.current_config = self.available_configs[0].clone();
            self.config = parse_config(&self.current_config);
        }
    }

    fn add_game_status(&self, ui: &mut Ui) {
        ui.horizontal_top(|ui| {
            ui.label(
                egui::RichText::new(self.status.string())
                    .line_height(Some(8.0))
                    .color(match self.status {
                        GameStatus::Working => Colors::GREEN,
                        GameStatus::GameNotStarted => Colors::YELLOW,
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

    fn color_picker(&self, ui: &mut Ui, color: &Color32, label: &str) -> Option<Color32> {
        let [mut r, mut g, mut b, _] = color.to_array();
        let res = ui
            .horizontal(|ui| {
                let (response, painter) =
                    ui.allocate_painter(ui.spacing().interact_size, Sense::hover());
                painter.rect_filled(
                    response.rect,
                    ui.style().visuals.widgets.inactive.corner_radius,
                    *color,
                );
                let mut res = ui.add(DragValue::new(&mut r).prefix("r: "));
                res = res.union(ui.add(DragValue::new(&mut g).prefix("g: ")));
                res = res.union(ui.add(DragValue::new(&mut b).prefix("b: ")));
                ui.label(label);
                res
            })
            .inner;

        if res.changed() {
            Some(Color32::from_rgb(r, g, b))
        } else {
            None
        }
    }

    fn overlay(&self, ctx: &Context) {
        ctx.set_pixels_per_point(1.0);
        let painter = ctx.debug_painter();
        let font = FontId::proportional(self.config.hud.font_size);
        let text_stroke = Stroke::new(self.config.hud.line_width, Colors::TEXT);

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

        if self.config.hud.debug {
            painter.line(
                vec![
                    pos2(0.0, 0.0),
                    pos2(data.window_size.x as f32, data.window_size.y as f32),
                ],
                text_stroke,
            );
            painter.line(
                vec![
                    pos2(data.window_size.x as f32, 0.0),
                    pos2(0.0, data.window_size.y as f32),
                ],
                text_stroke,
            );
        }

        if self.config.player.enabled {
            for player in &data.players {
                self.player_box(&painter, player, data);
                self.skeleton(&painter, player, data);
            }
        }

        if self.config.hud.dropped_weapons {
            for weapon in &data.weapons {
                let Some(pos) = world_to_screen(&weapon.1, data) else {
                    continue;
                };
                painter.text(
                    pos,
                    Align2::CENTER_CENTER,
                    weapon.0.as_ref(),
                    font.clone(),
                    self.config.hud.text_color,
                );
            }
        }

        if self.config.hud.bomb_timer && data.bomb.planted {
            if let Some(pos) = world_to_screen(&data.bomb.position, data) {
                painter.text(
                    pos,
                    Align2::CENTER_CENTER,
                    format!("{:.1}", data.bomb.timer),
                    font.clone(),
                    self.config.hud.text_color,
                );
                if data.bomb.being_defused {
                    painter.text(
                        pos2(pos.x, pos.y + self.config.hud.font_size),
                        Align2::CENTER_CENTER,
                        "defusing",
                        font,
                        self.config.hud.text_color,
                    );
                }
            }

            let fraction = (data.bomb.timer / 40.0).clamp(0.0, 1.0);
            let color = self.health_color((fraction * 100.0) as i32);
            painter.line(
                vec![
                    pos2(0.0, data.window_size.y as f32),
                    pos2(
                        data.window_size.x as f32 * fraction,
                        data.window_size.y as f32,
                    ),
                ],
                Stroke::new(self.config.hud.line_width * 3.0, color),
            );
        }

        // fov circle
        if self.config.hud.fov_circle && data.in_game {
            let weapon_config = self.aimbot_config(&data.weapon);
            let aim_fov = weapon_config.fov;
            let fov = if self.config.misc.fov_changer {
                self.config.misc.desired_fov
            } else {
                cs2::DEFAULT_FOV
            } as f32;
            let radius = (aim_fov.to_radians() / 2.0).tan() / (fov.to_radians() / 2.0).tan()
                * data.window_size.x as f32
                / 2.0;
            painter.circle_stroke(
                pos2(
                    data.window_size.x as f32 / 2.0,
                    data.window_size.y as f32 / 2.0,
                ),
                radius,
                Stroke::new(self.config.hud.line_width, Color32::WHITE),
            );
        }

        if self.config.hud.sniper_crosshair
            && WeaponClass::from_string(data.weapon.as_ref()) == WeaponClass::Sniper
        {
            painter.line(
                vec![
                    pos2(
                        data.window_size.x as f32 / 2.0,
                        data.window_size.y as f32 / 2.0 - 50.0,
                    ),
                    pos2(
                        data.window_size.x as f32 / 2.0,
                        data.window_size.y as f32 / 2.0 + 50.0,
                    ),
                ],
                text_stroke,
            );
            painter.line(
                vec![
                    pos2(
                        data.window_size.x as f32 / 2.0 - 50.0,
                        data.window_size.y as f32 / 2.0,
                    ),
                    pos2(
                        data.window_size.x as f32 / 2.0 + 50.0,
                        data.window_size.y as f32 / 2.0,
                    ),
                ],
                text_stroke,
            );
        }
    }

    fn player_box(&self, painter: &Painter, player: &PlayerData, data: &Data) {
        let health_color = self.health_color(player.health);
        let color = match &self.config.player.draw_box {
            crate::config::DrawMode::None => health_color,
            crate::config::DrawMode::Health => health_color,
            crate::config::DrawMode::Color => self.config.player.box_color,
        };
        let stroke = Stroke::new(self.config.hud.line_width, color);
        let font = egui::FontId::proportional(self.config.hud.font_size);

        let midpoint = (player.position + player.head) / 2.0;
        let height = player.head.z - player.position.z + 24.0;
        let half_height = height / 2.0;
        let top = midpoint + vec3(0.0, 0.0, half_height);
        let bottom = midpoint - vec3(0.0, 0.0, half_height);

        let Some(top) = world_to_screen(&top, data) else {
            return;
        };
        let Some(bottom) = world_to_screen(&bottom, data) else {
            return;
        };
        let half_height = bottom.y - top.y;
        let width = half_height / 2.0;
        let half_width = width / 2.0;
        // quarter width
        let qw = half_width - 2.0;
        // eigth width
        let ew = qw / 2.0;

        let tl = pos2(top.x - half_width, top.y);
        let tr = pos2(top.x + half_width, top.y);
        let bl = pos2(bottom.x - half_width, bottom.y);
        let br = pos2(bottom.x + half_width, bottom.y);

        if self.config.player.draw_box != DrawMode::None {
            painter.line(
                vec![pos2(tl.x + ew, tl.y), tl, pos2(tl.x, tl.y + qw)],
                stroke,
            );
            painter.line(
                vec![pos2(tr.x - ew, tl.y), tr, pos2(tr.x, tr.y + qw)],
                stroke,
            );
            painter.line(
                vec![pos2(bl.x + ew, bl.y), bl, pos2(bl.x, bl.y - qw)],
                stroke,
            );
            painter.line(
                vec![pos2(br.x - ew, bl.y), br, pos2(br.x, br.y - qw)],
                stroke,
            );
        }

        // health bar
        if self.config.player.health_bar {
            let x = bl.x - self.config.hud.line_width * 2.0;
            let delta = bl.y - tl.y;
            painter.line(
                vec![
                    pos2(x, bl.y),
                    pos2(x, bl.y - (delta * player.health as f32 / 100.0)),
                ],
                Stroke::new(self.config.hud.line_width, health_color),
            );
        }

        if self.config.player.armor_bar && player.armor > 0 {
            let x = bl.x
                - self.config.hud.line_width
                    * if self.config.player.health_bar {
                        4.0
                    } else {
                        2.0
                    };
            let delta = bl.y - tl.y;
            painter.line(
                vec![
                    pos2(x, bl.y),
                    pos2(x, bl.y - (delta * player.armor as f32 / 100.0)),
                ],
                Stroke::new(self.config.hud.line_width, Color32::BLUE),
            );
        }

        let mut offset = 0.0;
        let font_size = self.config.hud.font_size;
        if self.config.player.player_name {
            painter.text(
                pos2(tr.x + ew, tr.y + offset),
                Align2::LEFT_TOP,
                &player.name,
                font.clone(),
                self.config.hud.text_color,
            );
            offset += font_size;
        }

        if self.config.player.weapon_name {
            painter.text(
                pos2(tr.x + ew, tr.y + offset),
                Align2::LEFT_TOP,
                player.weapon.as_ref(),
                font.clone(),
                self.config.hud.text_color,
            );
            offset += font_size;
        }

        if self.config.player.tags && player.has_defuser {
            painter.text(
                pos2(tr.x + ew, tr.y + offset),
                Align2::LEFT_TOP,
                "defuser",
                font.clone(),
                self.config.hud.text_color,
            );
            offset += font_size;
        }

        if self.config.player.tags && player.has_helmet {
            painter.text(
                pos2(tr.x + ew, tr.y + offset),
                Align2::LEFT_TOP,
                "helmet",
                font.clone(),
                self.config.hud.text_color,
            );
            offset += font_size;
        }

        if self.config.player.tags && player.has_bomb {
            painter.text(
                pos2(tr.x + ew, tr.y + offset),
                Align2::LEFT_TOP,
                "bomb",
                font.clone(),
                self.config.hud.text_color,
            );
        }
    }

    fn skeleton(&self, painter: &Painter, player: &PlayerData, data: &Data) {
        let color = match &self.config.player.draw_skeleton {
            crate::config::DrawMode::None => return,
            crate::config::DrawMode::Health => self.health_color(player.health),
            crate::config::DrawMode::Color => self.config.player.skeleton_color,
        };
        let stroke = Stroke::new(self.config.hud.line_width, color);

        for (a, b) in &Bones::CONNECTIONS {
            let a = player.bones.get(a).unwrap();
            let b = player.bones.get(b).unwrap();

            let Some(a) = world_to_screen(a, data) else {
                continue;
            };
            let Some(b) = world_to_screen(b, data) else {
                continue;
            };

            painter.line(vec![a, b], stroke);
        }
    }

    fn health_color(&self, health: i32) -> Color32 {
        let health = health.clamp(0, 100);

        let (r, g) = if health <= 50 {
            let factor = health as f32 / 50.0;
            (255, (255.0 * factor) as u8)
        } else {
            let factor = 1.0 - (health - 50) as f32 / 50.0;
            ((255.0 * factor) as u8, 255)
        };

        Color32::from_rgb(r, g, 0)
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
    }
}
