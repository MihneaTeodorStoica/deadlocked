use std::time::Instant;

use glam::{IVec2, Mat4, Vec2, Vec3};
use log::{debug, info, warn};
use player::Player;
use rcs::Recoil;

use crate::{
    config::{AimbotConfig, Config, RcsConfig, TriggerbotConfig},
    constants::cs2::{self, TEAM_CT, TEAM_T},
    cs2::{
        bones::Bones, offsets::Offsets, planted_c4::PlantedC4, smoke::Smoke, target::Target,
        triggerbot::Triggerbot, weapon::Weapon,
    },
    data::{Data, PlayerData},
    game::Game,
    key_codes::KeyCode,
    math::{angles_from_vector, vec2_clamp},
    mouse::Mouse,
    process::Process,
    schema::Schema,
};

mod aimbot;
pub mod bones;
mod fov_changer;
mod no_flash;
mod offsets;
mod planted_c4;
pub mod player;
mod rcs;
mod smoke;
mod target;
mod triggerbot;
pub mod weapon;
pub mod weapon_class;

#[derive(Debug)]
pub struct CS2 {
    is_valid: bool,
    process: Process,
    offsets: Offsets,
    target: Target,
    players: Vec<Player>,
    entities: Vec<Entity>,
    recoil: Recoil,
    trigger: Triggerbot,
    weapon: Weapon,
}

impl Game for CS2 {
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
        self.cache_entities();

        for entity in &self.entities {
            if let Entity::Smoke(smoke) = entity {
                if config.misc.no_smoke {
                    smoke.disable(self);
                }

                if config.misc.change_smoke_color {
                    smoke.color(self, &config.misc.smoke_color);
                }
            }
        }

        self.no_flash(config);
        self.fov_changer(config);

        self.rcs(config, mouse);
        if self.is_button_down(&config.aim.triggerbot_hotkey) {
            self.triggerbot(config);
        }
        self.triggerbot_shoot(mouse);

        self.find_target();

        if self.is_button_down(&config.aim.hotkey) {
            self.aimbot(config, mouse);
        }
    }

    fn data(&self, data: &mut Data) {
        data.players.clear();
        data.weapons.clear();
        let Some(local_player) = Player::local_player(self) else {
            data.weapon = Weapon::default();
            data.in_game = false;
            return;
        };
        let local_team = local_player.team(self);
        if local_team != TEAM_T && local_team != TEAM_CT {
            data.weapon = Weapon::default();
            data.in_game = false;
            return;
        }
        for player in &self.players {
            if !self.is_ffa() && player.team(self) == local_team {
                continue;
            }
            let player_data = PlayerData {
                health: player.health(self),
                armor: player.armor(self),
                position: player.position(self),
                head: player.bone_position(self, Bones::Head.u64()),
                name: player.name(self),
                weapon: player.weapon(self),
                bones: player.all_bones(self),
                has_defuser: player.has_defuser(self),
                has_helmet: player.has_helmet(self),
                has_bomb: player.has_bomb(self),
            };
            data.players.push(player_data);
        }

        data.weapons.clear();
        for entity in &self.entities {
            if let Entity::Weapon(weapon, position) = entity {
                data.weapons.push((weapon.clone(), *position));
            }
        }

        data.weapon = local_player.weapon(self);
        data.in_game = true;
        data.is_ffa = self.is_ffa();

        data.view_matrix = self.process.read::<Mat4>(self.offsets.direct.view_matrix);
        let sdl_window = self.process.read::<u64>(self.offsets.direct.sdl_window);
        if sdl_window == 0 {
            data.window_position = IVec2::ZERO;
            data.window_size = IVec2::ZERO;
        } else {
            data.window_position = self.process.read(sdl_window + 0x18);
            data.window_size = self.process.read(sdl_window + 0x18 + 0x08);
        }

        if let Some(bomb) = PlantedC4::get(self) {
            data.bomb.planted = bomb.is_planted(self);
            data.bomb.timer = bomb.time_to_explosion(self);
            data.bomb.position = bomb.position(self);
            data.bomb.being_defused = bomb.is_being_defused(self);
        } else {
            data.bomb.planted = false;
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
            entities: Vec::with_capacity(128),
            recoil: Recoil::default(),
            trigger: Triggerbot::default(),
            weapon: Weapon::default(),
        }
    }

    fn aimbot_config<'a>(&mut self, config: &'a Config) -> &'a AimbotConfig {
        if let Some(weapon_config) = config.aim.weapons.get(&self.weapon) {
            if weapon_config.aimbot.enable_override {
                return &weapon_config.aimbot;
            }
        }
        &config.aim.global.aimbot
    }

    fn rcs_config<'a>(&mut self, config: &'a Config) -> &'a RcsConfig {
        if let Some(weapon_config) = config.aim.weapons.get(&self.weapon) {
            if weapon_config.rcs.enable_override {
                return &weapon_config.rcs;
            }
        }
        &config.aim.global.rcs
    }

    fn triggerbot_config<'a>(&mut self, config: &'a Config) -> &'a TriggerbotConfig {
        if let Some(weapon_config) = config.aim.weapons.get(&self.weapon) {
            if weapon_config.triggerbot.enable_override {
                return &weapon_config.triggerbot;
            }
        }
        &config.aim.global.triggerbot
    }

    fn angle_to_target(&self, local_player: &Player, position: &Vec3, aim_punch: &Vec2) -> Vec2 {
        let eye_position = local_player.eye_position(self);
        let forward = (position - eye_position).normalize();

        let mut angles = angles_from_vector(&forward) - aim_punch;
        vec2_clamp(&mut angles);

        angles
    }

    fn find_offsets(&self) -> Option<Offsets> {
        let start = Instant::now();
        let mut offsets = Offsets::default();

        offsets.library.client = self.process.module_base_address(cs2::CLIENT_LIB)?;
        offsets.library.engine = self.process.module_base_address(cs2::ENGINE_LIB)?;
        offsets.library.tier0 = self.process.module_base_address(cs2::TIER0_LIB)?;
        offsets.library.input = self.process.module_base_address(cs2::INPUT_LIB)?;
        offsets.library.sdl = self.process.module_base_address(cs2::SDL_LIB)?;
        offsets.library.schema = self.process.module_base_address(cs2::SCHEMA_LIB)?;

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

        let Some(local_player) = self
            .process
            .scan("48 83 3D ? ? ? ? 00 0F 95 C0 C3", offsets.library.client)
        else {
            warn!("could not find local player offset");
            return None;
        };
        offsets.direct.local_player = self.process.get_relative_address(local_player, 0x03, 0x08);
        offsets.direct.button_state = self.process.read::<u32>(
            self.process
                .get_interface_function(offsets.interface.input, 19)
                + 0x14,
        ) as u64;

        let Some(view_matrix) = self
            .process
            .scan("4C 8D 0D ? ? ? ? 4C 89 E6 4C 8D 05", offsets.library.client)
        else {
            warn!("could not find view matrix offset");
            return None;
        };
        offsets.direct.view_matrix =
            self.process
                .get_relative_address(view_matrix + 0x0A, 0x03, 0x07);

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

        let Some(planted_c4) = self
            .process
            .scan("48 8D 05 ? ? ? ? 8B 10 85 D2 7E", offsets.library.client)
        else {
            warn!("could not find planted c4 offset");
            return None;
        };
        offsets.direct.planted_c4 = self.process.get_relative_address(planted_c4, 0x03, 0x0F);

        let Some(global_vars) = self.process.scan(
            "8D ? ? ? ? ? 48 89 35 ? ? ? ? 48 89 ? ? C3",
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

        let schema = Schema::new(&self.process, offsets.library.schema)?;
        let client = schema.get_library(cs2::CLIENT_LIB)?;

        offsets.controller.name = client.get("CBasePlayerController", "m_iszPlayerName")?;
        offsets.controller.pawn = client.get("CBasePlayerController", "m_hPawn")?;
        offsets.controller.desired_fov = client.get("CBasePlayerController", "m_iDesiredFOV")?;
        offsets.controller.owner_entity = client.get("C_BaseEntity", "m_hOwnerEntity")?;

        offsets.pawn.health = client.get("C_BaseEntity", "m_iHealth")?;
        offsets.pawn.armor = client.get("C_CSPlayerPawn", "m_ArmorValue")?;
        offsets.pawn.team = client.get("C_BaseEntity", "m_iTeamNum")?;
        offsets.pawn.life_state = client.get("C_BaseEntity", "m_lifeState")?;
        offsets.pawn.weapon = client.get("C_CSPlayerPawnBase", "m_pClippingWeapon")?;
        offsets.pawn.fov_multiplier = client.get("C_BasePlayerPawn", "m_flFOVSensitivityAdjust")?;
        offsets.pawn.game_scene_node = client.get("C_BaseEntity", "m_pGameSceneNode")?;
        offsets.pawn.eye_offset = client.get("C_BaseModelEntity", "m_vecViewOffset")?;
        offsets.pawn.velocity = client.get("C_BaseEntity", "m_vecAbsVelocity")?;
        offsets.pawn.aim_punch_cache = client.get("C_CSPlayerPawn", "m_aimPunchCache")?;
        offsets.pawn.shots_fired = client.get("C_CSPlayerPawn", "m_iShotsFired")?;
        offsets.pawn.view_angles = client.get("C_BasePlayerPawn", "v_angle")?;
        offsets.pawn.spotted_state = client.get("C_CSPlayerPawn", "m_entitySpottedState")?;
        offsets.pawn.crosshair_entity = client.get("C_CSPlayerPawnBase", "m_iIDEntIndex")?;
        offsets.pawn.is_scoped = client.get("C_CSPlayerPawn", "m_bIsScoped")?;
        offsets.pawn.flash_alpha = client.get("C_CSPlayerPawnBase", "m_flFlashMaxAlpha")?;
        offsets.pawn.flash_duration = client.get("C_CSPlayerPawnBase", "m_flFlashDuration")?;

        offsets.pawn.camera_services = client.get("C_BasePlayerPawn", "m_pCameraServices")?;
        offsets.pawn.item_services = client.get("C_BasePlayerPawn", "m_pItemServices")?;
        offsets.pawn.weapon_services = client.get("C_BasePlayerPawn", "m_pWeaponServices")?;

        offsets.game_scene_node.dormant = client.get("CGameSceneNode", "m_bDormant")?;
        offsets.game_scene_node.origin = client.get("CGameSceneNode", "m_vecAbsOrigin")?;
        offsets.game_scene_node.model_state = client.get("CSkeletonInstance", "m_modelState")?;

        offsets.smoke.did_smoke_effect =
            client.get("C_SmokeGrenadeProjectile", "m_bDidSmokeEffect")?;
        offsets.smoke.smoke_color = client.get("C_SmokeGrenadeProjectile", "m_vSmokeColor")?;

        offsets.spotted_state.spotted = client.get("EntitySpottedState_t", "m_bSpotted")?;
        offsets.spotted_state.mask = client.get("EntitySpottedState_t", "m_bSpottedByMask")?;

        offsets.camera_services.fov = client.get("CCSPlayerBase_CameraServices", "m_iFOV")?;

        offsets.item_services.has_defuser =
            client.get("CCSPlayer_ItemServices", "m_bHasDefuser")?;
        offsets.item_services.has_helmet = client.get("CCSPlayer_ItemServices", "m_bHasHelmet")?;

        offsets.weapon_services.weapons = client.get("CPlayer_WeaponServices", "m_hMyWeapons")?;

        offsets.planted_c4.is_activated = client.get("C_PlantedC4", "m_bC4Activated")?;
        offsets.planted_c4.is_ticking = client.get("C_PlantedC4", "m_bBombTicking")?;
        offsets.planted_c4.blow_time = client.get("C_PlantedC4", "m_flC4Blow")?;
        offsets.planted_c4.being_defused = client.get("C_PlantedC4", "m_bBeingDefused")?;

        use offsets::Offset as _;
        if offsets.all_found() {
            debug!("offsets: {:?} ({:?})", offsets, Instant::now() - start);
            return Some(offsets);
        }

        warn!("not all offsets found: {:?}", offsets);
        None
    }

    fn entity_type(&self, entity: u64) -> Option<Entity> {
        let entity_instance: u64 = self.process.read(entity + 0x10);
        if entity_instance == 0 {
            return None;
        }

        let name_pointer = self.process.read(entity_instance + 0x20);
        if name_pointer == 0 {
            return None;
        }

        let name = self.process.read_string(name_pointer);

        if name.starts_with("weapon_") {
            if self.entity_has_owner(entity) {
                return None;
            }
            let position = Player::entity(entity).position(self);
            Some(Entity::Weapon(
                Weapon::from_str(&name.replace("weapon_", "")),
                position,
            ))
        } else if name.starts_with("smoke") {
            Some(Entity::Smoke(Smoke::new(entity)))
        } else {
            None
        }
    }

    fn entity_has_owner(&self, entity: u64) -> bool {
        self.process
            .read::<i32>(entity + self.offsets.controller.owner_entity)
            != -1
    }

    // convars
    fn get_sensitivity(&self) -> f32 {
        self.process.read(self.offsets.convar.sensitivity + 0x48)
    }

    fn is_ffa(&self) -> bool {
        self.process.read::<u32>(self.offsets.convar.ffa + 0x48) == 1
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

    fn cache_entities(&mut self) {
        self.entities.clear();
        for i in 64..=1024 {
            let Some(entity) = Player::get_client_entity(self, i) else {
                continue;
            };

            let Some(entity) = self.entity_type(entity) else {
                continue;
            };

            self.entities.push(entity);
        }
    }
}

#[derive(Debug)]
pub enum Entity {
    Weapon(Weapon, Vec3),
    Smoke(Smoke),
}
