use std::collections::HashMap;

use crate::process::Process;

pub struct Schema {
    scopes: HashMap<String, ModuleScope>,
}

impl Schema {
    pub fn new(process: &Process, schema_module: u64) -> Option<Self> {
        let schema_system = process.scan(
            "48 8D 0D ? ? ? ? 48 8D 3D ? ? ? ? E8 ? ? ? ? EB",
            schema_module,
        )?;
        let schema_system = process.get_relative_address(schema_system, 10, 14);

        let type_scopes_len: i32 = process.read(schema_system + 0x1F0);
        let type_scopes_vec: u64 = process.read(schema_system + 0x1F8);
        let mut scopes = HashMap::new();
        for i in 0..type_scopes_len as u64 {
            let type_scope_address = process.read(type_scopes_vec + (i * 8));
            let type_scope = ModuleScope::new(process, type_scope_address);
            scopes.insert(type_scope.name.clone(), type_scope);
        }

        Some(Self { scopes })
    }

    #[allow(unused)]
    pub fn get(&self, library: &str, class: &str, field: &str) -> Option<u64> {
        let scope = self.scopes.get(library)?;
        let class = scope.get_class(class)?;
        class.get(field)
    }

    pub fn get_library(&self, library: &str) -> Option<&ModuleScope> {
        self.scopes.get(library)
    }
}

pub struct ModuleScope {
    name: String,
    classes: HashMap<String, Class>,
}

impl ModuleScope {
    fn new(process: &Process, address: u64) -> Self {
        let name = process.read_string_uncached(address + 0x08);

        let mut classes = HashMap::new();
        // length is (probably?) 256
        let hash_vector = address + 0x560 + 0x90;
        for i in 0..256 {
            // first_uncomitted
            let mut current_element: u64 = process.read(hash_vector + (i * 0x30) + 0x28);

            while current_element != 0 {
                let data: u64 = process.read(current_element + 0x10);
                if data != 0 {
                    let class = Class::new(process, data);
                    classes.insert(class.name.clone(), class);
                }
                current_element = process.read(current_element + 0x08);
            }
        }

        // free_list (HashAllocatedBlob)
        let mut current_blob: u64 = process.read(address + 0x560 + 0x20);
        while current_blob != 0 {
            let data: u64 = process.read(current_blob + 0x10);
            if data > process.min && data < process.max {
                let class = Class::new(process, data);
                classes.insert(class.name.clone(), class);
            }
            current_blob = process.read(current_blob);
        }

        Self { name, classes }
    }

    pub fn get(&self, class: &str, field: &str) -> Option<u64> {
        let Some(c) = self.classes.get(class) else {
            log::warn!("could not find class {class}");
            return None;
        };
        let f = c.get(field);
        if f.is_none() {
            log::warn!("could not find field {field} in class {class}");
        }
        f
    }

    pub fn get_class(&self, class: &str) -> Option<&Class> {
        self.classes.get(class)
    }
}

pub struct Class {
    name: String,
    fields: HashMap<String, u64>,
}

impl Class {
    fn new(process: &Process, address: u64) -> Self {
        let name = process.read_string_uncached(process.read(address + 0x08));

        let mut fields = HashMap::new();
        let field_count: i16 = process.read(address + 0x1C);
        if field_count < 0 {
            return Self { name, fields };
        }
        let fields_vec: u64 = process.read(address + 0x28);
        for i in 0..field_count as u64 {
            let field = Field::new(process, fields_vec + (0x20 * i));
            fields.insert(field.name, field.offset);
        }
        Self { name, fields }
    }

    fn get(&self, field: &str) -> Option<u64> {
        self.fields.get(field).copied()
    }
}

struct Field {
    name: String,
    offset: u64,
}

impl Field {
    fn new(process: &Process, address: u64) -> Self {
        let name = process.read_string_uncached(process.read(address));
        let offset = process.read::<i32>(address + 0x10) as u64;

        Self { name, offset }
    }
}
