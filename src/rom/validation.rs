use super::error::LoadError;
use crate::*;
use std::fs;
use std::path::Path;

const GAME_BOY_FILE_EXT: &str = "gb";

pub fn load_rom(path: &Path) -> Result<Session, LoadError> {
    validate_extension(path)?;

    let buffer = fs::read(path)?;
    Ok(Session::new(buffer))
}

pub fn validate_extension(path: &Path) -> Result<(), LoadError> {
    let ext = path
        .extension()
        .ok_or(LoadError::MissingExtension)?
        .to_str()
        .ok_or(LoadError::MissingExtension)?;

    if !ext.eq_ignore_ascii_case(GAME_BOY_FILE_EXT) {
        return Err(LoadError::InvalidExtension {
            expected: GAME_BOY_FILE_EXT,
            found: ext.to_string(),
        });
    }

    Ok(())
}

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

    #[test]
    fn test_validate_extension_missing() {
        let path = Path::new("file"); // no extension
        let result = validate_extension(path);
        assert!(matches!(result, Err(LoadError::MissingExtension)));
    }

    #[test]
    fn test_validate_extension_invalid() {
        let cases = ["test.txt", "game.rom", "gb.txt", ".gb.txt"];
        for case in cases {
            let path = Path::new(case);
            let result = validate_extension(path);
            assert!(
                matches!(result, Err(LoadError::InvalidExtension { .. })),
                "Expected InvalidExtension for path {:?}, got {:?}",
                path,
                result
            );
        }
    }

    #[test]
    fn test_validate_extension_valid() {
        let path = Path::new("game.gb");
        let result = validate_extension(path);
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_wrong_extensions() {
        let cases = ["test.txt", "test.out", "game.rom", "gb.txt", ".gb.txt"];

        for case in cases {
            let path = Path::new(case);

            let result = validate_extension(path);

            assert!(
                matches!(result, Err(LoadError::InvalidExtension { .. })),
                "expected InvalidExtension for path {:?}, got {:?}",
                path,
                result
            );
        }
    }

    #[test]
    fn correct_file_extensions() {
        let cases = [
            "game.gb",
            "GaMe.GB",
            "Foreign keyboard chars åäö.Gb",
            "mIxEd.gB",
        ];

        for case in cases {
            let path = Path::new(case);

            let result = validate_extension(path);

            assert!(
                matches!(result, Ok { .. }),
                "expected Ok for path {:?}, got {:?}",
                path,
                result
            );
        }
    }
}
