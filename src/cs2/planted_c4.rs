use glam::Vec3;

use crate::cs2::{CS2, player::Player};

#[derive(Debug)]
pub enum BombSite {
    A,
    B,
}

impl BombSite {
    pub fn from_int(site: i32) -> Option<Self> {
        match site {
            0 => Some(Self::A),
            1 => Some(Self::B),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlantedC4 {
    handle: u64,
}

impl PlantedC4 {
    pub fn get(cs2: &CS2) -> Option<Self> {
        let handle = cs2.process.read(cs2.offsets.direct.planted_c4);
        if handle == 0 {
            return None;
        };

        let handle = cs2.process.read(handle);
        if handle == 0 {
            return None;
        };

        Some(Self { handle })
    }

    pub fn is_planted(&self, cs2: &CS2) -> bool {
        cs2.process
            .read::<u8>(self.handle + cs2.offsets.planted_c4.is_activated)
            != 0
            && cs2
                .process
                .read::<u8>(self.handle + cs2.offsets.planted_c4.is_ticking)
                != 0
    }

    pub fn is_being_defused(&self, cs2: &CS2) -> bool {
        cs2.process
            .read::<u8>(self.handle + cs2.offsets.planted_c4.being_defused)
            != 0
    }

    pub fn bomb_site(&self, cs2: &CS2) -> Option<BombSite> {
        let site = cs2
            .process
            .read(self.handle + cs2.offsets.planted_c4.bomb_site);
        BombSite::from_int(site)
    }

    pub fn time_to_explosion(&self, cs2: &CS2) -> f32 {
        let global_vars: u64 = cs2.process.read(cs2.offsets.direct.global_vars);
        let current_time: f32 = cs2.process.read(global_vars + 52);
        cs2.process
            .read::<f32>(self.handle + cs2.offsets.planted_c4.blow_time)
            - current_time
    }

    pub fn position(&self, cs2: &CS2) -> Vec3 {
        let planted_c4 = Player::pawn(self.handle);
        planted_c4.position(cs2)
    }
}
