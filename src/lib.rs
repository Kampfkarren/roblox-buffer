//! # roblox-buffer
//! This library exposes [`Buffer`] that serializes and deserializes to the `buffer` type in Roblox.
#![warn(missing_docs)]

use std::io::{Read, Write};

use data_encoding::BASE64;
use serde::{Deserialize, Serialize};

/// Represents a Roblox `buffer`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct Buffer(Vec<u8>);

impl Buffer {
    /// Creates a new buffer from bytes.
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    /// Returns the inner vector of the buffer.
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

impl<'de> Deserialize<'de> for Buffer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        enum BufferData {
            #[serde(rename = "base64")]
            Base64(String),
            #[serde(rename = "zbase64")]
            ZBase64(String),
        }

        #[derive(Deserialize)]
        struct BufferInner {
            t: String,
            #[serde(flatten)]
            data: BufferData,
        }

        let BufferInner { t, data } = BufferInner::deserialize(deserializer)?;

        if t != "buffer" {
            return Err(serde::de::Error::custom("expected buffer"));
        }

        let data = match data {
            BufferData::Base64(base64) => BASE64
                .decode(base64.as_bytes())
                .map_err(serde::de::Error::custom)?,
            BufferData::ZBase64(zbase64) => {
                let compressed = BASE64
                    .decode(zbase64.as_bytes())
                    .map_err(serde::de::Error::custom)?;
                let mut decoder = zstd::stream::Decoder::new(&compressed[..])
                    .map_err(serde::de::Error::custom)?;
                let mut data = Vec::new();
                decoder
                    .read_to_end(&mut data)
                    .map_err(serde::de::Error::custom)?;
                data
            }
        };

        Ok(Self(data))
    }
}

impl Serialize for Buffer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(3))?;

        map.serialize_entry("m", &())?; // "m": null
        map.serialize_entry("t", "buffer")?;

        // This DOESN'T deserialize in Roblox.
        // I couldn't figure out why.
        if false {
            let base64 = BASE64.encode(&self.0);

            let mut compressed: Vec<u8> = Vec::new();
            let mut encoder = zstd::stream::Encoder::new(&mut compressed, 0).unwrap();
            encoder
                .set_pledged_src_size(Some(self.0.len() as u64))
                .unwrap();
            encoder.include_contentsize(true).unwrap();
            encoder.write_all(&self.0).unwrap();
            encoder.finish().unwrap();

            if compressed.len() < base64.len() {
                map.serialize_entry("zbase64", &BASE64.encode(&compressed))?;
            } else {
                map.serialize_entry("base64", &base64)?;
            }
        }

        map.serialize_entry("base64", &BASE64.encode(&self.0))?;

        map.end()
    }
}

impl From<Buffer> for Vec<u8> {
    fn from(value: Buffer) -> Self {
        value.into_vec()
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Buffer {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl FromIterator<u8> for Buffer {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl Extend<u8> for Buffer {
    fn extend<T: IntoIterator<Item = u8>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_de() {
        assert_eq!(
            serde_json::from_str::<Buffer>(
                r#"{"m":null,"t":"buffer","base64":"aGVsbG8gd29ybGQ="}"#
            )
            .unwrap(),
            Buffer::new(b"hello world".to_vec())
        );
    }

    #[test]
    fn test_zbase64_de() {
        assert_eq!(
            serde_json::from_str::<Buffer>(r#"{"m":null,"t":"buffer","zbase64":"KLUv/SBfbQAAMGhlbGxvIAEAlqkUAQ=="}"#).unwrap(),
            Buffer::new(b"hello hello hello hello hello hello hello hello hello hello hello hello hello hello hello hello".to_vec())
        )
    }
}
