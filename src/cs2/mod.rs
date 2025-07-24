use glam::{IVec2, Mat4, Vec2, Vec3};
use log::{debug, info, warn};
use player::Player;
use rcs::Recoil;

use crate::{
    aimbot::Aimbot,
    config::{Config, WeaponConfig},
    constants::cs2,
    cs2::{bones::Bones, offsets::Offsets, target::Target, weapon::Weapon},
    data::{Data, PlayerData},
    key_codes::KeyCode,
    math::{angles_from_vector, vec2_clamp},
    mouse::Mouse,
    process::Process,
};

mod aimbot;
pub mod bones;
mod fov_changer;
mod noflash;
mod offsets;
mod planted_c4;
mod player;
mod rcs;
mod smoke;
mod target;
pub mod weapon;
mod weapon_class;

#[derive(Debug)]
pub struct CS2 {
    is_valid: bool,
    process: Process,
    offsets: Offsets,
    target: Target,
    players: Vec<Player>,
    recoil: Recoil,
    weapon: Weapon,
}

impl Aimbot for CS2 {
    fn is_valid(&self) -> bool {
        self.is_valid && self.process.is_valid()
    }

    fn setup(&mut self) {
        let Some(process) = Process::open(cs2::PROCESS_NAME) else {
            self.is_valid = false;
            return;
        };
        info!("process found, pid: {}", process.pid);
        self.process = process;

        self.offsets = match self.find_offsets() {
            Some(offsets) => offsets,
            None => {
                self.process = Process::new(-1);
                self.is_valid = false;
                return;
            }
        };
        info!("offsets found");

        self.is_valid = true;
    }

    fn run(&mut self, config: &Config, mouse: &mut Mouse) {
        if !self.process.is_valid() {
            self.is_valid = false;
            return;
        }

        self.cache_players();

        self.no_flash(config);
        self.fov_changer(config);

        if self.weapon_config(config).rcs {
            self.rcs(mouse);
        }

        self.find_target();

        if self.is_button_down(&config.aimbot.hotkey) {
            self.aimbot(config, mouse);
        }
    }

    fn data(&self, data: &mut Data) {
        data.players.clear();
        for player in &self.players {
            let player_data = PlayerData {
                health: player.health(self),
                armor: player.armor(self),
                team: player.team(self),
                position: player.position(self),
                head: player.bone_position(self, Bones::Head.u64()),
                name: player.name(self),
                weapon: player.weapon(self),
                bones: player.all_bones(self),
            };
            data.players.push(player_data);
        }

        let local_player = Player::local_player(self);
        if let Some(local_player) = local_player {
            data.weapon = local_player.weapon(self);
            data.in_game = true;
            data.is_ffa = self.is_ffa();
        } else {
            data.weapon = Weapon::default();
            data.in_game = false;
        }

        data.view_matrix = self.process.read::<Mat4>(self.offsets.direct.view_matrix);
        let sdl_window = self.process.read::<u64>(self.offsets.direct.sdl_window);
        if sdl_window == 0 {
            data.window_position = IVec2::ZERO;
            data.window_size = IVec2::ZERO;
        } else {
            data.window_position = self.process.read(sdl_window + 0x18);
            data.window_size = self.process.read(sdl_window + 0x18 + 0x08);
        }
    }
}

impl CS2 {
    pub fn new() -> Self {
        Self {
            is_valid: false,
            process: Process::new(-1),
            offsets: Offsets::default(),
            target: Target::default(),
            players: Vec::with_capacity(64),
            recoil: Recoil::default(),
            weapon: Weapon::default(),
        }
    }

    fn weapon_config<'a>(&mut self, config: &'a Config) -> &'a WeaponConfig {
        if config.aimbot.weapons.get(&self.weapon).unwrap().enabled {
            config.aimbot.weapons.get(&self.weapon).unwrap()
        } else {
            &config.aimbot.global
        }
    }

    fn angle_to_target(&self, local_player: &Player, position: &Vec3, aim_punch: &Vec2) -> Vec2 {
        let eye_position = local_player.eye_position(self);
        let forward = (position - eye_position).normalize();

        let mut angles = angles_from_vector(&forward) - aim_punch;
        vec2_clamp(&mut angles);

        angles
    }

    fn find_offsets(&self) -> Option<Offsets> {
        let mut offsets = Offsets::default();

        offsets.library.client = self.process.module_base_address(cs2::CLIENT_LIB)?;
        offsets.library.engine = self.process.module_base_address(cs2::ENGINE_LIB)?;
        offsets.library.tier0 = self.process.module_base_address(cs2::TIER0_LIB)?;
        offsets.library.input = self.process.module_base_address(cs2::INPUT_LIB)?;
        offsets.library.sdl = self.process.module_base_address(cs2::SDL_LIB)?;

        let Some(resource_offset) = self
            .process
            .get_interface_offset(offsets.library.engine, "GameResourceServiceClientV0")
        else {
            warn!("could not get offset for GameResourceServiceClient");
            return None;
        };
        offsets.interface.resource = resource_offset;

        offsets.interface.entity = self.process.read(offsets.interface.resource + 0x50);
        offsets.interface.player = offsets.interface.entity + 0x10;

        let Some(cvar_address) = self
            .process
            .get_interface_offset(offsets.library.tier0, "VEngineCvar0")
        else {
            warn!("could not get convar interface offset");
            return None;
        };
        offsets.interface.cvar = cvar_address;
        let Some(input_address) = self
            .process
            .get_interface_offset(offsets.library.input, "InputSystemVersion0")
        else {
            warn!("could not get input interface offset");
            return None;
        };
        offsets.interface.input = input_address;

        let Some(local_player) = self.process.scan_pattern(
            &[
                0x48, 0x83, 0x3D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0F, 0x95, 0xC0, 0xC3,
            ],
            "xxx????xxxxx".as_bytes(),
            offsets.library.client,
        ) else {
            warn!("could not find local player offset");
            return None;
        };
        offsets.direct.local_player = self.process.get_relative_address(local_player, 0x03, 0x08);
        offsets.direct.button_state = self.process.read::<u32>(
            self.process
                .get_interface_function(offsets.interface.input, 19)
                + 0x14,
        ) as u64;

        let Some(view_matrix) = self.process.scan_pattern(
            &[
                0x48, 0x8D, 0x05, 0x00, 0x00, 0x00, 0x00, 0x4C, 0x8D, 0x05, 0x00, 0x00, 0x00, 0x00,
                0x48, 0x8D, 0x0D,
            ],
            "xxx????xxx????xxx".as_bytes(),
            offsets.library.client,
        ) else {
            warn!("could not find view matrix offset");
            return None;
        };
        offsets.direct.view_matrix =
            self.process
                .get_relative_address(view_matrix + 0x07, 0x03, 0x07);

        let Some(sdl_window) = self
            .process
            .get_module_export(offsets.library.sdl, "SDL_GetKeyboardFocus")
        else {
            warn!("could not find sdl window offset");
            return None;
        };
        let sdl_window = self.process.get_relative_address(sdl_window, 0x02, 0x06);
        let sdl_window = self.process.read(sdl_window);
        offsets.direct.sdl_window = self.process.get_relative_address(sdl_window, 0x03, 0x07);

        let Some(planted_c4) = self.process.scan_pattern(
            &[0x00, 0x00, 0x00, 0x00, 0x8B, 0x10, 0x85, 0xD2, 0x0F, 0x8F],
            "????xxxxxx".as_bytes(),
            offsets.library.client,
        ) else {
            warn!("could not find planted c4 offset");
            return None;
        };
        offsets.direct.planted_c4 = planted_c4;

        let Some(global_vars) = self.process.scan_pattern(
            &[
                0x8D, 0x00, 0x00, 0x00, 0x00, 0x00, 0x48, 0x89, 0x35, 0x00, 0x00, 0x00, 0x00, 0x48,
                0x89, 0x00, 0x00, 0xC3,
            ],
            "x?????xxx????xx??x".as_bytes(),
            offsets.library.client,
        ) else {
            warn!("could not find global vars offset");
            return None;
        };
        offsets.direct.global_vars = self.process.get_relative_address(global_vars, 0x09, 0x0D);

        let Some(ffa_address) = self
            .process
            .get_convar(offsets.interface.cvar, "mp_teammates_are_enemies")
        else {
            warn!("could not get mp_tammates_are_enemies convar offset");
            return None;
        };
        offsets.convar.ffa = ffa_address;
        let Some(sensitivity_address) = self
            .process
            .get_convar(offsets.interface.cvar, "sensitivity")
        else {
            warn!("could not get sensitivity convar offset");
            return None;
        };
        offsets.convar.sensitivity = sensitivity_address;

        let client_module_size = self.process.module_size(offsets.library.client);
        let client_dump = self.process.dump_module(offsets.library.client);

        let base = offsets.library.client;
        for index in (0..=(client_module_size - 8)).rev().step_by(8) {
            let Some((netvar_name, network_enable)) =
                self.netvar_name(&client_dump, index, base, client_module_size)
            else {
                continue;
            };
            self.process_netvar(
                &mut offsets,
                &client_dump,
                netvar_name,
                network_enable,
                index,
            );

            use offsets::Offset as _;
            if offsets.all_found() {
                debug!("offsets: {:?}", offsets);
                return Some(offsets);
            }
        }

        warn!("not all offsets found: {:?}", offsets);
        None
    }

    fn netvar_name(
        &self,
        client_dump: &[u8],
        index: u64,
        base: u64,
        size: u64,
    ) -> Option<(String, bool)> {
        let mut ne_pointer = self.process.read_vec::<u64>(client_dump, index);

        if (base..base + size).contains(&ne_pointer) {
            ne_pointer = self.process.read_vec(client_dump, ne_pointer - base);
        }

        let network_enable = if (base..base + size).contains(&ne_pointer) {
            let name = self.process.read_string_vec(client_dump, ne_pointer - base);
            name.to_lowercase() == "MNetworkEnable".to_lowercase()
        } else {
            false
        };

        let name_pointer = self.process.read_vec::<u64>(
            client_dump,
            index + if network_enable { 0x08 } else { 0x00 },
        );

        if !(base..base + size).contains(&name_pointer) {
            return None;
        }

        Some((
            self.process
                .read_string_vec(client_dump, name_pointer - base),
            network_enable,
        ))
    }

    fn process_netvar(
        &self,
        offsets: &mut Offsets,
        client_dump: &[u8],
        netvar_name: String,
        network_enable: bool,
        index: u64,
    ) {
        match netvar_name.as_str() {
            "m_sSanitizedPlayerName" => {
                if !network_enable || offsets.controller.name != 0 {
                    return;
                }
                offsets.controller.name = self.get_netvar(client_dump, index + 0x18);
            }
            "m_hPawn" => {
                if !network_enable || offsets.controller.pawn != 0 {
                    return;
                }
                offsets.controller.pawn = self.get_netvar(client_dump, index + 0x18);
            }
            "m_iDesiredFOV" => {
                if offsets.controller.desired_fov != 0 {
                    return;
                }
                offsets.controller.desired_fov =
                    self.process.read_vec::<u32>(client_dump, index + 0x8) as u64;
            }
            "m_iHealth" => {
                if !network_enable || offsets.pawn.health != 0 {
                    return;
                }
                offsets.pawn.health = self.get_netvar(client_dump, index + 0x18);
            }
            "m_ArmorValue" => {
                if !network_enable || offsets.pawn.armor != 0 {
                    return;
                }
                offsets.pawn.armor = self.get_netvar(client_dump, index + 0x18);
            }
            "m_iTeamNum" => {
                if !network_enable || offsets.pawn.team != 0 {
                    return;
                }
                offsets.pawn.team = self.get_netvar(client_dump, index + 0x18);
            }
            "m_lifeState" => {
                if !network_enable || offsets.pawn.life_state != 0 {
                    return;
                }
                offsets.pawn.life_state = self.get_netvar(client_dump, index + 0x18);
            }
            "m_pClippingWeapon" => {
                if offsets.pawn.weapon != 0 {
                    return;
                }
                offsets.pawn.weapon = self.get_netvar(client_dump, index + 0x10);
            }
            "m_flFOVSensitivityAdjust" => {
                if offsets.pawn.fov_multiplier != 0 {
                    return;
                }
                offsets.pawn.fov_multiplier = self.get_netvar(client_dump, index + 0x08);
            }
            "m_pGameSceneNode" => {
                if offsets.pawn.game_scene_node != 0 {
                    return;
                }
                offsets.pawn.game_scene_node = self.get_netvar(client_dump, index + 0x10);
            }
            "m_vecViewOffset" => {
                if !network_enable || offsets.pawn.eye_offset != 0 {
                    return;
                }
                offsets.pawn.eye_offset = self.get_netvar(client_dump, index + 0x18);
            }
            "m_vecAbsVelocity" => {
                if offsets.pawn.velocity != 0 {
                    return;
                }
                offsets.pawn.velocity = self.get_netvar(client_dump, index + 0x08);
            }
            "m_aimPunchCache" => {
                if !network_enable || offsets.pawn.aim_punch_cache != 0 {
                    return;
                }
                offsets.pawn.aim_punch_cache = self.get_netvar(client_dump, index + 0x18);
            }
            "m_iShotsFired" => {
                if !network_enable || offsets.pawn.shots_fired != 0 {
                    return;
                }
                offsets.pawn.shots_fired = self.get_netvar(client_dump, index + 0x18);
            }
            "v_angle" => {
                if offsets.pawn.view_angles != 0 {
                    return;
                }
                offsets.pawn.view_angles = self.get_netvar(client_dump, index + 0x08);
            }
            "m_entitySpottedState" => {
                if !network_enable || offsets.pawn.spotted_state != 0 {
                    return;
                }
                let offset = self.get_netvar(client_dump, index + 0x18);
                if !(10000..=14000).contains(&offset) {
                    return;
                }
                offsets.pawn.spotted_state = offset;
            }
            "m_iIDEntIndex" => {
                if offsets.pawn.crosshair_entity != 0 {
                    return;
                }
                offsets.pawn.crosshair_entity = self.get_netvar(client_dump, index + 0x10);
            }
            "m_bIsScoped" => {
                if !network_enable || offsets.pawn.is_scoped != 0 {
                    return;
                }
                offsets.pawn.is_scoped = self.get_netvar(client_dump, index + 0x18);
            }
            "m_flFlashMaxAlpha" => {
                if offsets.pawn.flash_alpha != 0 {
                    return;
                }
                offsets.pawn.flash_alpha = self.get_netvar(client_dump, index + 0x10);
            }
            "m_flFlashDuration" => {
                if offsets.pawn.flash_duration != 0 {
                    return;
                }
                offsets.pawn.flash_duration = self.get_netvar(client_dump, index + 0x10);
            }
            "m_pCameraServices" => {
                if !network_enable || offsets.pawn.camera_services != 0 {
                    return;
                }
                offsets.pawn.camera_services = self.get_netvar(client_dump, index + 0x18);
            }
            "m_pItemServices" => {
                if offsets.pawn.item_services != 0 {
                    return;
                }
                offsets.pawn.item_services = self.get_netvar(client_dump, index + 0x08);
            }
            "m_pWeaponServices" => {
                if offsets.pawn.weapon_services != 0 {
                    return;
                }
                offsets.pawn.weapon_services = self.get_netvar(client_dump, index + 0x08);
            }
            "m_bDormant" => {
                if offsets.game_scene_node.dormant != 0 {
                    return;
                }
                offsets.game_scene_node.dormant = self.get_netvar(client_dump, index + 0x08);
            }
            "m_vecAbsOrigin" => {
                if !network_enable || offsets.game_scene_node.origin != 0 {
                    return;
                }
                offsets.game_scene_node.origin = self.get_netvar(client_dump, index + 0x18);
            }
            "m_modelState" => {
                if offsets.game_scene_node.model_state != 0 {
                    return;
                }
                offsets.game_scene_node.model_state = self.get_netvar(client_dump, index + 0x08);
            }
            "m_bDidSmokeEffect" => {
                if !network_enable || offsets.smoke.did_smoke_effect != 0 {
                    return;
                }
                offsets.smoke.did_smoke_effect = self.get_netvar(client_dump, index + 0x18);
            }
            "m_vSmokeColor" => {
                if !network_enable || offsets.smoke.smoke_color != 0 {
                    return;
                }
                offsets.smoke.smoke_color = self.get_netvar(client_dump, index + 0x18);
            }
            "m_bSpotted" => {
                if offsets.spotted_state.spotted != 0 {
                    return;
                }
                offsets.spotted_state.spotted = self.get_netvar(client_dump, index + 0x10);
            }
            "m_bSpottedByMask" => {
                if !network_enable || offsets.spotted_state.mask != 0 {
                    return;
                }
                offsets.spotted_state.mask = self.get_netvar(client_dump, index + 0x18);
            }
            "m_iFOV" => {
                if offsets.camera_services.fov != 0 {
                    return;
                }
                offsets.camera_services.fov = self.get_netvar(client_dump, index + 0x08);
            }
            "m_bHasDefuser" => {
                if offsets.item_services.has_defuser != 0 {
                    return;
                }
                offsets.item_services.has_defuser = self.get_netvar(client_dump, index + 0x10);
            }
            "m_bHasHelmet" => {
                if !network_enable || offsets.item_services.has_helmet != 0 {
                    return;
                }
                offsets.item_services.has_helmet = self.get_netvar(client_dump, index + 0x18);
            }
            "m_hMyWeapons" => {
                if offsets.weapon_services.weapons != 0 {
                    return;
                }
                offsets.weapon_services.weapons = self.get_netvar(client_dump, index + 0x08);
            }
            "m_bC4Activated" => {
                if offsets.planted_c4.is_activated != 0 {
                    return;
                }
                offsets.planted_c4.is_activated = self.get_netvar(client_dump, index + 0x10);
            }
            "m_bBombTicking" => {
                if offsets.planted_c4.is_ticking != 0 {
                    return;
                }
                offsets.planted_c4.is_ticking = self.get_netvar(client_dump, index + 0x10);
            }
            "m_nBombSite" => {
                if !network_enable || offsets.planted_c4.bomb_site != 0 {
                    return;
                }
                offsets.planted_c4.bomb_site = self.get_netvar(client_dump, index + 0x18);
            }
            "m_flC4Blow" => {
                if offsets.planted_c4.blow_time != 0 {
                    return;
                }
                offsets.planted_c4.blow_time = self.get_netvar(client_dump, index + 0x10);
            }
            "m_bBeingDefused" => {
                if !network_enable || offsets.planted_c4.being_defused != 0 {
                    return;
                }
                offsets.planted_c4.being_defused = self.get_netvar(client_dump, index + 0x18);
            }
            _ => {}
        }
    }

    fn get_netvar(&self, client_dump: &[u8], address: u64) -> u64 {
        self.process.read_vec::<u32>(client_dump, address) as u64
    }

    // convars
    fn get_sensitivity(&self) -> f32 {
        self.process.read(self.offsets.convar.sensitivity + 0x40)
    }

    fn is_ffa(&self) -> bool {
        self.process.read::<u32>(self.offsets.convar.ffa + 0x40) == 1
    }

    // misc
    fn is_button_down(&self, button: &KeyCode) -> bool {
        if *button == KeyCode::None {
            return true;
        }
        // what the actual fuck is happening here?
        let value = self.process.read::<u32>(
            self.offsets.interface.input
                + (((button.u64() >> 5) * 4) + self.offsets.direct.button_state),
        );
        ((value >> (button.u64() & 31)) & 1) != 0
    }

    fn distance_scale(&self, distance: f32) -> f32 {
        if distance > 500.0 {
            1.0
        } else {
            5.0 - (distance / 125.0)
        }
    }
}
