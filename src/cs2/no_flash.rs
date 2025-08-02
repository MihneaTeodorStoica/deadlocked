use crate::config::Config;

use super::{CS2, player::Player};

impl CS2 {
    pub fn no_flash(&self, config: &Config) {
        let Some(local_player) = Player::local_player(self) else {
            return;
        };

        if config.misc.no_flash {
            local_player.no_flash(self, config.misc.max_flash_alpha);
        } else {
            local_player.no_flash(self, 255.0);
        }
    }
}
