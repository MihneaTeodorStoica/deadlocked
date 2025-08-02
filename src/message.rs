use crate::{
    config::{GameStatus, Config},
    mouse::DeviceStatus,
};

#[derive(Clone, Debug)]
pub enum Message {
    Config(Config),
    Status(GameStatus),
    MouseStatus(DeviceStatus),
}
