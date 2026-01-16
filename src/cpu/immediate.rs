use serde::Deserialize;

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq)]
pub enum Immediate {
    #[serde(rename = "n8")]
    N8,

    #[serde(rename = "n16")]
    N16,

    #[serde(rename = "a8")]
    A8,

    #[serde(rename = "a16")]
    A16,

    #[serde(rename = "e8")]
    E8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_deserialize_variants() {
        // Mapping of JSON strings to their expected Enum variants
        let cases = vec![
            (r#""n8""#, Immediate::N8),
            (r#""n16""#, Immediate::N16),
            (r#""a8""#, Immediate::A8),
            (r#""a16""#, Immediate::A16),
            (r#""e8""#, Immediate::E8),
        ];

        for (json, expected) in cases {
            let deserialized: Immediate =
                serde_json::from_str(json).expect(&format!("Failed to deserialize {}", json));

            // We can compare these because of the #[derive(Debug)] and you should
            // likely add PartialEq to your derives!
            match (deserialized, expected) {
                (Immediate::N8, Immediate::N8) => (),
                (Immediate::N16, Immediate::N16) => (),
                (Immediate::A8, Immediate::A8) => (),
                (Immediate::A16, Immediate::A16) => (),
                (Immediate::E8, Immediate::E8) => (),
                _ => panic!(
                    "Deserialized value {:?} did not match expected",
                    deserialized
                ),
            }
        }
    }

    #[test]
    fn test_simple_equality() {
        let val: Immediate = serde_json::from_str(r#""n8""#).unwrap();
        assert_eq!(val, Immediate::N8);
    }

    #[test]
    fn test_invalid_input() {
        // Ensure that random strings or wrong cases fail to deserialize
        let result: Result<Immediate, _> = serde_json::from_str(r#""N8""#); // Wrong case
        assert!(result.is_err());

        let result: Result<Immediate, _> = serde_json::from_str(r#""unknown""#);
        assert!(result.is_err());
    }
}
