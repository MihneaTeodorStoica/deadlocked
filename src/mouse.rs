use std::{
    fs::{self, File, OpenOptions, read_dir},
    io::Write,
    os::unix::fs::FileTypeExt,
    time::{SystemTime, UNIX_EPOCH},
};

use glam::{IVec2, Vec2};
use log::warn;

#[derive(Clone, Debug, PartialEq)]
pub enum DeviceStatus {
    Working(String),
    Disconnected,
    PermissionsRequired,
    NotFound,
}

#[derive(Debug, Clone, Copy)]
struct Timeval {
    seconds: u64,
    microseconds: u64,
}

#[derive(Debug, Clone, Copy)]
struct InputEvent {
    time: Timeval,
    event_type: u16,
    code: u16,
    value: i32,
}

impl InputEvent {
    fn bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::with_capacity(24);

        bytes.extend(&self.time.seconds.to_le_bytes());
        bytes.extend(&self.time.microseconds.to_le_bytes());

        bytes.extend(&self.event_type.to_le_bytes());
        bytes.extend(&self.code.to_le_bytes());
        bytes.extend(&self.value.to_le_bytes());

        bytes
    }
}

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const SYN_REPORT: u16 = 0x00;
const AXIS_X: u16 = 0x00;
const AXIS_Y: u16 = 0x01;
const BTN_LEFT: u16 = 0x110;

pub struct Mouse {
    file: File,
    pub status: DeviceStatus,
}

impl Mouse {
    pub fn open() -> Self {
        for file in read_dir("/dev/input").unwrap() {
            let entry = file.unwrap();
            if !entry.file_type().unwrap().is_char_device() {
                continue;
            }
            let name = entry.file_name().into_string().unwrap();
            if !name.starts_with("event") {
                continue;
            }
            // get device info from /sys/class/input
            let rel = decode_capabilities(&format!(
                "/sys/class/input/{}/device/capabilities/rel",
                name
            ));

            if !rel[AXIS_X as usize] || !rel[AXIS_Y as usize] {
                continue;
            }

            let device_name =
                fs::read_to_string(format!("/sys/class/input/{}/device/name", name)).unwrap();

            let path = format!("/dev/input/{}", name);
            let file = OpenOptions::new().write(true).open(path);
            match file {
                Ok(file) => {
                    return Self {
                        file,
                        status: DeviceStatus::Working(device_name),
                    };
                }
                Err(_) => {
                    warn!("please add your user to the input group or execute with sudo");
                    warn!(
                        "without this, mouse movements will be written to /dev/null and discarded"
                    );
                    let file = OpenOptions::new().write(true).open("/dev/null").unwrap();
                    return Self {
                        file,
                        status: DeviceStatus::PermissionsRequired,
                    };
                }
            }
        }

        let file = OpenOptions::new().write(true).open("/dev/null").unwrap();
        warn!("no mouse found");
        Self {
            file,
            status: DeviceStatus::NotFound,
        }
    }

    pub fn move_rel(&mut self, coords: &Vec2) {
        let coords = IVec2::new(coords.x as i32, coords.y as i32);

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let time = Timeval {
            seconds: now.as_secs(),
            microseconds: now.subsec_micros() as u64,
        };

        let x = InputEvent {
            time,
            event_type: EV_REL,
            code: AXIS_X,
            value: coords.x,
        };

        let y = InputEvent {
            time,
            event_type: EV_REL,
            code: AXIS_Y,
            value: coords.y,
        };

        let syn = InputEvent {
            time,
            event_type: EV_SYN,
            code: SYN_REPORT,
            value: 0,
        };

        self.file.write_all(&x.bytes()).unwrap();
        self.file.write_all(&syn.bytes()).unwrap();

        self.file.write_all(&y.bytes()).unwrap();
        self.file.write_all(&syn.bytes()).unwrap();
    }

    pub fn left_press(&mut self) {
        self.key(1);
    }

    pub fn left_release(&mut self) {
        self.key(0);
    }

    fn key(&mut self, pressed: i32) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let time = Timeval {
            seconds: now.as_secs(),
            microseconds: now.subsec_micros() as u64,
        };

        let press = InputEvent {
            time,
            event_type: EV_KEY,
            code: BTN_LEFT,
            value: pressed,
        };

        let syn = InputEvent {
            time,
            event_type: EV_SYN,
            code: SYN_REPORT,
            value: 0,
        };

        self.file.write_all(&press.bytes()).unwrap();
        self.file.write_all(&syn.bytes()).unwrap();
    }

    pub fn is_valid(&mut self) -> bool {
        self.file.write_all(&SYN.bytes()).is_ok()
    }
}

const SYN: InputEvent = InputEvent {
    time: Timeval {
        seconds: 0,
        microseconds: 0,
    },
    event_type: EV_SYN,
    code: SYN_REPORT,
    value: 0,
};

fn hex_to_reversed_binary(hex_char: char) -> Vec<bool> {
    let value = match hex_char {
        '0'..='9' => hex_char as u8 - b'0',
        'a'..='f' => hex_char as u8 - b'a' + 10,
        'A'..='F' => hex_char as u8 - b'A' + 10,
        _ => 0,
    };
    (0..4).map(|i| (value >> i) & 1 == 1).collect()
}

pub fn decode_capabilities(filename: &str) -> Vec<bool> {
    let Ok(content) = std::fs::read_to_string(filename) else {
        return Vec::new();
    };

    let mut binary_out = Vec::new();
    let mut hex_count = 0;

    // line has to be processed in reverse (why?)
    for c in content.chars().rev().filter(|&c| c != '\n') {
        if c == ' ' {
            binary_out.extend(std::iter::repeat_n(false, 4 * (16 - hex_count)));
            hex_count = 0;
        } else if c.is_ascii_hexdigit() {
            binary_out.extend(hex_to_reversed_binary(c));
            hex_count += 1;
        }
    }

    // pad final group if incomplete
    if (1..16).contains(&hex_count) {
        binary_out.extend(std::iter::repeat_n(false, 4 * (16 - hex_count)));
    }

    binary_out
}
