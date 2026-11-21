//! Research: list enums (hash, value count, sample values) in every meta
//! resource of an archive. enumscan <archive path> <index json> [min values]
use sixng_core::{resource::Resource, scan};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let min: usize = a.get(3).map(|x| x.parse().unwrap()).unwrap_or(50);
    let mut seen = std::collections::HashSet::new();
    for e in ix.entries.iter().filter(|e| e.sub == "7a") {
        let Ok(r) = Resource::load_sys(std::path::Path::new(&a[1]), e) else { continue };
        let ne = r.u16(0x4A) as usize;
        let Some(et) = r.sys_off(r.u64(0x30)) else { continue };
        for i in 0..ne {
            let o = et + 24 * i;
            let h = r.u32(o);
            let n = r.u32(o + 16) as usize;
            if n < min || !seen.insert(h) {
                continue;
            }
            let Some(vp) = r.sys_off(r.u64(o + 8)) else { continue };
            let vals: Vec<String> = (0..n.min(400)).map(|k| format!("{:x}:{:08x}", r.u32(vp + 16 * k), r.u32(vp + 16 * k + 8))).collect();
            println!("{} {:08x} {} {}", e.id, h, n, vals.join(","));
        }
    }
    Ok(())
}
