//! Research: search every resource of an indexed archive (system and
//! graphics pages, decompressed one entry at a time) for byte needles:
//! ASCII strings and the little-endian joaat of each string (lower case).
//! Usage: needles <archive path> <index json> <needle> [needle ...]
//! A needle `0x1234abcd` is searched as that raw little-endian u32 only;
//! 16 hex digits (`0x0123456789abcdef`) as a raw little-endian u64.
//! Prints one JSON line per entry with hits.
use rayon::prelude::*;
use sixng_core::{resource::Resource, scan};

fn joaat(s: &str) -> u32 {
    let mut h: u32 = 0;
    for c in s.to_ascii_lowercase().bytes() {
        h = h.wrapping_add(c as u32);
        h = h.wrapping_add(h << 10);
        h ^= h >> 6;
    }
    h = h.wrapping_add(h << 3);
    h ^= h >> 11;
    h.wrapping_add(h << 15)
}

fn find_all(hay: &[u8], n: &[u8]) -> Vec<usize> {
    if n.is_empty() || hay.len() < n.len() {
        return Vec::new();
    }
    hay.windows(n.len()).enumerate().filter(|(_, w)| *w == n).map(|(i, _)| i).take(8).collect()
}

fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let archive = std::path::PathBuf::from(&a[1]);
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let mut needles: Vec<(String, Vec<u8>)> = Vec::new();
    for s in &a[3..] {
        if let Some(h) = s.strip_prefix("0x").filter(|h| h.len() == 16).and_then(|h| u64::from_str_radix(h, 16).ok()) {
            needles.push((format!("u64:{h:016x}"), h.to_le_bytes().to_vec()));
            continue;
        }
        if let Some(h) = s.strip_prefix("0x").and_then(|h| u32::from_str_radix(h, 16).ok()) {
            needles.push((format!("u32:{h:08x}"), h.to_le_bytes().to_vec()));
            continue;
        }
        needles.push((format!("str:{s}"), s.as_bytes().to_vec()));
        needles.push((format!("joaat:{s}={:08x}", joaat(s)), joaat(s).to_le_bytes().to_vec()));
    }
    let done = std::sync::atomic::AtomicUsize::new(0);
    let n = ix.entries.len();
    ix.entries.par_iter().for_each(|e| {
        let k = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if k % 5000 == 0 {
            eprintln!("{k}/{n}");
        }
        let Ok(r) = Resource::load(&archive, e) else { return };
        let mut hits = Vec::new();
        for (label, nd) in &needles {
            for (page, buf) in [("sys", &r.sys), ("gfx", &r.gfx)] {
                let f = find_all(buf, nd);
                if !f.is_empty() {
                    hits.push(serde_json::json!({"needle": label, "page": page, "at": f}));
                }
            }
        }
        if !hits.is_empty() {
            println!("{}", serde_json::json!({"id": e.id, "name": e.name, "kind": e.kind, "sub": e.sub, "usize": e.usize, "hits": hits}));
        }
    });
    Ok(())
}
