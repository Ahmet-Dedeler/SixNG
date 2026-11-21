//! Research: print archetype definitions (bbox, flags) for given names.
//! archdef <archive path> <index json> <name>...
use rayon::prelude::*;
use sixng_core::{resource::Resource, scan};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let want: Vec<(u32, String)> = a[3..].iter().map(|n| (sixng_core::meta7a::joaat(n), n.clone())).collect();
    ix.entries.par_iter().filter(|e| e.sub == "7a").for_each(|e| {
        let Ok(r) = Resource::load_sys(std::path::Path::new(&a[1]), e) else { return };
        let Some(m) = sixng_core::meta7a::Meta::parse(&r) else { return };
        for o in m.objects(sixng_core::export::ARCHETYPE_DEF) {
            let h = r.u32(o + 0x74);
            if let Some((_, n)) = want.iter().find(|w| w.0 == h) {
                let v3 = |x: usize| [r.f32(x), r.f32(x + 4), r.f32(x + 8)];
                println!("{n} {} min {:?} max {:?} centre {:?} r {}", e.id, v3(o + 0x40), v3(o + 0x4C), v3(o + 0x58), r.f32(o + 0x64));
            }
        }
    });
    Ok(())
}
