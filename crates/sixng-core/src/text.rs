//! Text tables: the game's localised strings (UI, subtitles, item names,
//! phone and ambient speech). Each table is an unnamed 0x7a meta resource
//! (data.rpf, pack shared_minigame000.rpf) with one copy per language.
//!
//! Header: block table at 0x38 ({u32 struct hash, u32 size, u64 ptr} per
//! block, count u16 at 0x4C). A text table has three blocks:
//! - struct 0xa8478889: one 16-byte string ref per string, the string's
//!   byte offset into the blob in bits 20.. of the u64 at +8;
//! - 0x101: a hash map of 16-byte slots (groups of control bytes and
//!   slots); a used slot is {u32 label hash, u32 0, u64 (index << 24) | 1};
//! - 0x10: the UTF-8 string blob, NUL-terminated strings.
//!
//! The label hash is joaat(lowercase label). For dialogue subtitles the
//! label is the voice-line name plus a 2-digit variation
//! (`AG_SUR_GEAR_FIRST1_01`). Strings carry inline markup: `~sl:gap:dur~`
//! splits a line into timed subtitle pieces (seconds), `~n~` new line,
//! `~b~` highlight, `~1~` parameters, `~INPUT_…~` button icons.
use anyhow::{bail, Result};
use std::collections::BTreeMap;

const STRING_REFS: u32 = 0xa847_8889;
const HASH_MAP: u32 = 0x101;
const BLOB: u32 = 0x10;

fn rd<const N: usize>(b: &[u8], o: usize) -> Option<[u8; N]> {
    b.get(o..o + N).map(|s| s.try_into().unwrap())
}
fn u16_(b: &[u8], o: usize) -> u16 {
    rd::<2>(b, o).map(u16::from_le_bytes).unwrap_or(0)
}
fn u32_(b: &[u8], o: usize) -> u32 {
    rd::<4>(b, o).map(u32::from_le_bytes).unwrap_or(0)
}
fn u64_(b: &[u8], o: usize) -> u64 {
    rd::<8>(b, o).map(u64::from_le_bytes).unwrap_or(0)
}

/// Blocks of a 0x7a meta resource (struct hash, offset, size); system-page
/// pointers only.
fn blocks(b: &[u8]) -> Vec<(u32, usize, usize)> {
    let t = u64_(b, 0x38);
    if b.get(0x10..0x15) != Some(b"0DRPz") || t >> 28 != 5 {
        return Vec::new();
    }
    let t = (t & 0x0FFF_FFFF) as usize;
    (0..u16_(b, 0x4C) as usize)
        .filter_map(|i| {
            let o = t + 16 * i;
            let p = u64_(b, o + 8);
            let (at, size) = ((p & 0x0FFF_FFFF) as usize, u32_(b, o + 4) as usize);
            (p >> 28 == 5 && at + size <= b.len()).then_some((u32_(b, o), at, size))
        })
        .collect()
}

/// Cheap test: the resource has the three text-table blocks.
pub fn is_text_table(b: &[u8]) -> bool {
    let bl = blocks(b);
    [STRING_REFS, HASH_MAP, BLOB].iter().all(|h| bl.iter().any(|x| x.0 == *h))
}

/// Decode a text table (the whole decompressed resource): label hash ->
/// string, sorted by hash.
pub fn parse(b: &[u8]) -> Result<BTreeMap<u32, String>> {
    let bl = blocks(b);
    let find = |h: u32| bl.iter().find(|x| x.0 == h).copied();
    let (Some(refs), Some(map), Some(blob)) = (find(STRING_REFS), find(HASH_MAP), find(BLOB)) else {
        bail!("not a text table");
    };
    let n = refs.2 / 16;
    let mut labels = Vec::new();
    for o in (map.1..map.1 + map.2).step_by(16) {
        let (h, z, v) = (u32_(b, o), u32_(b, o + 4), u64_(b, o + 8));
        if h != 0 && z == 0 && v & 0xFF_FFFF == 1 {
            let i = (v >> 24) as usize;
            if i < n {
                labels.push((h, i));
            }
        }
    }
    if labels.is_empty() && n > 0 {
        bail!("hash map has no entries");
    }
    let bytes = &b[blob.1..blob.1 + blob.2];
    let mut out = BTreeMap::new();
    for (h, i) in labels {
        let at = (u64_(b, refs.1 + 16 * i + 8) >> 20) as usize;
        let Some(s) = bytes.get(at..) else { bail!("string offset past the blob") };
        let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
        out.insert(h, String::from_utf8_lossy(&s[..end]).into_owned());
    }
    Ok(out)
}

/// Strip the inline markup (`~…~` codes) and join timed pieces with a space.
pub fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('~') {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 1..];
        match tail.find('~') {
            Some(j) => {
                let code = &tail[..j];
                if code == "n" || code.starts_with("sl:") {
                    out.push(' ');
                }
                rest = &tail[j + 1..];
            }
            None => {
                out.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub struct Table {
    pub id: String,
    pub off: u64,
    pub strings: BTreeMap<u32, String>,
}

/// Read every text table of an archive (in offset order). Returns the
/// tables and the number of candidates that failed to decode.
pub fn read_all(a: &crate::game::ArchiveInfo, all: &[crate::scan::Entry], only: &[u64], progress: crate::export::Progress) -> Result<(Vec<Table>, usize)> {
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    let cands: Vec<&crate::scan::Entry> = all
        .iter()
        .filter(|e| e.sub == "7a" && e.usize <= 64 << 20 && (only.is_empty() || only.contains(&e.off)))
        .collect();
    let total = cands.len() as u64;
    let done = AtomicU64::new(0);
    let errors = AtomicUsize::new(0);
    let mut tables: Vec<Table> = cands
        .par_iter()
        .filter_map(|e| {
            let d = done.fetch_add(1, Ordering::Relaxed) + 1;
            if d % 256 == 0 || d == total {
                progress(&e.id, d, total);
            }
            let b = crate::scan::read_entry(&a.path, e).ok()?;
            if !is_text_table(&b) {
                return None;
            }
            match parse(&b) {
                Ok(s) => Some(Table { id: e.id.clone(), off: e.off, strings: s }),
                Err(_) => {
                    errors.fetch_add(1, Ordering::Relaxed);
                    None
                }
            }
        })
        .collect();
    tables.sort_by_key(|t| t.off);
    Ok((tables, errors.load(Ordering::Relaxed)))
}

/// Write `<out>/text/<archive>/<offset>.json` ({hash: string}) per table
/// plus `index.jsonl` (id, file, strings, first string). Returns
/// (tables, strings).
pub fn export(a: &crate::game::ArchiveInfo, tables: &[Table], out: &std::path::Path) -> Result<(usize, usize)> {
    let dir = out.join("text").join(crate::game::index_stem(&a.id));
    std::fs::create_dir_all(&dir)?;
    let mut idx = String::new();
    let mut n = 0;
    for t in tables {
        let m: serde_json::Map<String, serde_json::Value> = t.strings.iter().map(|(h, s)| (format!("{h:08x}"), s.clone().into())).collect();
        let file = format!("{:x}.json", t.off);
        std::fs::write(dir.join(&file), serde_json::to_vec_pretty(&serde_json::json!({"id": t.id, "key": "joaat(lowercase label)", "strings": m}))?)?;
        let first = t.strings.values().find(|s| !s.is_empty()).map(|s| plain(s)).unwrap_or_default();
        let first: String = first.chars().take(80).collect();
        idx.push_str(&serde_json::json!({"id": t.id, "file": file, "strings": t.strings.len(), "first": first}).to_string());
        idx.push('\n');
        n += t.strings.len();
    }
    std::fs::write(dir.join("index.jsonl"), idx)?;
    Ok((tables.len(), n))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plain_strips_markup() {
        assert_eq!(plain("~sl:0:3.15~I said you'll need gears.~sl:0.01~Find them."), "I said you'll need gears. Find them.");
        assert_eq!(plain("~b~AUTO SHOTGUN~b~"), "AUTO SHOTGUN");
        assert_eq!(plain("a ~n~b"), "a b");
    }
}
