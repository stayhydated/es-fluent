//! Shared source-location metadata types.

use std::fmt;

/// Source file path recorded in generated inventory metadata.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceFile(String);

impl SourceFile {
    /// Creates a source file path when the recorded path is non-empty.
    pub fn new(path: impl Into<String>) -> Option<Self> {
        let path = path.into();
        if path.is_empty() {
            None
        } else {
            Some(Self(path))
        }
    }

    /// Returns the source file path as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SourceFile {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Source line recorded in generated inventory metadata.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceLine(u32);

impl SourceLine {
    /// Creates source line metadata.
    pub fn new(line: u32) -> Self {
        Self(line)
    }

    /// Returns the recorded line number.
    pub fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SourceLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

/// Source file and line metadata for a generated Fluent entry.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SourceLocation {
    file: SourceFile,
    line: SourceLine,
}

impl SourceLocation {
    /// Creates a source location when the recorded file path is non-empty.
    pub fn new(file: impl Into<String>, line: u32) -> Option<Self> {
        Some(Self {
            file: SourceFile::new(file)?,
            line: SourceLine::new(line),
        })
    }

    /// Returns the source file.
    pub fn file(&self) -> &SourceFile {
        &self.file
    }

    /// Returns the source line.
    pub fn line(&self) -> SourceLine {
        self.line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_file_rejects_empty_paths() {
        assert!(SourceFile::new("").is_none());
        assert_eq!(
            SourceFile::new("src/lib.rs").unwrap().as_str(),
            "src/lib.rs"
        );
    }

    #[test]
    fn source_line_and_location_preserve_values() {
        let line = SourceLine::new(42);
        assert_eq!(line.get(), 42);
        assert_eq!(line.to_string(), "42");

        let location = SourceLocation::new("src/lib.rs", 42).unwrap();
        assert_eq!(location.file().as_str(), "src/lib.rs");
        assert_eq!(location.line().get(), 42);
    }
}
