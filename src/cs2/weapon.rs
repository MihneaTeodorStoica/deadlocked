use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter, EnumString};

#[derive(
    Debug,
    Default,
    Clone,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    AsRefStr,
    EnumIter,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Weapon {
    // Pistols
    Cz75A,
    Deagle,
    Elite,
    FiveSeven,
    Glock,
    Hkp2000,
    P250,
    Revolver,
    Tec9,
    UspSilencer,
    UspSilencerOff,

    // SMGs
    Bizon,
    Mac10,
    Mp5Sd,
    Mp7,
    Mp9,
    P90,
    Ump45,

    // LMGs
    M249,
    Negev,

    // Shotguns
    Mag7,
    Nova,
    Sawedoff,
    Xm1014,

    // Rifles
    #[default]
    Ak47,
    Aug,
    Famas,
    Galilar,
    M4A1,
    M4A1Silencer,
    M4A1SilencerOff,
    Sg556,

    // Snipers
    Awp,
    G3SG1,
    Scar20,
    Ssg08,

    // Utility
    Taser,
}
