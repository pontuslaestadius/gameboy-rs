use std::fmt;
use std::io;

#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    MissingExtension,
    InvalidExtension {
        expected: &'static str,
        found: String,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(err) => write!(f, "I/O error: {}", err),
            LoadError::MissingExtension => write!(f, "ROM file has no extension"),
            LoadError::InvalidExtension { expected, found } => write!(
                f,
                "Invalid ROM file extension: expected '{}', found '{}'",
                expected, found
            ),
        }
    }
}

impl From<io::Error> for LoadError {
    fn from(err: io::Error) -> Self {
        LoadError::Io(err)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_display_missing_extension() {
        let err = LoadError::MissingExtension;
        let msg = format!("{}", err);
        assert_eq!(msg, "ROM file has no extension");
    }

    #[test]
    fn test_display_invalid_extension() {
        let err = LoadError::InvalidExtension {
            expected: "gb",
            found: "txt".to_string(),
        };
        let msg = format!("{}", err);
        assert_eq!(
            msg,
            "Invalid ROM file extension: expected 'gb', found 'txt'"
        );
    }

    #[test]
    fn test_display_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "oh no");
        let err = LoadError::Io(io_err);
        let msg = format!("{}", err);
        assert!(msg.contains("I/O error: oh no"));
    }
}
