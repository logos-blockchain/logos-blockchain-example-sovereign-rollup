use serde::{Deserialize, Deserializer, Serialize};

use hex::FromHex;

#[derive(Serialize, Deserialize, Debug)]
pub struct CryptarchiaInfo {
    pub tip: HeaderId,
    pub slot: u64,
    pub height: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Copy, Hash, PartialOrd, Ord, Default)]
pub struct HeaderId([u8; 32]);

impl<'de> Deserialize<'de> for HeaderId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;

        let bytes = <[u8; 32]>::from_hex(hex_str)
            .map_err(|e| serde::de::Error::custom(format!("Invalid hex string: {}", e)))?;

        Ok(HeaderId(bytes))
    }
}

impl Serialize for HeaderId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex_str = hex::encode(self.0);
        serializer.serialize_str(&hex_str)
    }
}

use std::fmt;

impl fmt::Display for HeaderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}
