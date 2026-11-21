//! Research: show the model-pointer arrays of drawables (LOD table layout).
//! lodtab <archive path> <index json> <entry id>...
use sixng_core::{drawable, resource::Resource, scan};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    for id in &a[3..] {
        let Some(e) = ix.entries.iter().find(|e| &e.id == id) else {
            println!("{id}: not found");
            continue;
        };
        let r = Resource::load(std::path::Path::new(&a[1]), e)?;
        let info = drawable::describe(&r);
        println!("== {id} {:?}", info.name);
        for (i, m) in info.models.iter().enumerate() {
            println!("model {i} @{:x} lod {} geoms {:?}", m.struct_off, m.lod, m.geometries);
        }
        let model_at: std::collections::HashMap<u64, usize> = info.models.iter().enumerate().map(|(i, m)| (0x5000_0000 | (m.struct_off - 8), i)).collect();
        // pointer arrays of models and who points at their 16-byte headers
        let mut o = 0;
        while o + 0x10 <= r.sys.len() {
            let n = r.u16(o + 8) as usize;
            if n > 0 && n == r.u16(o + 10) as usize {
                if let Some(arr) = r.sys_off(r.u64(o)) {
                    let ms: Option<Vec<usize>> = (0..n).map(|k| model_at.get(&r.u64(arr + 8 * k)).copied()).collect();
                    if let Some(ms) = ms {
                        let target = 0x5000_0000u64 | o as u64;
                        let refs: Vec<String> = (0..r.sys.len() / 8).filter(|&k| r.u64(k * 8) == target).map(|k| format!("{:x}", k * 8)).collect();
                        println!("array hdr @{o:x} -> {arr:x} n {n} models {ms:?} referenced from {refs:?}");
                        // walk up: who points into the 0x100 bytes before each ref
                        for rf in &refs {
                            let mut cur = usize::from_str_radix(rf, 16).unwrap();
                            for _ in 0..4 {
                                let hit = (0..r.sys.len() / 8).find_map(|k| {
                                    let v = r.u64(k * 8);
                                    if v >> 28 != 5 {
                                        return None;
                                    }
                                    let t = (v & 0x0fff_ffff) as usize;
                                    (t <= cur && cur - t < 0x200 && k * 8 != cur).then_some((k * 8, t))
                                });
                                let Some((from, t)) = hit else { break };
                                print!("   <- struct @{t:x} (+{:x}) from @{from:x}", cur - t);
                                cur = from;
                            }
                            println!();
                        }
                    }
                }
            }
            o += 8;
        }
        // root header as u64s
        let words: Vec<String> = (0..0x30).map(|k| format!("{:x}:{:x}", k * 8, r.u64(k * 8))).collect();
        println!("root {}", words.join(" "));
    }
    Ok(())
}
