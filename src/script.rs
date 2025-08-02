use std::sync::{Arc, Mutex};

use glam::{Vec2, Vec3};
use mlua::{FromLua, Lua, MetaMethod, UserData, Value};

use crate::{cs2::player::Player, data::Data};

#[allow(unused)]
pub struct Script {
    lua: Lua,
    data: Arc<Mutex<Data>>,
}

#[allow(unused)]
impl Script {
    pub fn new(data: Arc<Mutex<Data>>) -> Self {
        let lua = Lua::new();
        let ret = Self { lua, data };
        ret.init();
        ret
    }

    fn init(&self) -> mlua::Result<()> {
        let globals = self.lua.globals();

        globals.set(
            "vec2",
            self.lua
                .create_function(|_, (x, y)| Ok(LuaVec2(Vec2 { x, y })))?,
        )?;
        globals.set(
            "vec3",
            self.lua
                .create_function(|_lua, (x, y, z)| Ok(LuaVec3(Vec3 { x, y, z })))?,
        )?;

        let ws_data = self.data.clone();
        globals.set(
            "window_size",
            self.lua.create_function(move |_lua, ()| {
                Ok(LuaVec2(ws_data.lock().unwrap().window_size.as_vec2()))
            })?,
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct LuaVec2(Vec2);
#[derive(Debug, Clone, Copy)]
struct LuaVec3(Vec3);
#[derive(Debug, Clone, Copy)]
struct LuaPlayer(Player);

impl FromLua for LuaVec2 {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => unreachable!(),
        }
    }
}

impl FromLua for LuaVec3 {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => unreachable!(),
        }
    }
}

impl FromLua for LuaPlayer {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => unreachable!(),
        }
    }
}

impl UserData for LuaVec2 {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_lua, this| Ok(this.0.x));
        fields.add_field_method_set("x", |_lua, this, value| {
            this.0.x = value;
            Ok(())
        });

        fields.add_field_method_get("y", |_lua, this| Ok(this.0.y));
        fields.add_field_method_set("y", |_lua, this, value| {
            this.0.y = value;
            Ok(())
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Add, |_lua, this, rhs: LuaVec2| {
            Ok(LuaVec2(this.0 + rhs.0))
        });
        methods.add_meta_method(MetaMethod::Sub, |_lua, this, rhs: LuaVec2| {
            Ok(LuaVec2(this.0 - rhs.0))
        });
        methods.add_meta_method(MetaMethod::Mul, |lua, this, rhs: Value| {
            if let Ok(scalar) = f32::from_lua(rhs.clone(), lua) {
                Ok(LuaVec2(this.0 * scalar))
            } else if let Ok(vec) = LuaVec2::from_lua(rhs, lua) {
                Ok(LuaVec2(this.0 * vec.0))
            } else {
                Err(mlua::Error::RuntimeError(
                    "invalid type for multiplication".into(),
                ))
            }
        });
        methods.add_meta_method(MetaMethod::Div, |lua, this, rhs: Value| {
            if let Ok(scalar) = f32::from_lua(rhs.clone(), lua) {
                Ok(LuaVec2(this.0 / scalar))
            } else if let Ok(vec) = LuaVec2::from_lua(rhs, lua) {
                Ok(LuaVec2(this.0 / vec.0))
            } else {
                Err(mlua::Error::RuntimeError(
                    "invalid type for division".into(),
                ))
            }
        });
        methods.add_meta_method(MetaMethod::Unm, |_, this, _: ()| Ok(LuaVec2(-this.0)));

        methods.add_method("dot", |_, this, rhs: LuaVec2| Ok(this.0.dot(rhs.0)));
        methods.add_method("length", |_, this, _: ()| Ok(this.0.length()));
        methods.add_method("normalize", |_, this, _: ()| {
            Ok(LuaVec2(this.0.normalize()))
        });

        methods.add_meta_method(MetaMethod::ToString, |_, this, _: ()| {
            Ok(format!("Vec2({}, {})", this.0.x, this.0.y))
        });
    }
}

impl UserData for LuaVec3 {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_lua, this| Ok(this.0.x));
        fields.add_field_method_set("x", |_lua, this, value| {
            this.0.x = value;
            Ok(())
        });

        fields.add_field_method_get("y", |_lua, this| Ok(this.0.y));
        fields.add_field_method_set("y", |_lua, this, value| {
            this.0.y = value;
            Ok(())
        });

        fields.add_field_method_get("z", |_lua, this| Ok(this.0.z));
        fields.add_field_method_set("z", |_lua, this, value| {
            this.0.z = value;
            Ok(())
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Add, |_lua, this, rhs: LuaVec3| {
            Ok(LuaVec3(this.0 + rhs.0))
        });
        methods.add_meta_method(MetaMethod::Sub, |_lua, this, rhs: LuaVec3| {
            Ok(LuaVec3(this.0 - rhs.0))
        });
        methods.add_meta_method(MetaMethod::Mul, |lua, this, rhs: Value| {
            if let Ok(scalar) = f32::from_lua(rhs.clone(), lua) {
                Ok(LuaVec3(this.0 * scalar))
            } else if let Ok(vec) = LuaVec3::from_lua(rhs, lua) {
                Ok(LuaVec3(this.0 * vec.0))
            } else {
                Err(mlua::Error::RuntimeError(
                    "invalid type for multiplication".into(),
                ))
            }
        });
        methods.add_meta_method(MetaMethod::Div, |lua, this, rhs: Value| {
            if let Ok(scalar) = f32::from_lua(rhs.clone(), lua) {
                Ok(LuaVec3(this.0 / scalar))
            } else if let Ok(vec) = LuaVec3::from_lua(rhs, lua) {
                Ok(LuaVec3(this.0 / vec.0))
            } else {
                Err(mlua::Error::RuntimeError(
                    "invalid type for division".into(),
                ))
            }
        });
        methods.add_meta_method(MetaMethod::Unm, |_, this, _: ()| Ok(LuaVec3(-this.0)));

        methods.add_method("dot", |_, this, rhs: LuaVec3| Ok(this.0.dot(rhs.0)));
        methods.add_method("length", |_, this, _: ()| Ok(this.0.length()));
        methods.add_method("normalize", |_, this, _: ()| {
            Ok(LuaVec3(this.0.normalize()))
        });
        methods.add_method("cross", |_, this, rhs: LuaVec3| {
            Ok(LuaVec3(this.0.cross(rhs.0)))
        });

        methods.add_meta_method(MetaMethod::ToString, |_, this, _: ()| {
            Ok(format!("Vec2({}, {})", this.0.x, this.0.y))
        });
    }
}

impl UserData for LuaPlayer {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {}
}
