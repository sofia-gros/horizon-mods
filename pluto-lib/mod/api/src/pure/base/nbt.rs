//! NBT, the tagged data the game saves and sends (Minecraft: `net.minecraft.nbt`). PlutoLib's own copy so that code using it does
//! not depend on the game's types.

use std::collections::BTreeMap;

/// One NBT value.
#[derive(Clone, Debug, PartialEq)]
pub enum Nbt {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List(Vec<Nbt>),
    Compound(NbtCompound),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

/// A named set of NBT values (Minecraft: `CompoundTag`).
pub type NbtCompound = BTreeMap<String, Nbt>;

impl Nbt {
    /// Any integer tag as an `i64` (`Byte`, `Short`, `Int`, `Long`); None for other tags.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Nbt::Byte(v) => Some(i64::from(*v)),
            Nbt::Short(v) => Some(i64::from(*v)),
            Nbt::Int(v) => Some(i64::from(*v)),
            Nbt::Long(v) => Some(*v),
            _ => None,
        }
    }

    /// Any number tag as an `f64`; None for other tags.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Nbt::Float(v) => Some(f64::from(*v)),
            Nbt::Double(v) => Some(*v),
            other => other.as_i64().map(|v| v as f64),
        }
    }

    /// The text of a `String` tag.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Nbt::String(text) => Some(text),
            _ => None,
        }
    }

    /// The compound inside a `Compound` tag.
    pub fn as_compound(&self) -> Option<&NbtCompound> {
        match self {
            Nbt::Compound(compound) => Some(compound),
            _ => None,
        }
    }

    /// The elements of a `List` tag.
    pub fn as_list(&self) -> Option<&[Nbt]> {
        match self {
            Nbt::List(items) => Some(items),
            _ => None,
        }
    }
}

/// Reading typed values out of a compound with a default (Minecraft: `CompoundTag.getInt` and friends).
pub trait NbtCompoundExt {
    /// The integer under `key`, or `default` if it is missing or not an integer tag.
    fn get_int(&self, key: &str, default: i64) -> i64;
    /// The number under `key`, or `default`.
    fn get_double(&self, key: &str, default: f64) -> f64;
    /// The text under `key`, if it is a string tag.
    fn get_string(&self, key: &str) -> Option<&str>;
    /// The compound under `key`, if there is one.
    fn get_compound(&self, key: &str) -> Option<&NbtCompound>;
}

impl NbtCompoundExt for NbtCompound {
    fn get_int(&self, key: &str, default: i64) -> i64 {
        self.get(key).and_then(Nbt::as_i64).unwrap_or(default)
    }

    fn get_double(&self, key: &str, default: f64) -> f64 {
        self.get(key).and_then(Nbt::as_f64).unwrap_or(default)
    }

    fn get_string(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Nbt::as_str)
    }

    fn get_compound(&self, key: &str) -> Option<&NbtCompound> {
        self.get(key).and_then(Nbt::as_compound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_reads_follow_the_tag_kind_and_fall_back_to_the_default() {
        let mut inner = NbtCompound::new();
        inner.insert("n".into(), Nbt::Int(7));
        let mut data = NbtCompound::new();
        data.insert("count".into(), Nbt::Short(3));
        data.insert("ratio".into(), Nbt::Float(0.5));
        data.insert("name".into(), Nbt::String("melter".into()));
        data.insert("inner".into(), Nbt::Compound(inner));
        data.insert("list".into(), Nbt::List(vec![Nbt::Int(1)]));
        assert_eq!(data.get_int("count", 0), 3);
        assert_eq!(data.get_int("missing", -1), -1);
        assert_eq!(data.get_int("name", -2), -2, "a string is not an integer");
        assert_eq!(data.get_double("ratio", 0.0), 0.5);
        assert_eq!(data.get_double("count", 0.0), 3.0);
        assert_eq!(data.get_string("name"), Some("melter"));
        assert_eq!(data.get_compound("inner").unwrap().get_int("n", 0), 7);
        assert_eq!(data["list"].as_list().unwrap().len(), 1);
    }
}
