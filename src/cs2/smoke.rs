use egui::{Color32, Rgba};

use crate::cs2::{CS2, player::Player};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Smoke {
    controller: u64,
}

impl Smoke {
    pub fn index(cs2: &CS2, index: u64) -> Option<Self> {
        let controller = Player::get_client_entity(cs2, index)?;
        Some(Self { controller })
    }

    pub fn disable(&self, cs2: &CS2) {
        cs2.process
            .write(self.controller + cs2.offsets.smoke.did_smoke_effect, true);
    }

    pub fn color(&self, cs2: &CS2, color: &Color32) {
        let offset = self.controller + cs2.offsets.smoke.smoke_color;
        let color = Rgba::from(*color);
        cs2.process.write(offset, color.r());
        cs2.process.write(offset + 0x04, color.g());
        cs2.process.write(offset + 0x08, color.b());
    }
}
