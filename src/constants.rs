pub mod cs2 {
    pub const PROCESS_NAME: &str = "cs2";
    pub const CLIENT_LIB: &str = "libclient.so";
    pub const ENGINE_LIB: &str = "libengine2.so";
    pub const TIER0_LIB: &str = "libtier0.so";
    pub const INPUT_LIB: &str = "libinputsystem.so";
    pub const SDL_LIB: &str = "libSDL3.so.0";
    pub const SCHEMA_LIB: &str = "libschemasystem.so";

    pub const LIBS: [&str; 6] = [
        CLIENT_LIB, ENGINE_LIB, TIER0_LIB, INPUT_LIB, SDL_LIB, SCHEMA_LIB,
    ];

    pub const TEAM_T: u8 = 2;
    pub const TEAM_CT: u8 = 3;

    pub const WEAPON_UNKNOWN: &str = "unknown";
    pub const DEFAULT_FOV: u32 = 90;
}

pub mod elf {
    pub const PROGRAM_HEADER_OFFSET: u64 = 0x20;
    pub const PROGRAM_HEADER_ENTRY_SIZE: u64 = 0x36;
    pub const PROGRAM_HEADER_NUM_ENTRIES: u64 = 0x38;

    pub const SECTION_HEADER_OFFSET: u64 = 0x28;
    pub const SECTION_HEADER_ENTRY_SIZE: u64 = 0x3A;
    pub const SECTION_HEADER_NUM_ENTRIES: u64 = 0x3C;

    pub const DYNAMIC_SECTION_PHT_TYPE: u64 = 0x02;
}
