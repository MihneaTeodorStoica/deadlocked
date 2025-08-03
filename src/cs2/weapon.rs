use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter};

#[derive(
    Debug, Default, Clone, PartialEq, Eq, Hash, AsRefStr, EnumIter, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Weapon {
    #[default]
    Unknown,

    Knife,

    // Pistols
    Cz75A,
    Deagle,
    DualBerettas,
    FiveSeven,
    Glock,
    P2000,
    P250,
    Revolver,
    Tec9,
    Usp,

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
    Ak47,
    Aug,
    Famas,
    Galilar,
    M4A4,
    M4A1,
    Sg556,

    // Snipers
    Awp,
    G3SG1,
    Scar20,
    Ssg08,

    // Utility
    Taser,

    // Grenades
    Flashbang,
    HeGrenade,
    Smoke,
    Molotov,
    Decoy,
    Incendiary,

    // Bomb
    C4,
}

impl Weapon {
    pub fn from_str(name: &str) -> Self {
        use Weapon::*;
        match name {
            "bayonet" => Knife,
            "knife" => Knife,
            "knife_bowie" => Knife,
            "knife_butterfly" => Knife,
            "knife_canis" => Knife,
            "knife_cord" => Knife,
            "knife_css" => Knife,
            "knife_falchion" => Knife,
            "knife_flip" => Knife,
            "knife_gut" => Knife,
            "knife_gypsy_jackknife" => Knife,
            "knife_karambit" => Knife,
            "knife_kukri" => Knife,
            "knife_m9_bayonet" => Knife,
            "knife_outdoor" => Knife,
            "knife_push" => Knife,
            "knife_skeleton" => Knife,
            "knife_stiletto" => Knife,
            "knife_survival_bowie" => Knife,
            "knife_t" => Knife,
            "knife_tactical" => Knife,
            "knife_twinblade" => Knife,
            "knife_ursus" => Knife,
            "knife_widowmaker" => Knife,

            "cz75a" => Cz75A,
            "deagle" => Deagle,
            "elite" => DualBerettas,
            "fiveseven" => FiveSeven,
            "glock" => Glock,
            "hkp2000" => P2000,
            "p250" => P250,
            "revolver" => Revolver,
            "tec9" => Tec9,
            "usp_silencer" => Usp,
            "usp_silencer_off" => Usp,

            "bizon" => Bizon,
            "mac10" => Mac10,
            "mp5sd" => Mp5Sd,
            "mp7" => Mp7,
            "mp9" => Mp9,
            "p90" => P90,
            "ump45" => Ump45,

            "m249" => M249,
            "negev" => Negev,

            "mag7" => Mag7,
            "nova" => Nova,
            "sawedoff" => Sawedoff,
            "xm1014" => Xm1014,

            "ak47" => Ak47,
            "aug" => Aug,
            "famas" => Famas,
            "galilar" => Galilar,
            "m4a1_silencer" => M4A1,
            "m4a1_silencer_off" => M4A1,
            "m4a1" => M4A4,
            "sg556" => Sg556,

            "awp" => Awp,
            "g3sg1" => G3SG1,
            "scar20" => Scar20,
            "ssg08" => Ssg08,

            "taser" => Taser,

            "flashbang" => Flashbang,
            "hegrenade" => HeGrenade,
            "smokegrenade" => Smoke,
            "molotov" => Molotov,
            "decoy" => Decoy,
            "incgrenade" => Incendiary,

            "c4" => C4,

            _ => Unknown,
        }
    }
}
