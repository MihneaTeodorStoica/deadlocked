use std::collections::HashMap;

use glam::{Vec2, Vec3};

use crate::{
    constants::cs2,
    cs2::{bones::Bones, weapon::Weapon},
};

use super::{CS2, weapon_class::WeaponClass};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Player {
    controller: u64,
    pawn: u64,
}

impl Player {
    pub fn index(cs2: &CS2, index: u64) -> Option<Self> {
        let controller = Self::get_client_entity(cs2, index)?;
        Self::get_pawn(cs2, controller).map(|pawn| Self { controller, pawn })
    }

    pub fn local_player(cs2: &CS2) -> Option<Self> {
        let controller = cs2.process.read(cs2.offsets.direct.local_player);
        if controller == 0 {
            return None;
        }
        Self::get_pawn(cs2, controller).map(|pawn| Self { controller, pawn })
    }

    pub fn pawn(pawn: u64) -> Self {
        Self {
            controller: 0,
            pawn,
        }
    }

    pub fn get_client_entity(cs2: &CS2, index: u64) -> Option<u64> {
        // wtf is this doing, and how?
        let v1: u64 = cs2
            .process
            .read(cs2.offsets.interface.entity + 0x08 * (index >> 9) + 0x10);
        if v1 == 0 {
            return None;
        }
        // what?
        let entity = cs2.process.read(v1 + 120 * (index & 0x1ff));
        if entity == 0 {
            return None;
        }
        Some(entity)
    }

    fn get_pawn(cs2: &CS2, controller: u64) -> Option<u64> {
        let v1: i32 = cs2.process.read(controller + cs2.offsets.controller.pawn);
        if v1 == -1 {
            return None;
        }

        // what the fuck is this doing?
        let v2: u64 = cs2
            .process
            .read(cs2.offsets.interface.player + 8 * ((v1 as u64 & 0x7fff) >> 9));
        if v2 == 0 {
            return None;
        }

        // bit-fuckery, why is this needed exactly?
        let entity = cs2.process.read(v2 + 120 * (v1 as u64 & 0x1ff));
        if entity == 0 {
            return None;
        }
        Some(entity)
    }

    pub fn health(&self, cs2: &CS2) -> i32 {
        let health = cs2.process.read(self.pawn + cs2.offsets.pawn.health);
        if !(0..=100).contains(&health) {
            return 0;
        }
        health
    }

    pub fn armor(&self, cs2: &CS2) -> i32 {
        cs2.process.read(self.pawn + cs2.offsets.pawn.armor)
    }

    pub fn team(&self, cs2: &CS2) -> u8 {
        cs2.process.read(self.pawn + cs2.offsets.pawn.team)
    }

    pub fn life_state(&self, cs2: &CS2) -> u8 {
        cs2.process.read(self.pawn + cs2.offsets.pawn.life_state)
    }

    pub fn name(&self, cs2: &CS2) -> String {
        cs2.process
            .read_string(self.controller + cs2.offsets.controller.name)
    }

    pub fn weapon_name(&self, cs2: &CS2) -> String {
        // CEntityInstance
        let weapon_entity_instance: u64 = cs2.process.read(self.pawn + cs2.offsets.pawn.weapon);
        if weapon_entity_instance == 0 {
            return String::from(cs2::WEAPON_UNKNOWN);
        }
        // CEntityIdentity, 0x10 = m_pEntity
        let weapon_entity_identity: u64 = cs2.process.read(weapon_entity_instance + 0x10);
        if weapon_entity_identity == 0 {
            return String::from(cs2::WEAPON_UNKNOWN);
        }
        // 0x20 = m_designerName (pointer -> string)
        let weapon_name_pointer = cs2.process.read(weapon_entity_identity + 0x20);
        if weapon_name_pointer == 0 {
            return String::from(cs2::WEAPON_UNKNOWN);
        }
        let name = cs2.process.read_string(weapon_name_pointer);
        name.replace("weapon_", "")
    }

    pub fn weapon_class(&self, cs2: &CS2) -> WeaponClass {
        WeaponClass::from_string(&self.weapon_name(cs2))
    }

    pub fn weapon(&self, cs2: &CS2) -> Weapon {
        Weapon::from_str(&self.weapon_name(cs2))
    }

    pub fn all_weapons(&self, cs2: &CS2) -> Vec<Weapon> {
        let mut weapons = vec![];
        let weapon_services: u64 = cs2
            .process
            .read(self.pawn + cs2.offsets.pawn.weapon_services);
        if weapon_services == 0 {
            return weapons;
        }

        let length: i32 = cs2
            .process
            .read(weapon_services + cs2.offsets.weapon_services.weapons);
        let weapon_list: u64 = cs2
            .process
            .read(weapon_services + cs2.offsets.weapon_services.weapons + 0x08);

        for i in 0..length as u64 {
            let weapon_index = cs2.process.read::<i32>(weapon_list + 0x04 * i) as u64 & 0xFFF;
            // CEntityInstance
            let Some(weapon_entity_instance) = Self::get_client_entity(cs2, weapon_index) else {
                continue;
            };
            if weapon_entity_instance == 0 {
                continue;
            }
            // CEntityIdentity, 0x10 = m_pEntity
            let weapon_entity_identity: u64 = cs2.process.read(weapon_entity_instance + 0x10);
            if weapon_entity_identity == 0 {
                continue;
            }
            // 0x20 = m_designerName (pointer -> string)
            let weapon_name_pointer = cs2.process.read(weapon_entity_identity + 0x20);
            if weapon_name_pointer == 0 {
                continue;
            }
            let name = cs2.process.read_string(weapon_name_pointer);
            let name = name.replace("weapon_", "");
            weapons.push(Weapon::from_str(name.as_str()))
        }

        weapons
    }

    fn game_scene_node(&self, cs2: &CS2) -> u64 {
        cs2.process
            .read(self.pawn + cs2.offsets.pawn.game_scene_node)
    }

    fn is_dormant(&self, cs2: &CS2) -> bool {
        let gs_node = self.game_scene_node(cs2);
        cs2.process
            .read::<u8>(gs_node + cs2.offsets.game_scene_node.dormant)
            != 0
    }

    pub fn position(&self, cs2: &CS2) -> Vec3 {
        let gs_node = self.game_scene_node(cs2);
        cs2.process
            .read(gs_node + cs2.offsets.game_scene_node.origin)
    }

    pub fn eye_position(&self, cs2: &CS2) -> Vec3 {
        let position = self.position(cs2);
        let eye_offset: Vec3 = cs2.process.read(self.pawn + cs2.offsets.pawn.eye_offset);

        position + eye_offset
    }

    pub fn bone_position(&self, cs2: &CS2, bone_index: u64) -> Vec3 {
        let gs_node = self.game_scene_node(cs2);
        let bone_data: u64 = cs2
            .process
            .read(gs_node + cs2.offsets.game_scene_node.model_state + 0x80);

        if bone_data == 0 {
            return Vec3::ZERO;
        }

        cs2.process.read(bone_data + (bone_index * 32))
    }

    pub fn all_bones(&self, cs2: &CS2) -> HashMap<Bones, Vec3> {
        use strum::IntoEnumIterator as _;

        let mut bones = HashMap::new();
        for bone in Bones::iter() {
            let pos = self.bone_position(cs2, bone.u64());
            bones.insert(bone, pos);
        }
        bones
    }

    pub fn shots_fired(&self, cs2: &CS2) -> i32 {
        cs2.process.read(self.pawn + cs2.offsets.pawn.shots_fired)
    }

    pub fn fov_multiplier(&self, cs2: &CS2) -> f32 {
        cs2.process
            .read(self.pawn + cs2.offsets.pawn.fov_multiplier)
    }

    pub fn spotted_mask(&self, cs2: &CS2) -> i64 {
        cs2.process
            .read(self.pawn + cs2.offsets.pawn.spotted_state + cs2.offsets.spotted_state.mask)
    }

    pub fn is_valid(&self, cs2: &CS2) -> bool {
        if self.is_dormant(cs2) {
            return false;
        }

        if self.health(cs2) <= 0 {
            return false;
        }

        if self.life_state(cs2) != 0 {
            return false;
        }

        true
    }

    pub fn is_flashed(&self, cs2: &CS2) -> bool {
        cs2.process
            .read::<f32>(self.pawn + cs2.offsets.pawn.flash_duration)
            > 0.2
    }

    pub fn view_angles(&self, cs2: &CS2) -> Vec2 {
        cs2.process.read(self.pawn + cs2.offsets.pawn.view_angles)
    }

    pub fn aim_punch(&self, cs2: &CS2) -> Vec2 {
        let length: u64 = cs2
            .process
            .read(self.pawn + cs2.offsets.pawn.aim_punch_cache);
        if length < 1 {
            return Vec2::ZERO;
        }

        let data_address: u64 = cs2
            .process
            .read(self.pawn + cs2.offsets.pawn.aim_punch_cache + 0x08);

        cs2.process.read(data_address + (length - 1) * 12)
    }

    pub fn has_defuser(&self, cs2: &CS2) -> bool {
        let item_services: u64 = cs2.process.read(self.pawn + cs2.offsets.pawn.item_services);
        if item_services == 0 {
            return false;
        }

        cs2.process
            .read::<u8>(item_services + cs2.offsets.item_services.has_defuser)
            != 0
    }

    pub fn has_helmet(&self, cs2: &CS2) -> bool {
        let item_services: u64 = cs2.process.read(self.pawn + cs2.offsets.pawn.item_services);
        if item_services == 0 {
            return false;
        }

        cs2.process
            .read::<u8>(item_services + cs2.offsets.item_services.has_helmet)
            != 0
    }

    pub fn has_bomb(&self, cs2: &CS2) -> bool {
        let weapons = self.all_weapons(cs2);
        weapons.contains(&Weapon::C4)
    }

    #[allow(unused)]
    pub fn velocity(&self, cs2: &CS2) -> Vec3 {
        cs2.process.read(self.pawn + cs2.offsets.pawn.velocity)
    }

    pub fn no_flash(&self, cs2: &CS2, flash_alpha: f32) {
        let flash_alpha = flash_alpha.clamp(0.0, 1.0);
        if cs2
            .process
            .read::<f32>(self.pawn + cs2.offsets.pawn.flash_alpha)
            != flash_alpha
        {
            cs2.process
                .write(self.pawn + cs2.offsets.pawn.flash_alpha, flash_alpha);
        }
    }

    pub fn set_fov(&self, cs2: &CS2, value: u32) {
        let camera_service = cs2
            .process
            .read::<u64>(self.pawn + cs2.offsets.pawn.camera_services);
        if camera_service == 0 {
            return;
        }
        if cs2
            .process
            .read::<u32>(camera_service + cs2.offsets.camera_services.fov)
            != value
        {
            cs2.process
                .write(self.controller + cs2.offsets.controller.desired_fov, value);
        }
    }
}

impl CS2 {
    pub fn cache_players(&mut self) {
        if !self.process.is_valid() {
            self.players.clear();
            return;
        };

        let Some(local_player) = Player::local_player(self) else {
            return;
        };

        self.players.clear();
        for i in 0..=64 {
            let player = match Player::index(self, i) {
                Some(player) => player,
                None => continue,
            };

            if !player.is_valid(self) {
                continue;
            }

            if player == local_player {
                self.target.local_pawn_index = i - 1;
            } else {
                self.players.push(player);
            }
        }
    }
}
