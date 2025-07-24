pub trait Offset {
    fn all_found(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct LibraryOffsets {
    pub client: u64,
    pub engine: u64,
    pub tier0: u64,
    pub input: u64,
    pub sdl: u64,
}

#[derive(Debug, Default)]
pub struct InterfaceOffsets {
    pub resource: u64,
    pub entity: u64,
    pub cvar: u64,
    pub player: u64,
    pub input: u64,
}

#[derive(Debug, Default)]
pub struct DirectOffsets {
    pub local_player: u64,
    pub button_state: u64,
    pub view_matrix: u64,
    pub sdl_window: u64,
    pub planted_c4: u64,
    pub global_vars: u64,
}

#[derive(Debug, Default)]
pub struct ConvarOffsets {
    pub ffa: u64,
    pub sensitivity: u64,
}

#[derive(Debug, Default)]
pub struct PlayerControllerOffsets {
    pub name: u64,        // Pointer -> String (m_sSanitizedPlayerName)
    pub pawn: u64,        // Pointer -> Pawn (m_hPawn)
    pub desired_fov: u64, // u32 (m_iDesiredFOV)
}

impl Offset for PlayerControllerOffsets {
    fn all_found(&self) -> bool {
        self.name != 0 && self.pawn != 0 && self.desired_fov != 0
    }
}

#[derive(Debug, Default)]
pub struct PawnOffsets {
    pub health: u64,           // i32 (m_iHealth)
    pub armor: u64,            // i32 (m_ArmorValue)
    pub team: u64,             // i32 (m_iTeamNum)
    pub life_state: u64,       // i32 (m_lifeState)
    pub weapon: u64,           // Pointer -> WeaponBase (m_pClippingWeapon)
    pub fov_multiplier: u64,   // f32 (m_flFOVSensitivityAdjust)
    pub game_scene_node: u64,  // Pointer -> GameSceneNode (m_pGameSceneNode)
    pub eye_offset: u64,       // Vec3 (m_vecViewOffset)
    pub velocity: u64,         // Vec3 (m_vecAbsVelocity)
    pub aim_punch_cache: u64,  // Vector<Vec3> (m_aimPunchCache)
    pub shots_fired: u64,      // i32 (m_iShotsFired)
    pub view_angles: u64,      // Vec2 (v_angle)
    pub spotted_state: u64,    // SpottedState (m_entitySpottedState)
    pub crosshair_entity: u64, // EntityIndex (m_iIDEntIndex)
    pub is_scoped: u64,        // bool (m_bIsScoped)
    pub flash_alpha: u64,      // f32 (m_flFlashMaxAlpha)
    pub flash_duration: u64,   // f32 (m_flFlashDuration)
    pub camera_services: u64,  // Pointer -> CameraServices (m_pCameraServices)
    pub item_services: u64,    // Pointer -> ItemServices (m_pItemServices)
    pub weapon_services: u64,  // Pointer -> WeaponSercies (m_pWeaponServices)
}

impl Offset for PawnOffsets {
    fn all_found(&self) -> bool {
        self.health != 0
            && self.armor != 0
            && self.team != 0
            && self.life_state != 0
            && self.weapon != 0
            && self.fov_multiplier != 0
            && self.game_scene_node != 0
            && self.eye_offset != 0
            && self.aim_punch_cache != 0
            && self.shots_fired != 0
            && self.view_angles != 0
            && self.spotted_state != 0
            && self.crosshair_entity != 0
            && self.is_scoped != 0
            && self.flash_alpha != 0
            && self.flash_duration != 0
            && self.camera_services != 0
            && self.item_services != 0
            && self.weapon_services != 0
    }
}

#[derive(Debug, Default)]
pub struct GameSceneNodeOffsets {
    pub dormant: u64,     // bool (m_bDormant)
    pub origin: u64,      // Vec3 (m_vecAbsOrigin)
    pub model_state: u64, // Pointer -> ModelState (m_modelState)
}

impl Offset for GameSceneNodeOffsets {
    fn all_found(&self) -> bool {
        self.dormant != 0 && self.origin != 0 && self.model_state != 0
    }
}

#[derive(Debug, Default)]
pub struct SmokeOffsets {
    pub did_smoke_effect: u64, // bool (m_bDidSmokeEffect)
    pub smoke_color: u64,      // Vec3 (m_vSmokeColor)
}

impl Offset for SmokeOffsets {
    fn all_found(&self) -> bool {
        self.did_smoke_effect != 0 && self.smoke_color != 0
    }
}

#[derive(Debug, Default)]
pub struct SpottedStateOffsets {
    pub spotted: u64, // bool (m_bSpotted)
    pub mask: u64,    // i32[2] or u64? (m_bSpottedByMask)
}

impl Offset for SpottedStateOffsets {
    fn all_found(&self) -> bool {
        self.spotted != 0 && self.mask != 0
    }
}

#[derive(Debug, Default)]
pub struct CameraServicesOffsets {
    pub fov: u64, // u32 (m_iFOV)
}

impl Offset for CameraServicesOffsets {
    fn all_found(&self) -> bool {
        self.fov != 0
    }
}

#[derive(Debug, Default)]
pub struct ItemServicesOffsets {
    pub has_defuser: u64, // bool (m_bHasDefuser)
    pub has_helmet: u64,  // bool (m_bHasHelmet)
}

impl Offset for ItemServicesOffsets {
    fn all_found(&self) -> bool {
        self.has_defuser != 0 && self.has_helmet != 0
    }
}

#[derive(Debug, Default)]
pub struct WeaponServicesOffsets {
    pub weapons: u64, // Pointer -> Vec<Pointer -> Weapon> (m_hMyWeapons)
}

impl Offset for WeaponServicesOffsets {
    fn all_found(&self) -> bool {
        self.weapons != 0
    }
}

#[derive(Debug, Default)]
pub struct PlantedC4Offsets {
    pub is_activated: u64,  // bool (m_bC4Activated)
    pub is_ticking: u64,    // bool (m_bBombTicking)
    pub bomb_site: u64,     // i32 (m_nBombSite)
    pub blow_time: u64,     // f32 (m_flC4Blow)
    pub being_defused: u64, // bool (m_bBeingDefused)
}

impl Offset for PlantedC4Offsets {
    fn all_found(&self) -> bool {
        self.is_activated != 0
            && self.is_ticking != 0
            && self.bomb_site != 0
            && self.blow_time != 0
            && self.being_defused != 0
    }
}

#[derive(Debug, Default)]
pub struct Offsets {
    pub library: LibraryOffsets,
    pub interface: InterfaceOffsets,
    pub direct: DirectOffsets,
    pub convar: ConvarOffsets,
    pub controller: PlayerControllerOffsets,
    pub pawn: PawnOffsets,
    pub game_scene_node: GameSceneNodeOffsets,
    pub smoke: SmokeOffsets,
    pub spotted_state: SpottedStateOffsets,
    pub camera_services: CameraServicesOffsets,
    pub item_services: ItemServicesOffsets,
    pub weapon_services: WeaponServicesOffsets,
    pub planted_c4: PlantedC4Offsets,
}

impl Offset for Offsets {
    fn all_found(&self) -> bool {
        self.controller.all_found()
            && self.pawn.all_found()
            && self.game_scene_node.all_found()
            && self.smoke.all_found()
            && self.spotted_state.all_found()
            && self.camera_services.all_found()
            && self.item_services.all_found()
            && self.weapon_services.all_found()
            && self.planted_c4.all_found()
    }
}
