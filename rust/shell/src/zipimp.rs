//! ZIP reader for ROM import and shelf packs. Unencrypted only.

use miniz_oxide::inflate::decompress_to_vec;

pub const ROM_MAX_BYTES: u64 = 128 * 1024 * 1024;
pub const ROM_MAX_ENTRIES: usize = 4096;
pub const ROM_MAX_BINS: usize = 32;
pub const SHELF_MAX_ENTRIES: usize = 10_000;
pub const SHELF_MAX_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZipMeta {
    pub name: String,
    pub method: u16,
    pub flags: u16,
    pub crc: u32,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub local_offset: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZipError {
    CannotRead,
    TooLarge,
}

pub fn is_zip(data: &[u8]) -> bool {
    data.len() >= 4
        && (data.starts_with(b"PK\x03\x04") || data.starts_with(b"PK\x05\x06") || data.starts_with(b"PK\x07\x08"))
}

pub fn list_zip(data: &[u8]) -> Result<Vec<ZipMeta>, ZipError> {
    if data.len() as u64 > ROM_MAX_BYTES.max(SHELF_MAX_BYTES) {
        return Err(ZipError::TooLarge);
    }
    let eocd = find_eocd(data).ok_or(ZipError::CannotRead)?;
    let total = u16::from_le_bytes([data[eocd + 10], data[eocd + 11]]) as usize;
    let cd_size = u32::from_le_bytes(data[eocd + 12..eocd + 16].try_into().unwrap()) as usize;
    let cd_off = u32::from_le_bytes(data[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    if cd_off.saturating_add(cd_size) > data.len() {
        return Err(ZipError::CannotRead);
    }
    let cd = &data[cd_off..cd_off + cd_size];
    let mut out = Vec::with_capacity(total);
    let mut i = 0;
    while i + 46 <= cd.len() {
        if &cd[i..i + 4] != b"PK\x01\x02" {
            return Err(ZipError::CannotRead);
        }
        let flags = u16::from_le_bytes([cd[i + 8], cd[i + 9]]);
        if flags & 1 != 0 {
            return Err(ZipError::CannotRead);
        }
        let method = u16::from_le_bytes([cd[i + 10], cd[i + 11]]);
        let crc = u32::from_le_bytes(cd[i + 16..i + 20].try_into().unwrap());
        let comp = u32::from_le_bytes(cd[i + 20..i + 24].try_into().unwrap()) as u64;
        let uncomp = u32::from_le_bytes(cd[i + 24..i + 28].try_into().unwrap()) as u64;
        let name_len = u16::from_le_bytes([cd[i + 28], cd[i + 29]]) as usize;
        let extra_len = u16::from_le_bytes([cd[i + 30], cd[i + 31]]) as usize;
        let comment_len = u16::from_le_bytes([cd[i + 32], cd[i + 33]]) as usize;
        let local = u32::from_le_bytes(cd[i + 42..i + 46].try_into().unwrap()) as u64;
        let name_off = i + 46;
        let name_end = name_off + name_len;
        if name_end > cd.len() {
            return Err(ZipError::CannotRead);
        }
        let name = String::from_utf8_lossy(&cd[name_off..name_end]).replace('\\', "/");
        out.push(ZipMeta { name, method, flags, crc, compressed_size: comp, uncompressed_size: uncomp, local_offset: local });
        i = name_end + extra_len + comment_len;
        if i > cd.len() {
            return Err(ZipError::CannotRead);
        }
    }
    if total != 0 && out.len() != total && out.is_empty() {
        return Err(ZipError::CannotRead);
    }
    Ok(out)
}

pub fn check_limits(entries: &[ZipMeta], max_entries: usize, max_bytes: u64, max_bins: Option<usize>) -> Result<(), ZipError> {
    if entries.len() > max_entries {
        return Err(ZipError::TooLarge);
    }
    let expanded: u64 = entries.iter().map(|e| e.uncompressed_size).sum();
    if expanded > max_bytes {
        return Err(ZipError::TooLarge);
    }
    if let Some(max) = max_bins {
        let bins = entries.iter().filter(|e| is_bin_name(&e.name)).count();
        if bins > max {
            return Err(ZipError::TooLarge);
        }
    }
    Ok(())
}

pub fn is_bin_name(name: &str) -> bool {
    base_name(name).to_ascii_lowercase().ends_with(".bin")
}

pub fn base_name(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

pub fn skip_rom_junk(name: &str) -> bool {
    if name.ends_with('/') {
        return true;
    }
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("__macosx/") {
        return true;
    }
    base_name(name).starts_with("._")
}

pub fn escapes(name: &str) -> bool {
    if name.starts_with('/') || name.starts_with('\\') {
        return true;
    }
    name.split(['/', '\\']).any(|p| p == "..")
}

pub fn extract(data: &[u8], meta: &ZipMeta) -> Result<Vec<u8>, ZipError> {
    if meta.flags & 1 != 0 || !(meta.method == 0 || meta.method == 8) {
        return Err(ZipError::CannotRead);
    }
    let off = meta.local_offset as usize;
    if off + 30 > data.len() || &data[off..off + 4] != b"PK\x03\x04" {
        return Err(ZipError::CannotRead);
    }
    let name_len = u16::from_le_bytes([data[off + 26], data[off + 27]]) as usize;
    let extra_len = u16::from_le_bytes([data[off + 28], data[off + 29]]) as usize;
    let start = off + 30 + name_len + extra_len;
    let end = start + meta.compressed_size as usize;
    if end > data.len() {
        return Err(ZipError::CannotRead);
    }
    let comp = &data[start..end];
    let out = if meta.method == 0 {
        comp.to_vec()
    } else {
        decompress_to_vec(comp).map_err(|_| ZipError::CannotRead)?
    };
    if out.len() as u64 != meta.uncompressed_size {
        return Err(ZipError::CannotRead);
    }
    if crc32(&out) != meta.crc {
        return Err(ZipError::CannotRead);
    }
    Ok(out)
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            let mask = (c & 1).wrapping_neg();
            c = (c >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !c
}

fn find_eocd(data: &[u8]) -> Option<usize> {
    if data.len() < 22 {
        return None;
    }
    let start = data.len().saturating_sub(22 + 65_535);
    let mut i = data.len() - 22;
    loop {
        if &data[i..i + 4] == b"PK\x05\x06" {
            let comment = u16::from_le_bytes([data[i + 20], data[i + 21]]) as usize;
            if i + 22 + comment == data.len() {
                return Some(i);
            }
        }
        if i == start {
            break;
        }
        i -= 1;
    }
    None
}

pub fn zip_stored(files: &[(&str, &[u8])]) -> Vec<u8> {
    zip_files(files, false)
}

pub fn zip_deflated(files: &[(&str, &[u8])]) -> Vec<u8> {
    zip_files(files, true)
}

fn zip_files(files: &[(&str, &[u8])], deflate: bool) -> Vec<u8> {
    let mut locals = Vec::new();
    let mut central = Vec::new();
    for (name, raw) in files {
        let name_b = name.as_bytes();
        let (method, payload) = if deflate {
            (8u16, miniz_oxide::deflate::compress_to_vec(raw, 6))
        } else {
            (0u16, raw.to_vec())
        };
        let crc = crc32(raw);
        let local_off = locals.len() as u32;
        locals.extend_from_slice(b"PK\x03\x04");
        locals.extend_from_slice(&20u16.to_le_bytes());
        locals.extend_from_slice(&0u16.to_le_bytes());
        locals.extend_from_slice(&method.to_le_bytes());
        locals.extend_from_slice(&0u16.to_le_bytes());
        locals.extend_from_slice(&0u16.to_le_bytes());
        locals.extend_from_slice(&crc.to_le_bytes());
        locals.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        locals.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        locals.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        locals.extend_from_slice(&0u16.to_le_bytes());
        locals.extend_from_slice(name_b);
        locals.extend_from_slice(&payload);

        central.extend_from_slice(b"PK\x01\x02");
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&method.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        central.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&local_off.to_le_bytes());
        central.extend_from_slice(name_b);
    }
    let cd_off = locals.len() as u32;
    let mut out = locals;
    let cd_len = central.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_len.to_le_bytes());
    out.extend_from_slice(&cd_off.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// A ZIP whose local headers are empty and whose central directory
/// declares `uncomp`. Starts with `PK\x03\x04` so importers treat it as a ZIP.
/// The declared sizes are what the limit checks see; the bytes are not stored.
pub fn zip_names(names: &[&str], uncomp: u32) -> Vec<u8> {
    let mut locals = Vec::new();
    let mut central = Vec::new();
    for name in names {
        let name_b = name.as_bytes();
        let local_off = locals.len() as u32;
        locals.extend_from_slice(b"PK\x03\x04");
        locals.extend_from_slice(&20u16.to_le_bytes());
        locals.extend_from_slice(&0u16.to_le_bytes());
        locals.extend_from_slice(&0u16.to_le_bytes());
        locals.extend_from_slice(&0u16.to_le_bytes());
        locals.extend_from_slice(&0u16.to_le_bytes());
        locals.extend_from_slice(&0u32.to_le_bytes());
        locals.extend_from_slice(&0u32.to_le_bytes());
        locals.extend_from_slice(&uncomp.to_le_bytes());
        locals.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        locals.extend_from_slice(&0u16.to_le_bytes());
        locals.extend_from_slice(name_b);

        central.extend_from_slice(b"PK\x01\x02");
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&uncomp.to_le_bytes());
        central.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&local_off.to_le_bytes());
        central.extend_from_slice(name_b);
    }
    let cd_off = locals.len() as u32;
    let cd_len = central.len() as u32;
    let mut out = locals;
    out.extend_from_slice(&central);
    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(names.len() as u16).to_le_bytes());
    out.extend_from_slice(&(names.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_len.to_le_bytes());
    out.extend_from_slice(&cd_off.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

pub fn empty_zip() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&[0u8; 18]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_limits_crc_and_junk() {
        assert!(is_zip(b"PK\x03\x04"));
        assert!(is_zip(b"PK\x05\x06...."));
        assert!(is_zip(b"PK\x07\x08...."));
        assert!(!is_zip(b"not a zip"));

        let stored = zip_stored(&[("a.txt", b"hi")]);
        let list = list_zip(&stored).unwrap();
        assert_eq!(extract(&stored, &list[0]).unwrap(), b"hi");

        let deflated = zip_deflated(&[("a.txt", b"hello hello hello")]);
        let list = list_zip(&deflated).unwrap();
        assert_eq!(list[0].method, 8);
        assert_eq!(extract(&deflated, &list[0]).unwrap(), b"hello hello hello");

        let mut bad = zip_stored(&[("a.txt", b"hi")]);
        // Flip a payload byte inside the local file so the CRC fails.
        let list = list_zip(&bad).unwrap();
        let off = list[0].local_offset as usize + 30 + 5;
        bad[off] ^= 0xFF;
        assert_eq!(extract(&bad, &list[0]), Err(ZipError::CannotRead));

        let mut enc = zip_stored(&[("a.txt", b"hi")]);
        // general-purpose bit at local+6 and central flag. list_zip reads central.
        // Set encrypt flag in the central directory (byte 8 of the CD entry).
        let cd = find_eocd(&enc).unwrap();
        let cd_off = u32::from_le_bytes(enc[cd + 16..cd + 20].try_into().unwrap()) as usize;
        enc[cd_off + 8] |= 1;
        assert_eq!(list_zip(&enc), Err(ZipError::CannotRead));

        let bins: Vec<String> = (0..33).map(|i| format!("f{i}.bin")).collect();
        let refs: Vec<&str> = bins.iter().map(|s| s.as_str()).collect();
        let z = zip_names(&refs, 0);
        let entries = list_zip(&z).unwrap();
        assert_eq!(check_limits(&entries, ROM_MAX_ENTRIES, ROM_MAX_BYTES, Some(ROM_MAX_BINS)), Err(ZipError::TooLarge));

        let many: Vec<String> = (0..4097).map(|i| format!("e{i}.txt")).collect();
        let refs: Vec<&str> = many.iter().map(|s| s.as_str()).collect();
        let z = zip_names(&refs, 0);
        let entries = list_zip(&z).unwrap();
        assert_eq!(check_limits(&entries, ROM_MAX_ENTRIES, ROM_MAX_BYTES, Some(ROM_MAX_BINS)), Err(ZipError::TooLarge));

        let huge = zip_names(&["a.bin"], (128 * 1024 * 1024 + 1) as u32);
        let entries = list_zip(&huge).unwrap();
        assert_eq!(check_limits(&entries, ROM_MAX_ENTRIES, ROM_MAX_BYTES, Some(32)), Err(ZipError::TooLarge));

        assert!(skip_rom_junk("__MACOSX/foo.bin"));
        assert!(skip_rom_junk("dir/"));
        assert!(skip_rom_junk("._foo.bin"));
        assert!(!skip_rom_junk("ROMs/Fairy.bin"));
        assert!(escapes("../x"));
        assert!(escapes("/abs"));
        assert!(!escapes("pack/shelf.json"));
        let empty = empty_zip();
        assert!(is_zip(&empty));
        assert!(list_zip(&empty).unwrap().is_empty());
    }
}
