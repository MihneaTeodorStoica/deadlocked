use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

use bytemuck::AnyBitPattern;
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use sha2::Digest;
use ureq::Agent;

use crate::{
    bvh::{Bvh, Triangle},
    config::exe_path,
};

const RELEASE_URL: &str =
    "https://api.github.com/repos/ValveResourceFormat/ValveResourceFormat/releases/latest";
const ASSET_NAME: &str = "cli-linux-x64.zip";

pub fn parse_maps(bvh: Arc<Mutex<HashMap<String, Bvh>>>) {
    let client = ureq::Agent::new_with_defaults();

    let release: serde_json::Value = client
        .get(RELEASE_URL)
        .call()
        .expect("failed to fetch release metadata")
        .body_mut()
        .read_json()
        .expect("failed to extract json from release metadata");

    let assets = release["assets"]
        .as_array()
        .expect("invalid asset metadata format");
    let asset = assets
        .iter()
        .find(|&a| a["name"].as_str().unwrap_or_default() == ASSET_NAME)
        .expect("could not find source 2 viewer linux cli");

    let checksum = asset["digest"]
        .as_str()
        .expect("could not find asset hash")
        .replace("sha256:", "");

    let file_path = exe_path().join(ASSET_NAME);
    let dir = file_path.parent().unwrap().join("source2viewer");
    let exe_path = dir.join("Source2Viewer-CLI");

    if needs_updating(&file_path, &checksum) {
        let url = asset["browser_download_url"]
            .as_str()
            .expect("could not find asset download url");
        download_s2v(&client, url, &file_path);
        unzip_s2v(&file_path, &dir);
    }
    if !dir.exists() {
        unzip_s2v(&file_path, &dir);
    }

    let home = std::env::var("HOME").expect("could not find home directory");
    let steam_path = PathBuf::from(home).join(".steam/steam");
    if !steam_path.exists() {
        log::warn!("could not locate steam directory");
        return;
    }

    let library_folders = steam_path.join("config/libraryfolders.vdf");
    let content =
        std::fs::read_to_string(&library_folders).expect("could not read steam library folders");
    let libs: Vec<&str> = content
        .lines()
        .filter_map(|line| {
            if line.contains("\"path\"") {
                Some(line.rsplit('"').nth(1).unwrap())
            } else {
                None
            }
        })
        .collect();

    let game_dir = libs
        .iter()
        .find(|&&lib| {
            let dir = PathBuf::from(lib).join("steamapps/common/Counter-Strike Global Offensive");
            dir.exists()
        })
        .expect("could not find game directory");
    let maps_dir = PathBuf::from(game_dir)
        .join("steamapps/common/Counter-Strike Global Offensive/game/csgo/maps");

    let mut files = Vec::with_capacity(16);
    for file in std::fs::read_dir(&maps_dir).expect("could not read cs2 maps dir") {
        let Ok(file) = file else {
            continue;
        };

        if !file.file_type().unwrap().is_file() {
            continue;
        }

        let file_name = file.file_name();
        let file_name = file_name.to_str().unwrap();
        if file_name.contains("_vanity") {
            continue;
        }

        if !file_name.starts_with("ar_")
            && !file_name.starts_with("cs_")
            && !file_name.starts_with("de_")
        {
            continue;
        }

        if !file_name.ends_with(".vpk") {
            continue;
        }

        files.push(file_name.to_string());
    }

    for file in &files {
        let path = maps_dir.join(file);
        let map_name = file.replace(".vpk", "");

        if maps_dir.join("geometry/maps").join(&map_name).exists() {
            continue;
        }

        Command::new(exe_path.as_os_str())
            .args([
                "-i",
                path.to_str().unwrap(),
                "-d",
                "-o",
                maps_dir.join("geometry").to_str().unwrap(),
                "-f",
                &format!("maps/{map_name}/world_physics.vmdl_c"),
            ])
            .output()
            .unwrap();
    }

    for map in &files {
        let map_name = map.replace(".vpk", "");
        let bvh_name = format!("{map_name}.bvh");
        let bvh_path = maps_dir.join(bvh_name);

        if bvh_path.exists() {
            let mut bvh_file = File::open(&bvh_path).unwrap();
            if let Some(map_bvh) = Bvh::load(&mut bvh_file) {
                log::debug!("loaded bvh for {map_name}");
                bvh.lock().unwrap().insert(map_name, map_bvh);
                continue;
            }
        }

        let geom_dir = maps_dir.join("geometry/maps").join(&map_name);

        let mut map_bvh = Bvh::new();
        for file in std::fs::read_dir(&geom_dir).unwrap() {
            let Ok(file) = file else {
                continue;
            };
            let file_name = file.file_name();
            let file_name = file_name.to_str().unwrap();
            let file_type = if file_name.contains("world_physics_hull") {
                FileType::Hull
            } else if file_name.contains("world_physics_phys") {
                FileType::Phys
            } else {
                continue;
            };
            let file = File::open(file.path()).unwrap();
            let mut reader = BufReader::new(file);
            let elements = parse_dmx(&mut reader);
            let vertex_element = elements.get("DmeVertexData_bind").unwrap();
            let Some(Attribute::Vec3Array(vertices)) = vertex_element.attributes.get("position$0")
            else {
                continue;
            };
            let vertex_indices: Vec<&[i32]> = if file_type == FileType::Hull {
                let Some(face_element) = elements.get("DmeFaceSet_hull faces") else {
                    continue;
                };
                let Some(Attribute::IntegerArray(indices)) = face_element.attributes.get("faces")
                else {
                    continue;
                };
                indices.split(|i| *i == -1).collect()
            } else {
                let Some(Attribute::IntegerArray(indices)) =
                    vertex_element.attributes.get("position$0Indices")
                else {
                    continue;
                };
                indices.chunks_exact(3).collect()
            };

            for face in vertex_indices {
                if face.len() < 3 || face.iter().any(|index| *index as usize >= vertices.len()) {
                    continue;
                } else if face.len() == 3 {
                    let v1 = vertices[face[0] as usize];
                    let v2 = vertices[face[1] as usize];
                    let v3 = vertices[face[2] as usize];
                    let triangle = Triangle::new(v1, v2, v3);
                    map_bvh.insert(triangle);
                } else {
                    for i in 1..face.len() - 1 {
                        let v1 = vertices[face[0] as usize];
                        let v2 = vertices[face[i] as usize];
                        let v3 = vertices[face[i + 1] as usize];
                        let triangle = Triangle::new(v1, v2, v3);
                        map_bvh.insert(triangle);
                    }
                }
            }
        }
        map_bvh.build();
        let mut bvh_file = File::create(&bvh_path).unwrap();
        map_bvh.save(&mut bvh_file);
        log::info!("parsed bvh for {map_name}");
        bvh.lock().unwrap().insert(map_name, map_bvh);
    }
    log::info!("loaded map info");
}

#[derive(PartialEq)]
enum FileType {
    Hull,
    Phys,
}

fn read_element(reader: &mut impl Read, strings: &[String]) -> Element {
    let kind = &strings[read::<i32>(reader) as usize];
    let name = &strings[read::<i32>(reader) as usize];
    let _uuid = read_bytes(reader, 16);
    Element::new(kind.to_string(), name.to_string())
}

fn parse_dmx(reader: &mut impl Read) -> HashMap<String, Element> {
    let _header = read_string(reader);
    let _prefix_elements: i32 = read(reader);
    let string_count: i32 = read(reader);
    let mut strings = Vec::with_capacity(string_count as usize);
    for _ in 0..string_count {
        strings.push(read_string(reader));
    }

    let element_count: i32 = read(reader);
    let mut elements = Vec::with_capacity(element_count as usize);
    for _ in 0..element_count {
        let element = read_element(reader, &strings);
        elements.push(element);
    }

    for element in &mut elements {
        let attribute_count: i32 = read(reader);
        for _ in 0..attribute_count {
            let name = &strings[read::<i32>(reader) as usize];
            let kind: u8 = read(reader);
            use Attribute as AT;
            let value = match kind {
                1 => AT::Element({
                    let index: i32 = read(reader);
                    if index == -1 {
                        None
                    } else if index == -2 {
                        panic!();
                    } else {
                        Some(index)
                    }
                }),
                2 => AT::Integer(read(reader)),
                3 => AT::Float(read(reader)),
                4 => AT::Bool(read::<u8>(reader) != 0),
                5 => AT::String(strings[read::<i32>(reader) as usize].clone()),
                6 => AT::ByteArray({
                    let count: i32 = read(reader);
                    read_bytes(reader, count as usize)
                }),
                7 => AT::TimeSpan(read(reader)),
                8 => AT::Color(read(reader)),
                9 => AT::Vec2(read(reader)),
                10 => AT::Vec3(read(reader)),
                11 => AT::Angle(read(reader)),
                12 => AT::Vec4(read(reader)),
                13 => AT::Quaternion(read(reader)),
                14 => AT::Matrix(read(reader)),
                15 => AT::Byte(read(reader)),
                16 => AT::U64(read(reader)),

                33 => AT::ElementArray({
                    let count: i32 = read(reader);
                    (0..count)
                        .map(|_| {
                            let idx: i32 = read(reader);
                            match idx {
                                -1 => None,
                                -2 => panic!("Invalid Element index in array"),
                                x => Some(x),
                            }
                        })
                        .collect()
                }),
                34 => AT::IntegerArray({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read(reader)).collect()
                }),
                35 => AT::FloatArray({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read(reader)).collect()
                }),
                36 => AT::BoolArray({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read::<u8>(reader) != 0).collect()
                }),
                37 => AT::StringArray({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read_string(reader)).collect()
                }),
                38 => panic!(),
                39 => AT::TimeSpanArray({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read(reader)).collect()
                }),
                40 => AT::ColorArray({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read(reader)).collect()
                }),
                41 => AT::Vec2Array({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read(reader)).collect()
                }),
                42 => AT::Vec3Array({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read(reader)).collect()
                }),
                43 => AT::AngleArray({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read(reader)).collect()
                }),
                44 => AT::Vec4Array({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read(reader)).collect()
                }),

                45 => AT::QuaternionArray({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read(reader)).collect()
                }),
                46 => AT::MatrixArray({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read(reader)).collect()
                }),
                47 => AT::ByteArray({
                    let count: i32 = read(reader);
                    read_bytes(reader, count as usize)
                }),
                48 => AT::U64Array({
                    let count: i32 = read(reader);
                    (0..count).map(|_| read(reader)).collect()
                }),

                _ => panic!(),
            };
            element.add(name.to_string(), value);
        }
    }
    let mut elems = HashMap::new();
    elements.into_iter().for_each(|e| {
        let name = format!("{}_{}", e.kind, e.name);
        elems.insert(name, e);
    });

    elems
}

#[derive(Debug, Clone)]
struct Element {
    kind: String,
    name: String,
    attributes: HashMap<String, Attribute>,
}

impl Element {
    pub fn new(kind: String, name: String) -> Self {
        Self {
            kind,
            name,
            attributes: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: String, attribute: Attribute) {
        self.attributes.insert(name, attribute);
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
enum Attribute {
    // element index
    Element(Option<i32>),
    Integer(i32),
    Float(f32),
    Bool(bool),
    String(String),
    ByteArray(Vec<u8>),
    TimeSpan(i32),
    Color(u32),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
    Angle(Vec3),
    Quaternion(Quat),
    Matrix(Mat4),
    U64(u64),
    Byte(u8),

    ElementArray(Vec<Option<i32>>),
    IntegerArray(Vec<i32>),
    FloatArray(Vec<f32>),
    BoolArray(Vec<bool>),
    StringArray(Vec<String>),
    TimeSpanArray(Vec<i32>),
    ColorArray(Vec<u32>),
    Vec2Array(Vec<Vec2>),
    Vec3Array(Vec<Vec3>),
    Vec4Array(Vec<Vec4>),
    AngleArray(Vec<Vec3>),
    QuaternionArray(Vec<Quat>),
    MatrixArray(Vec<Mat4>),
    U64Array(Vec<u64>),
}

fn needs_updating(file_path: &Path, checksum: &str) -> bool {
    if !file_path.exists() {
        return true;
    }

    let mut file = File::open(file_path).expect("could not open source 2 viewer file");
    let mut hasher = sha2::Sha256::new();
    io::copy(&mut file, &mut hasher).unwrap();
    let file_checksum = hasher.finalize();
    let file_checksum = format!("{file_checksum:x}");

    file_checksum != checksum
}

fn download_s2v(client: &Agent, url: &str, file_path: &Path) {
    let mut res = client
        .get(url)
        .call()
        .expect("could not fetch source 2 viewer cli");

    let mut file = File::create(file_path).expect("could not create source 2 viewer cli");
    io::copy(&mut res.body_mut().as_reader(), &mut file).unwrap();
    log::info!("downloaded source 2 viewer cli");
}

fn unzip_s2v(file_path: &Path, dir: &Path) {
    Command::new("unzip")
        .args([file_path.to_str().unwrap(), "-d", dir.to_str().unwrap()])
        .output()
        .expect("could not unzip source 2 viewer cli");
    log::info!("unzipped source 2 viewer cli");
}

fn read<T: AnyBitPattern + Default>(reader: &mut impl Read) -> T {
    let mut buffer = vec![0u8; std::mem::size_of::<T>()];
    reader.read_exact(&mut buffer).unwrap();
    *bytemuck::from_bytes(&buffer)
}

fn read_string(reader: &mut impl Read) -> String {
    let mut buffer = Vec::with_capacity(8);
    let mut byte = [0u8; 1];

    loop {
        reader.read_exact(&mut byte).unwrap();
        if byte[0] == 0 {
            break;
        }
        buffer.push(byte[0]);
    }
    String::from_utf8(buffer).unwrap()
}

fn read_bytes(reader: &mut impl Read, count: usize) -> Vec<u8> {
    let mut buf = vec![0u8; count];
    reader.read_exact(&mut buf).unwrap();
    buf
}
