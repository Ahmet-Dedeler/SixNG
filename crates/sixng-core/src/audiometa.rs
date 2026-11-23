//! Audio metadata files: the sound, game-object, curve, speech, music,
//! mixer and synth definitions that go with a pack's audio banks (plain
//! `bin` files; a content pack's text file lists them as `<pack>:<file>:
//! Speech / GameObjects / AudioConfig / Sounds / ModularSynth / Curves /
//! DynamicMixer / Music`).
//!
//! Layout (little-endian), close to the GTA V audio metadata files:
//!
//! ```text
//! u32 type (schema id: 18, 20, 25, 26, 61, 86, 166, 581 in the minigame pack)
//! u32 data length, then the data (+8): the objects, back to back
//! u32 0x045a88ea at +8 (the first data word in every file)
//! two tables, each u32 byte length L then L - 4 bytes: u32 count, then
//!   count x {u32, u32} pairs and the hashes they refer to (empty: 4, 0)
//! u32 object count, then count x {u32 name hash, u32 data offset}
//!   (the index, sorted by the hash bytes; names are not stored)
//! further offset tables (hash and container references) follow; their
//!   layout differs between schemas and is not decoded.
//! ```
//!
//! Each object starts with a u8 class id (the first byte at its offset).
//! Object sizes are the distance to the next object's offset. Field layouts
//! per class are not decoded.
use anyhow::{bail, Result};
use serde::Serialize;

pub const MAGIC: u32 = 0x045a_88ea;

#[derive(Debug, Clone, Serialize)]
pub struct Object {
    /// Name hash (joaat of the object name; names are not in the install).
    pub hash: String,
    /// Offset in the data block.
    pub offset: u32,
    /// Bytes to the next object (or the end of the data).
    pub size: u32,
    /// Class id: the first byte of the object.
    pub class: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioMeta {
    pub schema: u32,
    pub data_len: u32,
    /// Entry counts of the two tables before the index.
    pub table_counts: [u32; 2],
    pub objects: Vec<Object>,
    /// Objects per class id.
    pub classes: std::collections::BTreeMap<u8, usize>,
}

pub fn is_audio_meta(b: &[u8]) -> bool {
    b.len() >= 16 && u32::from_le_bytes(b[8..12].try_into().unwrap()) == MAGIC && (u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize) + 8 <= b.len()
}

pub fn parse(b: &[u8]) -> Result<AudioMeta> {
    if !is_audio_meta(b) {
        bail!("not an audio metadata file");
    }
    let u = |o: usize| -> Result<u32> { b.get(o..o + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap())).ok_or_else(|| anyhow::anyhow!("truncated at {o:#x}")) };
    let (schema, data_len) = (u(0)?, u(4)?);
    let mut p = 8 + data_len as usize;
    let mut table_counts = [0u32; 2];
    for c in &mut table_counts {
        let len = u(p)? as usize;
        if len < 4 {
            bail!("bad table length {len} at {p:#x}");
        }
        *c = u(p + 4)?;
        p += 4 + len;
    }
    let n = u(p)? as usize;
    if n > 1 << 20 || p + 4 + 8 * n > b.len() {
        bail!("bad object count {n}");
    }
    let mut idx: Vec<(u32, u32)> = (0..n).map(|i| Ok((u(p + 4 + 8 * i)?, u(p + 8 + 8 * i)?))).collect::<Result<_>>()?;
    if idx.iter().any(|&(_, o)| o >= data_len) {
        bail!("object offset past the data");
    }
    let mut offs: Vec<u32> = idx.iter().map(|x| x.1).collect();
    offs.sort_unstable();
    offs.dedup();
    idx.sort_by_key(|x| x.1);
    let objects: Vec<Object> = idx
        .iter()
        .map(|&(h, o)| {
            let next = offs.iter().find(|&&x| x > o).copied().unwrap_or(data_len);
            Object { hash: format!("{h:08x}"), offset: o, size: next - o, class: b[8 + o as usize] }
        })
        .collect();
    let mut classes = std::collections::BTreeMap::new();
    for o in &objects {
        *classes.entry(o.class).or_insert(0) += 1;
    }
    Ok(AudioMeta { schema, data_len, table_counts, objects, classes })
}

/// Decode every audio metadata file of an archive to
/// `<out>/audio_meta/<archive>/<offset>.json` plus `index.jsonl`.
/// Returns (decoded, failed).
pub fn export(a: &crate::game::ArchiveInfo, entries: &[&crate::scan::Entry], out: &std::path::Path) -> Result<(usize, usize)> {
    let dir = out.join("audio_meta").join(a.id.trim_end_matches(".rpf"));
    std::fs::create_dir_all(&dir)?;
    let (mut ok, mut bad) = (0, 0);
    let mut lines = String::new();
    for e in entries {
        let Ok(b) = crate::scan::read_entry(&a.path, e) else { continue };
        if !is_audio_meta(&b) {
            continue;
        }
        match parse(&b) {
            Ok(m) => {
                ok += 1;
                let file = format!("{:x}.json", e.off);
                std::fs::write(dir.join(&file), serde_json::to_vec_pretty(&serde_json::json!({"id": e.id, "meta": m}))?)?;
                lines += &serde_json::json!({"id": e.id, "file": file, "schema": m.schema, "objects": m.objects.len(), "table_counts": m.table_counts, "classes": m.classes}).to_string();
            }
            Err(x) => {
                bad += 1;
                lines += &serde_json::json!({"id": e.id, "error": x.to_string()}).to_string();
            }
        }
        lines.push('\n');
    }
    std::fs::write(dir.join("index.jsonl"), lines)?;
    Ok((ok, bad))
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_minimal_file() {
        // schema 26, 8 data bytes (two objects of 4), empty tables, 2 index entries
        let mut b = Vec::new();
        for v in [26u32, 8, super::MAGIC, 0x0300_0000, 4, 0, 4, 0, 2, 0xaaaa, 0, 0xbbbb, 4] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        let m = super::parse(&b).unwrap();
        assert_eq!(m.objects.len(), 2);
        assert_eq!((m.objects[0].class, m.objects[0].size), (0xea, 4));
        assert_eq!((m.objects[1].class, m.objects[1].size), (0x00, 4));
    }
}
