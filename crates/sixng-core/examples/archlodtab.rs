//! Research: every archetype definition's lodDist (+0x38), hd distance
//! candidate (+0x70), bsRadius (+0x64), flags (+0x68) as jsonl.
//! archlodtab <archive path> <index json> <out jsonl>
use rayon::prelude::*;
use sixng_core::{meta7a, resource::Resource, scan};
use std::io::Write;
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let rows: Vec<String> = ix
        .entries
        .par_iter()
        .filter(|e| e.sub == "7a")
        .flat_map_iter(|e| {
            let mut out = Vec::new();
            if let Ok(r) = Resource::load_sys(std::path::Path::new(&a[1]), e) {
                if let Some(m) = meta7a::Meta::parse(&r) {
                    for o in m.objects(sixng_core::export::ARCHETYPE_DEF) {
                        out.push(format!(
                            "{{\"a\":\"{:08x}\",\"lod\":{},\"hd\":{},\"r\":{},\"flags\":{},\"f6c\":{},\"f3c\":{},\"c\":[{},{},{}],\"file\":\"{}\"}}",
                            r.u32(o + 0x74),
                            r.f32(o + 0x38),
                            r.f32(o + 0x70),
                            r.f32(o + 0x64),
                            r.u32(o + 0x68),
                            r.u32(o + 0x6c),
                            r.u32(o + 0x3c),
                            r.f32(o + 0x58),
                            r.f32(o + 0x5c),
                            r.f32(o + 0x60),
                            e.id
                        ));
                    }
                }
            }
            out
        })
        .collect();
    let mut f = std::io::BufWriter::new(std::fs::File::create(&a[3])?);
    for l in rows {
        writeln!(f, "{l}")?;
    }
    Ok(())
}
