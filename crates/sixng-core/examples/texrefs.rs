//! Research: dump shader texture-ref structs and embedded texture names.
//! texrefs <archive path> <index json> <name regex> [max]
use sixng_core::{resource::Resource, scan, texture, drawable};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let re = regex::Regex::new(&a[3])?;
    let max: usize = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(3);
    let mut n = 0;
    for e in ix.entries.iter().filter(|e| e.name.as_deref().map_or(false, |s| re.is_match(s))) {
        if n >= max { break; }
        let Ok(r) = Resource::load_sys(std::path::Path::new(&a[1]), e) else { continue };
        n += 1;
        println!("== {} {:?}", e.id, e.name);
        for t in texture::find_textures(&r) { println!("  emb {:08x} {}", sixng_core::meta7a::joaat(&t.name), t.name); }
        for s in drawable::find_shaders(&r) {
            let Some(pt) = r.sys_off(r.u64(s.record_off as usize + 0x20)) else { continue };
            let _ = pt;
            let ta = r.sys_off(r.u64(s.record_off as usize + 0x10));
            for (i, ph) in s.param_hashes.iter().enumerate() {
                let Some(ta) = ta else { break };
                let Some(t) = r.sys_off(r.u64(ta + 8 * i)) else { continue };
                let Some(tr) = r.sys_off(r.u64(t + 0x30)) else { continue };
                let b: Vec<String> = (0..0x40).step_by(8).map(|k| format!("{:016x}", r.u64(tr + k))).collect();
                let nm = r.sys_off(r.u64(tr)).map(|o| r.sys[o.saturating_sub(5)..(o+40).min(r.sys.len())].iter().map(|&c| if c.is_ascii_graphic() {c as char} else {'.'}).collect::<String>()).unwrap_or_default();
                println!("  sh {} p{} {} tr: {} [{}]", s.name_hash, i, ph, b.join(" "), nm);
            }
        }
    }
    Ok(())
}
