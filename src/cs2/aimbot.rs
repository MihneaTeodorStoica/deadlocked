use glam::vec2;

use crate::{
    config::Config,
    math::{angles_to_fov, vec2_clamp},
    mouse::Mouse,
};

use super::{CS2, bones::Bones, player::Player};

impl CS2 {
    pub fn aimbot(&mut self, config: &Config, mouse: &mut Mouse) {
        let config = self.aimbot_config(config);

        if !config.enabled || self.target.player.is_none() {
            return;
        }
        let target = self.target.player.as_ref().unwrap();

        let Some(local_player) = Player::local_player(self) else {
            return;
        };

        if config.flash_check && local_player.is_flashed(self) {
            return;
        }

        if config.visibility_check {
            let map_name = self.current_map();
            let bvh_map = self.bvh.lock().unwrap();
            if let Some(bvh) = bvh_map.get(&map_name) {
                let eye_pos = local_player.eye_position(self);
                if !bvh.has_line_of_sight(eye_pos, target.eye_position(self))
                    && !bvh.has_line_of_sight(eye_pos, target.position(self))
                {
                    dbg!("no line of sight");
                    return;
                }
                dbg!("line of sight");
            } else {
                let spotted_mask = target.spotted_mask(self);
                if (spotted_mask & (1 << self.target.local_pawn_index)) == 0 {
                    return;
                }
            }
        }

        let target_angle = if config.multibone {
            self.target.angle
        } else {
            let head_position = target.bone_position(self, Bones::Head.u64());
            self.angle_to_target(
                &local_player,
                &head_position,
                &self.target.previous_aim_punch,
            )
        };

        let view_angles = local_player.view_angles(self);
        if angles_to_fov(&view_angles, &target_angle)
            > (config.fov * self.distance_scale(self.target.distance))
        {
            return;
        }

        if !target.is_valid(self) {
            return;
        }

        if local_player.shots_fired(self) < config.start_bullet {
            return;
        }

        let mut aim_angles = view_angles - target_angle;
        if aim_angles.y < -180.0 {
            aim_angles.y += 360.0
        }
        vec2_clamp(&mut aim_angles);

        let sensitivity = self.get_sensitivity() * local_player.fov_multiplier(self);

        let mouse_angles = vec2(
            aim_angles.y / sensitivity * 50.0,
            -aim_angles.x / sensitivity * 50.0,
        ) / (config.smooth + 1.0).clamp(1.0, 10.0);

        mouse.move_rel(&mouse_angles);
    }
}
