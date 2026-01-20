use super::error::LoadError;
use crate::cartridge::validation::validate_extension;
use std::fs;
use std::path::Path;

pub fn load_rom(path: &Path) -> Result<Vec<u8>, LoadError> {
    // info!("Loading ROM...");
    validate_extension(path)?;

    let buffer = fs::read(path)?;

    Ok(buffer)
}

#[cfg(test)]
mod tests {

    use super::*;

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
