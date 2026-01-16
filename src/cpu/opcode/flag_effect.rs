use serde::de::{self, Deserialize, Deserializer, Visitor};
use std::fmt;

#[derive(Copy, Clone, Debug)]
pub enum FlagEffect {
    Zero,
    Subtract,
    HalfCarry,
    Carry,
    Reset,
    Set,
    Untouched,
}

impl FlagEffect {
    pub fn from_char(ch: char) -> Self {
        match ch {
            'Z' => FlagEffect::Zero,
            'N' => FlagEffect::Subtract,
            'H' => FlagEffect::HalfCarry,
            'C' => FlagEffect::Carry,
            '0' => FlagEffect::Reset,
            '1' => FlagEffect::Set,
            '-' => FlagEffect::Untouched,
            _ => panic!("Unsupported FlagEffect: '{}'", ch),
        }
    }
}

impl<'de> Deserialize<'de> for FlagEffect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FlagEffectVisitor;

        impl<'de> Visitor<'de> for FlagEffectVisitor {
            type Value = FlagEffect;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a single-character string representing a flag effect")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let ch = v
                    .chars()
                    .next()
                    .ok_or_else(|| E::custom("expected a non-empty string"))?;

                Ok(FlagEffect::from_char(ch))
            }

            fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(FlagEffect::from_char(v))
            }
        }

        deserializer.deserialize_any(FlagEffectVisitor)
    }
}
