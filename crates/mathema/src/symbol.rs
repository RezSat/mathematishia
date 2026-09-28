use std::fmt;

/// A free symbol constructor received an empty name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptySymbolName;

impl fmt::Display for EmptySymbolName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a symbol name must be non-empty")
    }
}

impl std::error::Error for EmptySymbolName {}

/// A free symbol identified by its exact, non-empty name.
///
/// Names are opaque strings, without Unicode normalization, lexical restrictions,
/// scopes, or assumptions. Ordering is lexicographic by the stored name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(String);

impl Symbol {
    /// Returns [`EmptySymbolName`] for an empty string.
    pub fn new(name: impl Into<String>) -> Result<Self, EmptySymbolName> {
        let name = name.into();
        if name.is_empty() {
            return Err(EmptySymbolName);
        }
        Ok(Self(name))
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
