use std::collections::HashMap;

use glam::{IVec2, Mat4, Vec3};

use crate::cs2::{bones::Bones, weapon::Weapon};

#[derive(Debug, Default)]
pub struct Data {
    pub in_game: bool,
    pub is_ffa: bool,
    pub weapon: Weapon,
    pub players: Vec<PlayerData>,
    pub view_matrix: Mat4,
    pub window_position: IVec2,
    pub window_size: IVec2,
}

#[derive(Debug, Default)]
pub struct PlayerData {
    pub health: i32,
    pub armor: i32,
    pub team: u8,
    pub position: Vec3,
    pub head: Vec3,
    pub name: String,
    pub weapon: Weapon,
    pub bones: HashMap<Bones, Vec3>,
}

#[derive(Debug, Default)]
pub struct BombData {
    pub planted: bool,
    pub timer: f32,
    pub being_defused: bool,
}
