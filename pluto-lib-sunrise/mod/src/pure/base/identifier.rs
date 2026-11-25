//! `Identifier` (Fabric/Minecraft: `namespace:path`).

use std::fmt;

/// The namespace used when an identifier is written without one, as in Minecraft.
pub const DEFAULT_NAMESPACE: &str = "minecraft";

/// A namespaced name such as `tconstruct:flint_pickaxe`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identifier {
    namespace: String,
    path: String,
}

/// Why an identifier was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentifierError {
    /// The namespace is empty or has a character outside `[a-z0-9_.-]`.
    InvalidNamespace(String),
    /// The path is empty or has a character outside `[a-z0-9_./-]`.
    InvalidPath(String),
}

impl fmt::Display for IdentifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNamespace(s) => write!(f, "invalid identifier namespace {s:?} (allowed: a-z 0-9 _ . -)"),
            Self::InvalidPath(s) => write!(f, "invalid identifier path {s:?} (allowed: a-z 0-9 _ . / -)"),
        }
    }
}

impl std::error::Error for IdentifierError {}

fn valid_namespace(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-'))
}

fn valid_path(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'/' | b'-'))
}

impl Identifier {
    /// Fabric: `Identifier.of(namespace, path)`.
    pub fn new(namespace: &str, path: &str) -> Result<Self, IdentifierError> {
        if !valid_namespace(namespace) {
            return Err(IdentifierError::InvalidNamespace(namespace.to_owned()));
        }
        if !valid_path(path) {
            return Err(IdentifierError::InvalidPath(path.to_owned()));
        }
        Ok(Self { namespace: namespace.to_owned(), path: path.to_owned() })
    }

    /// Fabric: `Identifier.of(String)`; `path` alone means the `minecraft` namespace.
    pub fn parse(text: &str) -> Result<Self, IdentifierError> {
        match text.split_once(':') {
            Some((namespace, path)) => Self::new(namespace, path),
            None => Self::new(DEFAULT_NAMESPACE, text),
        }
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_without_namespace_uses_minecraft() {
        assert_eq!(Identifier::parse("stone").unwrap(), Identifier::new("minecraft", "stone").unwrap());
    }

    #[test]
    fn parse_round_trips_through_display() {
        let id = Identifier::parse("tconstruct:tools/flint_pickaxe").unwrap();
        assert_eq!(id.to_string(), "tconstruct:tools/flint_pickaxe");
        assert_eq!((id.namespace(), id.path()), ("tconstruct", "tools/flint_pickaxe"));
    }

    #[test]
    fn rejects_uppercase_empty_and_bad_characters() {
        assert!(matches!(Identifier::parse("Mod:thing"), Err(IdentifierError::InvalidNamespace(_))));
        assert!(matches!(Identifier::parse("mod:Thing"), Err(IdentifierError::InvalidPath(_))));
        assert!(matches!(Identifier::parse("mod:"), Err(IdentifierError::InvalidPath(_))));
        assert!(matches!(Identifier::parse(":thing"), Err(IdentifierError::InvalidNamespace(_))));
        assert!(matches!(Identifier::parse("a:b:c"), Err(IdentifierError::InvalidPath(_))));
    }
}
