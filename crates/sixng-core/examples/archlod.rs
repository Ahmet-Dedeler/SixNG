//! Research: archetype-definition members and sample scalars (lodDist hunt).
//! archlod <archive path> <index json> <n files>
use sixng_core::{meta7a, resource::Resource, scan};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let n: usize = a[3].parse()?;
    for w in ["lodDist", "hdTextureDist", "LodDist", "lod_dist", "childLodDist", "bbMin", "bbMax", "bsCentre", "bsRadius", "flags", "specialAttribute", "name", "assetName", "assetType", "textureDictionary", "drawableDictionary", "physicsDictionary", "clipDictionary", "extensions", "parentIndex", "lodLevel", "numChildren", "priorityLevel"] {
        println!("{w} {:08x}", meta7a::joaat(w));
    }
    let mut shown = 0;
    for e in ix.entries.iter().filter(|e| e.sub == "7a") {
        let Ok(r) = Resource::load_sys(std::path::Path::new(&a[1]), e) else { continue };
        let Some(m) = meta7a::Meta::parse(&r) else { continue };
        let os = m.objects(sixng_core::export::ARCHETYPE_DEF);
        if os.is_empty() {
            continue;
        }
        if shown == 0 {
            let sd = &m.structs[&sixng_core::export::ARCHETYPE_DEF];
            for mb in &sd.members {
                println!("member {:08x} off {:#x} ty {:02x} sub {:02x} extra {:08x}", mb.hash, mb.off, mb.ty, mb.sub, mb.extra);
            }
        }
        for &o in os.iter().take(3) {
            println!("{} {:?}", e.id, m.scalars(sixng_core::export::ARCHETYPE_DEF, o, &[]));
        }
        shown += 1;
        if shown >= n {
            break;
        }
    }
    Ok(())
}
