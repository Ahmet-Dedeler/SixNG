//! Research: collect struct and member hashes of meta (0x7a) resources.
//! metahashes <archive> <index json> <every nth> <out.txt>
use rayon::prelude::*;
use sixng_core::{meta7a::Meta, resource::Resource, scan};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let nth: usize = a[3].parse()?;
    let es: Vec<_> = ix.entries.iter().filter(|e| e.sub == "7a").step_by(nth).collect();
    let v: Vec<Vec<(u32, u32, u8)>> = es
        .par_iter()
        .filter_map(|e| {
            let r = Resource::load_sys(std::path::Path::new(&a[1]), e).ok()?;
            let m = Meta::parse(&r)?;
            Some(m.structs.values().flat_map(|s| std::iter::once((s.hash, 0, 0)).chain(s.members.iter().map(move |mm| (s.hash, mm.hash, mm.ty)))).collect())
        })
        .collect();
    let mut set = std::collections::BTreeSet::new();
    for x in v.into_iter().flatten() {
        set.insert(x);
    }
    let out: Vec<String> = set.iter().map(|(s, m, t)| if *m == 0 { format!("S {s:08x}") } else { format!("M {m:08x} {s:08x} {t}") }).collect();
    std::fs::write(&a[4], out.join("\n"))?;
    println!("{} from {} files", set.len(), es.len());
    Ok(())
}
