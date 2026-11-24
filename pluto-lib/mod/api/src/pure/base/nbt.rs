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

    /// Reads SNBT text (`TagParser`, as the game's `parse_snbt` does): compounds, lists, typed
    /// arrays, quoted and bare strings, booleans and suffixed numbers. A trailing comma is allowed
    /// after the last entry of a compound, list or array; nesting past 512 is refused. The text
    /// must hold exactly one value: trailing characters after it are an error, as a predicate's
    /// `nbt` string must be one tag, not one tag followed by junk.
    pub fn parse_snbt(text: &str) -> Result<Nbt, String> {
        let mut parser = Snbt { s: text.as_bytes(), i: 0, depth: 0 };
        let value = parser.value()?;
        parser.skip();
        if parser.i != parser.s.len() {
            return Err(format!("trailing characters at {}", parser.i));
        }
        Ok(value)
    }
}

/// Whether the NBT `actual` carries the pattern `expected` (the game's `NbtUtils.compareNbt`
/// with the third argument true, as its `compare_nbt` spells it): the kinds must match exactly
/// (a byte is not an int, and no number widens); a compound pattern matches when the data has
/// at least its entries and every one matches; a list pattern matches when the data has at
/// least its elements and every pattern element matches some data element; anything else must
/// be equal. No data at all never matches.
pub fn compare_nbt(expected: &Nbt, actual: Option<&Nbt>) -> bool {
    let Some(actual) = actual else { return false };
    if std::mem::discriminant(expected) != std::mem::discriminant(actual) {
        return false;
    }
    match (expected, actual) {
        (Nbt::Compound(expected), Nbt::Compound(actual)) => {
            actual.len() >= expected.len()
                && expected.iter().all(|(key, value)| compare_nbt(value, actual.get(key)))
        }
        (Nbt::List(expected), Nbt::List(actual)) => {
            if expected.is_empty() {
                return actual.is_empty();
            }
            actual.len() >= expected.len()
                && expected
                    .iter()
                    .all(|wanted| actual.iter().any(|have| compare_nbt(wanted, Some(have))))
        }
        _ => expected == actual,
    }
}

/// SNBT nesting deeper than the game's own limit is refused rather than overflowing the stack.
const SNBT_MAX_DEPTH: usize = 512;

struct Snbt<'a> {
    s: &'a [u8],
    i: usize,
    depth: usize,
}

impl Snbt<'_> {
    fn skip(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip();
        self.s.get(self.i).copied()
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' at {}", c as char, self.i))
        }
    }

    fn bare(c: u8) -> bool {
        c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.' | b'+')
    }

    fn string(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(q @ (b'"' | b'\'')) => {
                self.i += 1;
                let mut out = Vec::new();
                while let Some(&c) = self.s.get(self.i) {
                    self.i += 1;
                    if c == b'\\' {
                        if let Some(&next) = self.s.get(self.i) {
                            self.i += 1;
                            out.push(match next {
                                b'n' => b'\n',
                                b't' => b'\t',
                                other => other,
                            });
                        }
                    } else if c == q {
                        return String::from_utf8(out).map_err(|e| e.to_string());
                    } else {
                        out.push(c);
                    }
                }
                Err("unterminated string".into())
            }
            _ => {
                let start = self.i;
                while self.s.get(self.i).is_some_and(|&c| Self::bare(c)) {
                    self.i += 1;
                }
                if start == self.i {
                    return Err(format!("expected a value at {start}"));
                }
                Ok(String::from_utf8_lossy(&self.s[start..self.i]).into_owned())
            }
        }
    }

    fn value(&mut self) -> Result<Nbt, String> {
        match self.peek() {
            Some(b'{') => {
                if self.depth >= SNBT_MAX_DEPTH {
                    return Err(format!("tag nesting too deep at {}", self.i));
                }
                self.depth += 1;
                let tag = self.compound();
                self.depth -= 1;
                tag
            }
            Some(b'[') => {
                if self.depth >= SNBT_MAX_DEPTH {
                    return Err(format!("tag nesting too deep at {}", self.i));
                }
                self.depth += 1;
                let tag = self.list();
                self.depth -= 1;
                tag
            }
            Some(b'"' | b'\'') => Ok(Nbt::String(self.string()?)),
            _ => {
                let word = self.string()?;
                Ok(Self::scalar(&word))
            }
        }
    }

    fn compound(&mut self) -> Result<Nbt, String> {
        self.i += 1;
        let mut map = NbtCompound::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Nbt::Compound(map));
        }
        loop {
            let key = self.string()?;
            self.expect(b':')?;
            map.insert(key, self.value()?);
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                    if self.peek() == Some(b'}') {
                        self.i += 1;
                        return Ok(Nbt::Compound(map));
                    }
                }
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Nbt::Compound(map));
                }
                _ => return Err(format!("expected ',' or '}}' at {}", self.i)),
            }
        }
    }

    fn list(&mut self) -> Result<Nbt, String> {
        self.i += 1;
        let typed = match (self.s.get(self.i), self.s.get(self.i + 1)) {
            (Some(&t @ (b'B' | b'I' | b'L')), Some(b';')) => {
                self.i += 2;
                Some(t)
            }
            _ => None,
        };
        let mut items = Vec::new();
        if self.peek() != Some(b']') {
            loop {
                items.push(self.value()?);
                match self.peek() {
                    Some(b',') => {
                        self.i += 1;
                        if self.peek() == Some(b']') {
                            break;
                        }
                    }
                    Some(b']') => break,
                    _ => return Err(format!("expected ',' or ']' at {}", self.i)),
                }
            }
        }
        self.i += 1;
        let long = |t: &Nbt| match *t {
            Nbt::Byte(v) => i64::from(v),
            Nbt::Short(v) => i64::from(v),
            Nbt::Int(v) => i64::from(v),
            Nbt::Long(v) => v,
            _ => 0,
        };
        Ok(match typed {
            Some(b'B') => Nbt::ByteArray(items.iter().map(|t| long(t) as u8).collect()),
            Some(b'I') => Nbt::IntArray(items.iter().map(|t| long(t) as i32).collect()),
            Some(_) => Nbt::LongArray(items.iter().map(long).collect()),
            None => Nbt::List(items),
        })
    }

    /// A bare word: a typed number, a boolean, or else a string. Only digits (and an exponent)
    /// make a number: a word like `nan` or `infd` that a float parse would take is a string here.
    fn scalar(word: &str) -> Nbt {
        match word {
            "true" => return Nbt::Byte(1),
            "false" => return Nbt::Byte(0),
            _ => {}
        }
        let lower = word.to_ascii_lowercase();
        let (body, suffix) = match lower.as_bytes().last() {
            Some(c @ (b'b' | b's' | b'l' | b'f' | b'd')) => (&word[..word.len() - 1], Some(*c)),
            _ => (word, None),
        };
        if body.bytes().any(|c| c.is_ascii_alphabetic() && c != b'e' && c != b'E') {
            return Nbt::String(word.to_owned());
        }
        let parsed = match suffix {
            Some(b'b') => body.parse::<i8>().ok().map(Nbt::Byte),
            Some(b's') => body.parse::<i16>().ok().map(Nbt::Short),
            Some(b'l') => body.parse::<i64>().ok().map(Nbt::Long),
            Some(b'f') => body
                .parse::<f32>()
                .ok()
                .map(|v| Nbt::Float(if v == 0.0 { 0.0 } else { v })),
            Some(_) => body
                .parse::<f64>()
                .ok()
                .map(|v| Nbt::Double(if v == 0.0 { 0.0 } else { v })),
            None if body.contains('.') || body.contains('e') || body.contains('E') => body
                .parse::<f64>()
                .ok()
                .map(|v| Nbt::Double(if v == 0.0 { 0.0 } else { v })),
            None => body.parse::<i32>().ok().map(Nbt::Int),
        };
        parsed.unwrap_or_else(|| Nbt::String(word.to_owned()))
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

    #[test]
    fn snbt_reads_the_forms_a_predicate_writes() {
        assert_eq!(Nbt::parse_snbt("{id:\"minecraft:chest\",Count:3b}").unwrap(), {
            let mut map = NbtCompound::new();
            map.insert("id".into(), Nbt::String("minecraft:chest".into()));
            map.insert("Count".into(), Nbt::Byte(3));
            Nbt::Compound(map)
        });
        assert_eq!(
            Nbt::parse_snbt("[B;1b,2b]").unwrap(),
            Nbt::ByteArray(vec![1, 2])
        );
        assert_eq!(Nbt::parse_snbt("true").unwrap(), Nbt::Byte(1));
        assert_eq!(Nbt::parse_snbt("2.5d").unwrap(), Nbt::Double(2.5));
        assert_eq!(Nbt::parse_snbt("7").unwrap(), Nbt::Int(7));
        assert_eq!(Nbt::parse_snbt("bare").unwrap(), Nbt::String("bare".into()));
        assert_eq!(Nbt::parse_snbt("{a:1,}").unwrap(), Nbt::parse_snbt("{a:1}").unwrap());
        assert!(Nbt::parse_snbt("{a:1} trailing").is_err());
        assert!(Nbt::parse_snbt("{a:}").is_err());
        assert!(Nbt::parse_snbt("nan").unwrap(), Nbt::String("nan".into()));
    }

    #[test]
    fn compare_nbt_matches_a_subset_and_nothing_else() {
        let pattern = Nbt::parse_snbt("{Items:[{id:\"minecraft:torch\"}]}").unwrap();
        let data = Nbt::parse_snbt("{Items:[{id:\"minecraft:torch\",Count:4b},{id:\"minecraft:stone\"}],Lock:\"x\"}").unwrap();
        assert!(compare_nbt(&pattern, Some(&data)));
        assert!(!compare_nbt(&pattern, None));
        // A byte is not an int, and a list pattern needs every element somewhere in the data.
        let wrong_kind = Nbt::parse_snbt("{Count:3}").unwrap();
        let counted = Nbt::parse_snbt("{Count:3b}").unwrap();
        assert!(!compare_nbt(&wrong_kind, Some(&counted)));
        let two = Nbt::parse_snbt("{Items:[{id:\"minecraft:torch\"},{id:\"minecraft:dirt\"}]}").unwrap();
        assert!(!compare_nbt(&two, Some(&data)));
        // An empty list pattern matches only an empty list.
        assert!(compare_nbt(
            &Nbt::List(vec![]),
            Some(&Nbt::List(vec![]))
        ));
        assert!(!compare_nbt(
            &Nbt::List(vec![]),
            Some(&Nbt::List(vec![Nbt::Int(1)]))
        ));
    }
}
