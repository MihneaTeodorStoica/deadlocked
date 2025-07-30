

#[derive(Pod)]
#[repr(C)]
pub struct BaseClassInfoData {
    pub offset: u32,                             // 0x0000
    pad_0004: [u8; 4],                           // 0x0004
    pub base_class: TypedPointer<ClassInfoData>, // 0x0008
}

#[derive(Pod)]
#[repr(C)]
pub struct ClassFieldData {
    pub name: StringPointer,                       // 0x0000
    pub schema_type: TypedPointer<SchemaType>,     // 0x0008
    pub single_inheritance_offset: i32,            // 0x0010
    pub metadata_count: i32,                       // 0x0014
    pub metadata: TypedPointer<MetadataEntryData>, // 0x0018
}

#[derive(Pod, Debug, Clone)]
#[repr(C)]
pub struct ClassInfoData {
    pub base: TypedPointer<ClassInfoData>,              // 0x0000
    pub name: StringPointer,                            // 0x0008
    pub module_name: StringPointer,                     // 0x0010
    pub size: i32,                                      // 0x0018
    pub fields_count: i16,                              // 0x001C
    pub static_fields_count: i16,                       // 0x001E
    pub static_metadata_count: i16,                     // 0x0020
    pub align_of: u8,                                   // 0x0022
    pub has_base_class: FatBool,                        // 0x0023
    pub total_class_size: i16,                          // 0x0024
    pub derived_class_size: i16,                        // 0x0026
    pub fields: TypedPointer<[ClassFieldData]>,         // 0x0028
    pub static_fields: TypedPointer<[StaticFieldData]>, // 0x0030
    pub base_class: TypedPointer<BaseClassInfoData>,    // 0x0038
    pad_0040: [u8; 0x8],                                // 0x0040
    pub static_metadata: TypedPointer<[MetadataEntryData]>, // 0x0048
    pub type_scope: TypedPointer<SystemTypeScope>,      // 0x0050
    pub schema_type: TypedPointer<SchemaType>,          // 0x0058
    pad_0060: [u8; 0x10],                               // 0x0060
}

thread_local! {
    #[allow(clippy::type_complexity)]
    static INHERITANCE_CACHE: RefCell<HashMap<(TypedPointer<ClassInfoData>, TypedPointer<ClassInfoData>), bool>> = RefCell::new(HashMap::new());
    #[allow(clippy::type_complexity)]
    static DESCENDANCE_CACHE: RefCell<HashMap<(TypedPointer<ClassInfoData>, String), bool>> = RefCell::new(HashMap::new());
}

impl ClassInfoData {
    pub fn descends_from(&self, other_name: &str) -> bool {
        DESCENDANCE_CACHE.with(|cache| {
            if let Some(res) = cache.borrow().get(&(self.base, other_name.to_owned())) {
                return *res;
            }
            let other = match OFFSETS
                .schemavars
                .get_class_info("libclient.so", other_name)
            {
                Ok(other) => other,
                Err(e) => {
                    log::warn!("Failed to get class info for entity: {:?}", e);
                    return false;
                }
            };
            let res = other.is_inherited_from(self);
            log::debug!(
                "Checking descendancy for {:?} and {:?}: {}",
                self.name.deref(),
                other.name.deref(),
                res
            );
            cache
                .borrow_mut()
                .insert((self.base, other_name.to_owned()), res);
            res
        })
    }

    fn is_inherited_from(&self, other: &Self) -> bool {
        INHERITANCE_CACHE.with(|cache| {
            if let Some(res) = cache.borrow().get(&(self.base, other.base)) {
                return *res;
            }
            let res = match self.is_inhered_from_recursive(other) {
                Ok(res) => res,
                Err(e) => {
                    log::warn!(
                        "Failed to check inheritance for {:?} and {:?}: {:?}",
                        self.name.deref(),
                        other.name.deref(),
                        e
                    );
                    false
                }
            };
            log::debug!(
                "Checking inheritance for {:?} and {:?}: {}",
                self.name.deref(),
                other.name.deref(),
                res
            );
            cache.borrow_mut().insert((self.base, other.base), res);
            res
        })
    }

    fn has_base_class(&self) -> bool {
        self.has_base_class.into()
    }

    fn is_inhered_from_recursive(&self, other: &Self) -> Result<bool> {
        if !self.has_base_class() || self.base_class.is_null() {
            return Ok(false);
        }
        log::debug!("self base class: {:?}", self.base_class);
        log::debug!("other base class: {:?}", other.base_class);
        let base_class_info = self.base_class.deref()?.base_class.deref()?;
        log::debug!("Base class name: {:?}", base_class_info.name.deref());
        if self.name.deref()? == other.name.deref()? {
            return Ok(true);
        }
        Ok(base_class_info.is_inherited_from(other))
    }
}

#[derive(Pod, Debug)]
#[repr(C)]
pub struct EnumInfoData {
    pub base: TypedPointer<EnumInfoData>,                 // 0x0000
    pub name: StringPointer,                              // 0x0008
    pub module_name: StringPointer,                       // 0x0010
    pub size: u8,                                         // 0x0018
    pub align_of: u8,                                     // 0x0019
    pad_001a: [u8; 0x2],                                  // 0x001A
    pub enumerators_count: u16,                           // 0x001C
    pub static_metadata_count: u16,                       // 0x001E
    pub enumerators: TypedPointer<[EnumeratorInfoData]>,  // 0x0020
    pub static_metadata: TypedPointer<MetadataEntryData>, // 0x0028
    pub type_scope: TypedPointer<SystemTypeScope>,        // 0x0030
    pub min_enumerator_value: i64,                        // 0x0038
    pub max_enumerator_value: i64,                        // 0x0040
}

#[derive(Pod)]
#[repr(C)]
pub struct EnumeratorInfoData {
    pub name: StringPointer,                       // 0x0000
    pub value: SchemaEnumeratorInfoDataUnion,      // 0x0008
    pub metadata_count: i32,                       // 0x0010
    pad_0014: [u8; 0x4],                           // 0x0014
    pub metadata: TypedPointer<MetadataEntryData>, // 0x0018
}

#[repr(C)]
pub union SchemaEnumeratorInfoDataUnion {
    pub uchar: u8,
    pub ushort: u16,
    pub uint: u32,
    pub ulong: u64,
}

#[derive(Pod)]
#[repr(C)]
pub struct MetadataEntryData {
    pub name: StringPointer,                       // 0x0000
    pub network_value: TypedPointer<NetworkValue>, // 0x0008
}

#[derive(Pod)]
#[repr(C)]
pub struct NetworkValue {
    pub value: NetworkValueUnion, // 0x0000
}

#[repr(C)]
pub union NetworkValueUnion {
    pub name_ptr: StringPointer,
    pub int_value: i32,
    pub float_value: f32,
    pub ptr: Pointer,
    pub var_value: SchemaVarName,
    pub name_value: [c_char; 32],
}

unsafe impl Pod for NetworkValueUnion {}

#[derive(Clone, Copy, Pod)]
#[repr(C)]
pub struct SchemaVarName {
    pub name: StringPointer,      // 0x0000
    pub type_name: StringPointer, // 0x0008
}

#[repr(u8)]
pub enum SchemaAtomicCategory {
    Basic = 0,
    T,
    CollectionOfT,
    TF,
    TT,
    #[allow(clippy::upper_case_acronyms)]
    TTF,
    I,
    None,
}

#[repr(u8)]
pub enum SchemaTypeCategory {
    BuiltIn = 0,
    Ptr,
    Bitfield,
    FixedArray,
    Atomic,
    DeclaredClass,
    DeclaredEnum,
    None,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SchemaArrayT {
    pub array_size: u32,                   // 0x0000
    pad_0004: [u8; 0x4],                   // 0x0004
    pub element: TypedPointer<SchemaType>, // 0x0008
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SchemaAtomicI {
    pad_0000: [u8; 0x10], // 0x0000
    pub value: u64,       // 0x0010
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SchemaAtomicT {
    pub element: TypedPointer<SchemaType>,  // 0x0000
    pad_0008: [u8; 0x8],                    // 0x0008
    pub template: TypedPointer<SchemaType>, // 0x0010
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SchemaAtomicTT {
    pad_0000: [u8; 0x10],                         // 0x0000
    pub templates: [TypedPointer<SchemaType>; 2], // 0x0010
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SchemaAtomicTF {
    pad_0000: [u8; 0x10],                   // 0x0000
    pub template: TypedPointer<SchemaType>, // 0x0010
    pub size: i32,                          // 0x0018
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SchemaAtomicTTF {
    pad_0000: [u8; 0x10],                         // 0x0000
    pub templates: [TypedPointer<SchemaType>; 2], // 0x0010
    pub size: i32,                                // 0x0020
}

#[repr(C)]
pub struct SchemaType {
    pad_0000: [u8; 0x8],                           // 0x0000
    pub name: StringPointer,                       // 0x0008
    pub type_scope: TypedPointer<SystemTypeScope>, // 0x0010
    pub type_category: SchemaTypeCategory,         // 0x0018
    pub atomic_category: SchemaAtomicCategory,     // 0x0019
    pub value: SchemaTypeUnion,                    // 0x0020
}

unsafe impl Pod for SchemaType {}

pub union SchemaTypeUnion {
    pub schema_type: TypedPointer<SchemaType>,
    pub class_binding: TypedPointer<ClassInfoData>,
    pub enum_binding: TypedPointer<EnumInfoData>,
    pub array: SchemaArrayT,
    pub atomic: SchemaAtomicT,
    pub atomic_tt: SchemaAtomicTT,
    pub atomic_tf: SchemaAtomicTF,
    pub atomic_ttf: SchemaAtomicTTF,
    pub atomic_i: SchemaAtomicI,
}

#[derive(Pod)]
#[repr(C)]
pub struct StaticFieldData {
    pub name: StringPointer,                       // 0x0000
    pub type_: TypedPointer<SchemaType>,           // 0x0008
    pub instance: Pointer,                         // 0x0010
    pub metadata_count: i32,                       // 0x0018
    pad_001c: [u8; 0x4],                           // 0x001C
    pub metadata: TypedPointer<MetadataEntryData>, // 0x0020
}

#[derive(Pod)]
#[repr(C)]
pub struct SchemaSystem {
    pad_0: [u8; 0x1f0],                                        // 0x000
    pub type_scopes: UtlVector<TypedPointer<SystemTypeScope>>, // 0x1f0
    pad_1: [u8; 0x120],                                        // 0x200
    pub num_registrations: i32,                                // 0x320
    pub num_ignored_data_bytes: i32,                           // 0x324
    pub num_redundant_data_bytes: i32,                         // 0x328
    pad_2: [u8; 0x4],                                          // 0x32c
}

#[derive(Pod)]
#[repr(C)]
pub struct SystemTypeScope {
    _vtable: [u8; 0x8],                              // 0x0000
    pub name: [c_char; 256],                         // 0x0008
    pub global_scope: TypedPointer<SystemTypeScope>, // 0x0108
    pad_0110: [u8; 0x450],                           // 0x0110
    pub class_bindings: UtlTsHash<ClassInfoData>,    // 0x0560
    pub enum_bindings: UtlTsHash<EnumInfoData>,      // 0x3600
}
impl SystemTypeScope {
    pub fn name(&self) -> String {
        self.name
            .iter()
            .take_while(|&&c| c != 0)
            .map(|c| *c as u8 as char)
            .collect::<String>()
    }
}

fn class_info_to_class(class_info: TypedPointer<ClassInfoData>) -> Result<(String, Class)> {
    let class_info = class_info.deref()?;
    let name = class_info.name.deref()?;
    let class = class_info.try_into()?;
    Ok((name, class))
}

#[derive(Clone, Serialize)]
struct TypeScope {
    #[serde(flatten)]
    classes: HashMap<String, Class>,
    #[serde(skip)]
    class_infos: HashMap<String, ClassInfoData>,
}
impl TryFrom<SystemTypeScope> for TypeScope {
    type Error = anyhow::Error;

    fn try_from(type_scope: SystemTypeScope) -> anyhow::Result<Self> {
        let mut classes = HashMap::new();
        let mut class_infos = HashMap::new();
        let class_bindings = type_scope.class_bindings.elements()?;
        for binding_ptr in class_bindings.iter() {
            match class_info_to_class(*binding_ptr) {
                Ok((name, class)) => {
                    classes.insert(name.clone(), class);
                    match binding_ptr.deref() {
                        Ok(class_info) => {
                            class_infos.insert(name, class_info);
                        }
                        Err(e) => {
                            log::warn!("Failed to get class info for {}: {}", name, e);
                        }
                    };
                }
                Err(err) => {
                    log::warn!(
                        "Failed to convert a class in {}: {}",
                        type_scope.name(),
                        err
                    );
                }
            }
        }
        log::info!(
            "Read type scope {} with {} classes, {} classes failed",
            type_scope.name(),
            class_bindings.len(),
            class_bindings.len() - classes.len(),
        );
        Ok(Self {
            classes,
            class_infos,
        })
    }
}

#[derive(Clone, Serialize)]
pub struct Class {
    #[serde(flatten)]
    fields: HashMap<String, Field>,
    pub size: i32,
    parent: Option<String>,
}
impl TryFrom<ClassInfoData> for Class {
    type Error = anyhow::Error;

    fn try_from(data: ClassInfoData) -> Result<Self> {
        let mut fields = HashMap::new();
        for i in 0..data.fields_count as usize {
            let field_data = data.fields.at(i)?.deref()?;
            fields.insert(field_data.name.deref()?, field_data.into());
        }
        let parent = data
            .base_class
            .deref()
            .and_then(|base_class| {
                let base_class_info = base_class.base_class.deref()?;
                base_class_info.name.deref().map(|name| name.to_owned())
            })
            .ok();
        Ok(Self {
            fields,
            size: data.size,
            parent,
        })
    }
}
impl Class {
    pub fn field_offset(&self, name: &str) -> Result<u64> {
        let offset = self
            .fields
            .get(name)
            .map(|field| field.offset)
            .with_context(|| format!("Field {} not found", name))?;
        log::debug!("Field offset for {}: 0x{:x?}", name, offset);
        Ok(offset)
    }
}

#[derive(Clone, Serialize)]
struct Field {
    offset: u64,
    type_: String,
}
impl From<ClassFieldData> for Field {
    fn from(data: ClassFieldData) -> Self {
        let type_ = data
            .schema_type
            .deref()
            .and_then(|type_| type_.name.deref())
            .inspect_err(|e| log::warn!("Failed to get field type: {}", e))
            .unwrap_or_default();
        Self {
            offset: data.single_inheritance_offset as u64,
            type_,
        }
    }
}

pub struct Schemavars {
    type_scopes: HashMap<String, TypeScope>,
}
impl Schemavars {
    pub fn new(modules: &Modules) -> anyhow::Result<Self> {
        let _timer = Timer::new("Schemavars");
        let schema_system_ptr = MEMORY
            .pattern_scan_module_single(
                &modules.schemasystem,
                pattern!("48 8d 0d ? ? ? ? 48 8d 3d ? ? ? ? e8 ? ? ? ? eb"),
            )?
            .add(10)
            .relative_to_absolute()?
            .typed();
        let schema_system: SchemaSystem = schema_system_ptr.deref()?;
        log::info!("Found schema system at 0x{:x?} with {} type scopes, {} registrations, {} ignored data bytes and {} redundant data bytes",
            schema_system_ptr.pointer.value_no_log(),
            schema_system.type_scopes.count(),
            schema_system.num_registrations,
            schema_system.num_ignored_data_bytes,
            schema_system.num_redundant_data_bytes);

        let mut type_scopes = HashMap::new();
        for i in 0..schema_system.type_scopes.count() as usize {
            let type_scope = schema_system.type_scopes.get(i)?.deref()?;
            let name = type_scope.name();
            let type_scope: TypeScope = type_scope
                .try_into()
                .with_context(|| format!("Failed to convert type scope {}", name))?;
            type_scopes.insert(name, type_scope);
        }
        let res = Self { type_scopes };
        if let Err(e) = res.dump_json() {
            log::warn!("Failed to dump schemavars: {}", e);
        }
        Ok(res)
    }

    pub fn get_class(&self, module: &str, class: &str) -> Result<Class> {
        log::debug!("Getting schema class {:?} from module {:?}", class, module);
        let type_scope = self.type_scopes.get(module).context("Module not found")?;
        let class = type_scope.classes.get(class).context("Class not found")?;
        Ok(class.clone())
    }

    pub fn get_class_info(&self, module: &str, class: &str) -> Result<ClassInfoData> {
        log::debug!(
            "Getting schema class info {:?} from module {:?}",
            class,
            module
        );
        let type_scope = self.type_scopes.get(module).context("Module not found")?;
        let class_info = type_scope
            .class_infos
            .get(class)
            .context("Class not found")?;
        Ok(class_info.clone())
    }

    fn dump_json(&self) -> Result<()> {
        let file = File::create("schemavars.json")?;
        serde_json::to_writer_pretty(file, &self.type_scopes)?;
        Ok(())
    }
}
