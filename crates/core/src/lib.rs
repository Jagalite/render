//! Platform-independent authored state, transactions, geometry and reference rendering.
pub mod api;
pub mod document;
pub mod fixtures;
pub mod geometry;
pub mod imaging;
pub mod interchange;
pub mod jobs;
pub mod mesh_codec;
pub mod render;
pub mod storage;
pub mod topology;
pub mod uploads;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Error {
    pub code: String,
    pub stage: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default)]
    pub context: std::collections::BTreeMap<String, String>,
}
impl Error {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            stage: "validation".into(),
            message: message.into(),
            retryable: false,
            context: std::collections::BTreeMap::new(),
        }
    }
    pub fn at(mut self, stage: &str) -> Self {
        self.stage = stage.into();
        self
    }
    pub fn with_context(mut self, key: &str, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::new("encoding", e.to_string())
    }
}

pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Canonical JSON v0: sorted object keys, serde_json finite number spelling, UTF-8.
pub fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&serde_json::to_value(value)?)?)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u128);
impl Serialize for Id {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:032x}", self.0))
    }
}
impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        if text.len() != 32
            || !text
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(serde::de::Error::custom(
                "ID must be 32 lowercase hexadecimal digits",
            ));
        }
        u128::from_str_radix(&text, 16)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

/// Exact signed time in seconds. Frame conversion checks overflow and normalizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Time {
    pub numerator: i64,
    pub denominator: u64,
}
impl Time {
    pub fn new(numerator: i64, denominator: u64) -> Result<Self> {
        if denominator == 0 {
            return Err(Error::new("time", "zero denominator"));
        }
        let (mut a, mut b) = (numerator.unsigned_abs(), denominator);
        while b != 0 {
            (a, b) = (b, a % b);
        }
        Ok(Self {
            numerator: (i128::from(numerator) / i128::from(a)) as i64,
            denominator: denominator / a,
        })
    }
    pub fn frame(frame: i64, fps_numerator: u64, fps_denominator: u64) -> Result<Self> {
        if fps_denominator == 0 {
            return Err(Error::new("time", "zero frame-rate denominator"));
        }
        let n = i128::from(frame) * i128::from(fps_denominator);
        Self::new(
            i64::try_from(n).map_err(|_| Error::new("overflow", "frame time"))?,
            fps_numerator,
        )
    }
    pub fn seconds(self) -> Result<f64> {
        if self.denominator == 0 {
            Err(Error::new("time", "zero denominator"))
        } else {
            Ok(self.numerator as f64 / self.denominator as f64)
        }
    }
}
