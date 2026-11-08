//! Minecraft's `ResourceLocation` rules, which every id of the library follows (`slimeknights.mantle.util.ResourceId`).
use std::cmp::Ordering;
use std::fmt;

/// `namespace:path`. Equality is by both; ordering is by path first, then namespace, as Minecraft's `ResourceLocation.compareTo`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceId {
    namespace: String,
    path: String,
}

fn namespace_char(c: char) -> bool {
    c == '_' || c == '-' || c == '.' || c.is_ascii_digit() || c.is_ascii_lowercase()
}

fn path_char(c: char) -> bool {
    namespace_char(c) || c == '/'
}

/// Whether `path` is a valid path of a resource id (`ResourceLocation.isValidPath`).
pub fn is_valid_path(path: &str) -> bool {
    path.chars().all(path_char)
}

/// Whether `namespace` is a valid namespace (`ResourceLocation.isValidNamespace`).
pub fn is_valid_namespace(namespace: &str) -> bool {
    namespace.chars().all(namespace_char)
}

impl ResourceId {
    /// `None` if either part has a character an id may not (`ResourceLocation.tryBuild`).
    pub fn try_new(namespace: &str, path: &str) -> Option<Self> {
        (is_valid_namespace(namespace) && is_valid_path(path)).then(|| Self { namespace: namespace.to_owned(), path: path.to_owned() })
    }

    /// Splits `text` at the first colon and uses `default_namespace` when there is none (`ResourceLocation.tryParse`, with a default
    /// namespace as `IdParser.decompose`: an empty namespace before the colon also means the default).
    pub fn try_parse_in(default_namespace: &str, text: &str) -> Option<Self> {
        match text.split_once(':') {
            Some((namespace, path)) => Self::try_new(if namespace.is_empty() { default_namespace } else { namespace }, path),
            None => Self::try_new(default_namespace, text),
        }
    }

    /// `minecraft` is the default namespace, as in the game.
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::try_parse_in("minecraft", text)
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// The same id with `suffix` added to the path (`ResourceLocation.withSuffix`); `None` if that leaves a bad path.
    pub fn with_suffix(&self, suffix: &str) -> Option<Self> {
        Self::try_new(&self.namespace, &format!("{}{}", self.path, suffix))
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl PartialOrd for ResourceId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResourceId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path).then_with(|| self.namespace.cmp(&other.namespace))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_and_without_a_namespace() {
        assert_eq!(ResourceId::try_parse("tconstruct:wood").unwrap().to_string(), "tconstruct:wood");
        assert_eq!(ResourceId::try_parse("wood").unwrap().to_string(), "minecraft:wood");
        assert_eq!(ResourceId::try_parse_in("tconstruct", ":wood").unwrap().to_string(), "tconstruct:wood");
    }

    #[test]
    fn refuses_bad_characters() {
        assert!(ResourceId::try_parse("Tconstruct:wood").is_none());
        assert!(ResourceId::try_parse("tconstruct:Wood").is_none());
        assert!(ResourceId::try_parse("tconstruct:a b").is_none());
        assert!(ResourceId::try_parse("a/b:c").is_none(), "a slash is allowed in a path, not in a namespace");
        assert!(ResourceId::try_parse("tconstruct:a/b").is_some());
    }

    #[test]
    fn orders_by_path_first() {
        let a = ResourceId::try_parse("zzz:apple").unwrap();
        let b = ResourceId::try_parse("aaa:banana").unwrap();
        assert!(a < b);
        let c = ResourceId::try_parse("aaa:apple").unwrap();
        assert!(c < a);
    }
}
