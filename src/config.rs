use std::{
    collections::HashMap, fs::read_to_string, ops::RangeInclusive, path::Path, time::Duration,
};

use log::warn;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::{cs2::weapon::Weapon, key_codes::KeyCode};

const REFRESH_RATE: u64 = 100;
pub const LOOP_DURATION: Duration = Duration::from_millis(1000 / REFRESH_RATE);
pub const SLEEP_DURATION: Duration = Duration::from_secs(1);
pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AimbotStatus {
    Working,
    GameNotStarted,
}

impl AimbotStatus {
    pub fn string(&self) -> &str {
        match self {
            AimbotStatus::Working => "Working",
            AimbotStatus::GameNotStarted => "Game Not Started",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub aimbot: AimbotConfig,
    pub misc: UnsafeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponConfig {
    pub enabled: bool,
    pub start_bullet: i32,
    pub aim_lock: bool,
    pub visibility_check: bool,
    pub flash_check: bool,
    pub fov: f32,
    pub smooth: f32,
    pub multibone: bool,
    pub rcs: bool,
    pub rcs_smooth: f32,
    pub triggerbot: TriggerbotConfig,
}

impl WeaponConfig {
    pub fn enabled(enabled: bool) -> Self {
        Self {
            enabled,
            ..Default::default()
        }
    }
}

impl Default for WeaponConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            start_bullet: 2,
            aim_lock: false,
            visibility_check: true,
            flash_check: true,
            fov: 2.5,
            smooth: 5.0,
            multibone: true,
            rcs: false,
            rcs_smooth: 1.5,
            triggerbot: TriggerbotConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerbotConfig {
    pub enabled: bool,
    pub delay: RangeInclusive<u64>,
    pub visibility_check: bool,
    pub flash_check: bool,
    pub scope_check: bool,
    pub velocity_check: bool,
    pub velocity_threshold: f32,
}

impl Default for TriggerbotConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            delay: 100..=200,
            visibility_check: true,
            flash_check: true,
            scope_check: true,
            velocity_check: true,
            velocity_threshold: 100.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AimbotConfig {
    pub hotkey: KeyCode,
    pub triggerbot_hotkey: KeyCode,
    pub global: WeaponConfig,
    pub weapons: HashMap<Weapon, WeaponConfig>,
}

impl Default for AimbotConfig {
    fn default() -> Self {
        let mut weapons = HashMap::new();
        for weapon in Weapon::iter() {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsafeConfig {
    pub no_flash: bool,
    pub max_flash_alpha: f32,
    pub fov_changer: bool,
    pub desired_fov: u32,
}

impl Default for UnsafeConfig {
    fn default() -> Self {
        Self {
            no_flash: false,
            max_flash_alpha: 0.5,
            fov_changer: false,
            desired_fov: 90,
        }
    }
}

pub fn get_config_path() -> String {
    String::from(
        std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(CONFIG_FILE_NAME)
            .to_str()
            .unwrap(),
    )
}

pub fn parse_config() -> Config {
    let config_path = get_config_path();
    let path = Path::new(config_path.as_str());
    if !path.exists() {
        return Config::default();
    }

    let config_string = read_to_string(get_config_path()).unwrap();
    let config = toml::from_str(&config_string);
    if config.is_err() {
        warn!("config file invalid");
    }
    config.unwrap_or_default()
}

pub fn write_config(config: &Config) {
    let out = toml::to_string(&config).unwrap();
    std::fs::write(get_config_path(), out).unwrap();
}
