//! Research: count entries whose decompressed bytes contain the u32 joaat
//! of several names (vehicle models). hashgrep <game> <archive> <kinds,> <name>...
use rayon::prelude::*;
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ws = sixng_core::export::Workspace::new(&a[1], "work");
    let ar = ws.resolve(&a[2])?;
    let ix = ws.load_index(&ar)?;
    let kinds: Vec<&str> = a[3].split(',').collect();
    let pats: Vec<(String, [u8; 4])> = a[4..].iter().map(|n| (n.clone(), sixng_core::meta7a::joaat(n).to_le_bytes())).collect();
    let mut hits: Vec<(String, String, Vec<usize>, u64)> = ix.entries.par_iter()
        .filter(|e| kinds.iter().any(|k| format!("{:?}", e.kind).eq_ignore_ascii_case(k) || (*k == "7a" && e.sub == "7a")) && e.usize < 64 << 20)
        .filter_map(|e| {
            let d = sixng_core::scan::read_entry(&ar.path, e).ok()?;
            let c: Vec<usize> = pats.iter().map(|(_, p)| d.windows(4).filter(|w| w == p).count()).collect();
            (c.iter().filter(|&&x| x > 0).count() * 2 > pats.len()).then(|| (e.id.clone(), format!("{:?}/{}", e.kind, e.sub), c, e.usize))
        }).collect();
    hits.sort_by_key(|h| std::cmp::Reverse(h.2.iter().sum::<usize>()));
    for h in hits.iter().take(40) { println!("{} {} {:?} {}", h.0, h.1, h.2, h.3); }
    println!("{} entries", hits.len());
    Ok(())
}
