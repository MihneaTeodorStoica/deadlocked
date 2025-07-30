use std::collections::HashMap;

use crate::process::{Module, Process};

// https://bin.sn3k.dev/c/YFXW

pub struct Schema {
    type_scopes: HashMap<String, ModuleScope>,
}

impl Schema {
    pub fn new(process: &Process, schema_module: u64) -> Option<Self> {
        let schema_system = process.scan_pattern(
            &[
                0x48, 0x8D, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x48, 0x8D, 0x3D, 0x00, 0x00, 0x00, 0x00,
                0xE8, 0x00, 0x00, 0x00, 0x00, 0xEB,
            ],
            "xxx????xxx????x????x".as_bytes(),
            schema_module,
        )?;
        let schema_system = process.get_relative_address(schema_system, 10, 7);
        let module = Module::new(process, schema_module);
        let schema_system = SchemaSystem::new(&module, schema_system);
        Some(Self {
            type_scopes: HashMap::new(),
        })
    }
}

struct SchemaSystem {
    scopes: Vec<ModuleScope>,
    num_registrations: i32,
}

impl SchemaSystem {
    fn new(module: &Module, address: u64) -> Self {
        let num_registrations = module.read(address + 0x320);
        let type_scopes_len = module.read(address + 0x1F0);
        let mut type_scopes = Vec::with_capacity(type_scopes_len as usize);
        for i in 0..type_scopes_len {
            let type_scope_address = module.read(address + 0x1F8 + (i * 8));
            let type_scope = ModuleScope::new(module, type_scope_address);
            type_scopes.push(type_scope);
        }
        Self {
            scopes: type_scopes,
            num_registrations,
        }
    }
}

struct ModuleScope {
    name: String,
}

impl ModuleScope {
    fn new(module: &Module, address: u64) -> Self {
        let name = module.read_string(address + 0x08);

        Self { name }
    }
}

struct Class {
    fields: HashMap<String, Field>,
}

struct Field {
    offset: u64,
    kind: String,
}
