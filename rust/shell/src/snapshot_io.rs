//! Shell-side snapshot reader. Same container the core writes:
//! `T18SNAP1`, zlib, then unique chunks.

use miniz_oxide::inflate::decompress_to_vec_zlib;

pub const RAM_LEN: usize = 16_777_216;
pub const SRAM_LEN: usize = 1_048_576;
pub const CPU_LEN: usize = 180;
pub const INFO_MIN: usize = 16;

#[derive(Clone, Debug)]
pub struct Chunks {
    pub items: Vec<(String, Vec<u8>)>,
}

impl Chunks {
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.items
            .iter()
            .find(|c| c.0 == name)
            .map(|c| c.1.as_slice())
    }

    pub fn require(&self, name: &str, len: Option<usize>) -> Result<&[u8], String> {
        let data = self
            .get(name)
            .ok_or_else(|| format!("not a valid snapshot: missing {name}"))?;
        if let Some(n) = len {
            if data.len() != n {
                return Err(format!("not a valid snapshot: {name} has the wrong size"));
            }
        }
        Ok(data)
    }
}

pub fn read_chunks(data: &[u8]) -> Result<Chunks, String> {
    if !data.starts_with(b"T18SNAP1") {
        return Err("not a valid snapshot: wrong file type".into());
    }
    let payload =
        decompress_to_vec_zlib(&data[8..]).map_err(|e| format!("not a valid snapshot: {e:?}"))?;
    let mut items = Vec::new();
    let mut i = 0;
    while i < payload.len() {
        let name_len = payload[i] as usize;
        i += 1;
        if i + name_len + 4 > payload.len() {
            return Err("not a valid snapshot: chunk too short".into());
        }
        let name = String::from_utf8_lossy(&payload[i..i + name_len]).into_owned();
        i += name_len;
        let len = u32::from_le_bytes(payload[i..i + 4].try_into().unwrap()) as usize;
        i += 4;
        if i + len > payload.len() {
            return Err("not a valid snapshot: chunk too short".into());
        }
        if items.iter().any(|c: &(String, Vec<u8>)| c.0 == name) {
            return Err("not a valid snapshot: duplicate chunk".into());
        }
        items.push((name, payload[i..i + len].to_vec()));
        i += len;
    }
    Ok(Chunks { items })
}

pub fn shelf_chunks(data: &[u8]) -> Result<Chunks, String> {
    let c = read_chunks(data)?;
    let info = c.require("info", None)?;
    if info.len() < INFO_MIN {
        return Err("not a valid snapshot: info".into());
    }
    c.require("ram", Some(RAM_LEN))?;
    c.require("sram", Some(SRAM_LEN))?;
    c.require("cpu", Some(CPU_LEN))?;
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_snapshots() {
        assert!(read_chunks(b"nope")
            .unwrap_err()
            .contains("not a valid snapshot"));
        assert!(read_chunks(b"T18SNAP1garbage").is_err());
    }
}
