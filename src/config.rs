use std::{
    collections::HashMap,
    fs::read_to_string,
    ops::RangeInclusive,
    path::{Path, PathBuf},
    time::Duration,
};

use egui::Color32;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

use crate::{color::Colors, cs2::weapon::Weapon, key_codes::KeyCode};

const REFRESH_RATE: u64 = 100;
pub const LOOP_DURATION: Duration = Duration::from_millis(1000 / REFRESH_RATE);
pub const SLEEP_DURATION: Duration = Duration::from_secs(1);
pub const DEFAULT_CONFIG_NAME: &str = "deadlocked.toml";
pub const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum GameStatus {
    Working,
    GameNotStarted,
}

impl GameStatus {
    pub fn string(&self) -> &str {
        match self {
            GameStatus::Working => "Working",
            GameStatus::GameNotStarted => "Game Not Started",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub aim: AimConfig,
    pub player: PlayerConfig,
    pub hud: HudConfig,
    pub misc: UnsafeConfig,
    pub accent_color: Color32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aim: AimConfig::default(),
            player: PlayerConfig::default(),
            hud: HudConfig::default(),
            misc: UnsafeConfig::default(),
            accent_color: Colors::BLUE,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeaponConfig {
    pub aimbot: AimbotConfig,
    pub rcs: RcsConfig,
    pub triggerbot: TriggerbotConfig,
}

impl WeaponConfig {
    pub fn enabled(enabled: bool) -> Self {
        let aimbot = AimbotConfig {
            enable_override: enabled,
            ..Default::default()
        };
        Self {
            aimbot,
            rcs: RcsConfig::default(),
            triggerbot: TriggerbotConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AimbotConfig {
    pub enable_override: bool,
    pub enabled: bool,
    pub start_bullet: i32,
    pub visibility_check: bool,
    pub flash_check: bool,
    pub fov: f32,
    pub smooth: f32,
    pub multibone: bool,
}

impl Default for AimbotConfig {
    fn default() -> Self {
        Self {
            enable_override: false,
            enabled: true,
            start_bullet: 2,
            visibility_check: true,
            flash_check: true,
            fov: 2.5,
            smooth: 5.0,
            multibone: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RcsConfig {
    pub enable_override: bool,
    pub enabled: bool,
    pub smooth: f32,
}

impl Default for RcsConfig {
    fn default() -> Self {
        Self {
            enable_override: false,
            enabled: false,
            smooth: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerbotConfig {
    pub enable_override: bool,
    pub enabled: bool,
    pub delay: RangeInclusive<u64>,
    pub flash_check: bool,
    pub scope_check: bool,
    pub velocity_check: bool,
    pub velocity_threshold: f32,
    pub head_only: bool,
}

impl Default for TriggerbotConfig {
    fn default() -> Self {
        Self {
            enable_override: false,
            enabled: false,
            delay: 100..=200,
            flash_check: true,
            scope_check: true,
            velocity_check: true,
            velocity_threshold: 100.0,
            head_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AimConfig {
    pub hotkey: KeyCode,
    pub triggerbot_hotkey: KeyCode,
    pub global: WeaponConfig,
    pub weapons: HashMap<Weapon, WeaponConfig>,
}

impl Default for AimConfig {
    fn default() -> Self {
        let mut weapons = HashMap::new();
        for weapon in Weapon::iter() {
            if weapon == Weapon::Unknown {
                continue;
            }
            weapons.insert(weapon, WeaponConfig::default());
        }

        Self {
            hotkey: KeyCode::Mouse5,
            triggerbot_hotkey: KeyCode::Mouse4,
            global: WeaponConfig::enabled(true),
            weapons,
        }
    }
}

#[derive(Debug, Clone, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum DrawMode {
    None,
    Health,
    Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    pub enabled: bool,
    pub draw_box: DrawMode,
    pub box_color: Color32,
    pub draw_skeleton: DrawMode,
    pub skeleton_color: Color32,
    pub health_bar: bool,
    pub armor_bar: bool,
    pub player_name: bool,
    pub weapon_name: bool,
    pub tags: bool,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            draw_box: DrawMode::Color,
            box_color: Colors::TEXT,
            draw_skeleton: DrawMode::Health,
            skeleton_color: Colors::TEXT,
            health_bar: true,
            armor_bar: true,
            player_name: true,
            weapon_name: true,
            tags: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudConfig {
    pub bomb_timer: bool,
    pub fov_circle: bool,
    pub sniper_crosshair: bool,
    pub dropped_weapons: bool,
    pub text_color: Color32,
    pub line_width: f32,
    pub font_size: f32,
    pub debug: bool,
}

impl Default for HudConfig {
    fn default() -> Self {
        Self {
            bomb_timer: true,
            fov_circle: false,
            sniper_crosshair: true,
            dropped_weapons: true,
            text_color: Colors::TEXT,
            line_width: 2.0,
            font_size: 16.0,
            debug: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsafeConfig {
    pub no_flash: bool,
    pub max_flash_alpha: f32,
    pub fov_changer: bool,
    pub desired_fov: u32,
    pub no_smoke: bool,
    pub change_smoke_color: bool,
    pub smoke_color: Color32,
}

impl Default for UnsafeConfig {
    fn default() -> Self {
        Self {
            no_flash: false,
            max_flash_alpha: 127.0,
            fov_changer: false,
            desired_fov: 90,
            no_smoke: false,
            change_smoke_color: false,
            smoke_color: Color32::RED,
        }
    }
}

pub fn get_config_path() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn parse_config(path: &Path) -> Config {
    if !path.exists() {
        return Config::default();
    }

    let config_string = read_to_string(path).unwrap();
    let config = toml::from_str(&config_string);
    if config.is_err() {
        warn!("config file invalid");
    }
    info!("loaded config {:?}", path.file_name().unwrap());
    config.unwrap_or_default()
}

pub fn write_config(config: &Config, path: &Path) {
    let out = toml::to_string(&config).unwrap();
    std::fs::write(path, out).unwrap();
}

pub fn delete_config(path: &Path) {
    if !path.exists() {
        return;
    }

    std::fs::remove_file(path).unwrap();
    info!("deleted config {:?}", path.file_name().unwrap());
}

pub fn available_configs() -> Vec<PathBuf> {
    let dir = get_config_path();
    let mut files = Vec::with_capacity(8);
    for path in std::fs::read_dir(dir).unwrap() {
        let Ok(file) = path else {
            continue;
        };
        if !file.file_type().unwrap().is_file() {
            continue;
        }
        if !file.file_name().to_str().unwrap().ends_with(".toml") {
            continue;
        }
        files.push(file.path())
    }
    if files.is_empty() {
        let path = get_config_path().join(DEFAULT_CONFIG_NAME);
        write_config(&Config::default(), &path);
        files.push(path);
    }
    files
}
