//! Research: archetype definitions per meta resource (load_sys).
//! defcount <archive path> <index json>
use rayon::prelude::*;
use sixng_core::{resource::Resource, scan};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let es: Vec<_> = ix.entries.iter().filter(|e| e.sub == "7a").collect();
    let v: Vec<(String, usize, usize)> = es
        .par_iter()
        .filter_map(|e| {
            let r = Resource::load_sys(std::path::Path::new(&a[1]), e).ok()?;
            let m = sixng_core::meta7a::Meta::parse(&r)?;
            Some((e.id.clone(), m.objects(sixng_core::export::ARCHETYPE_DEF).len(), r.sys.len()))
        })
        .collect();
    for (id, n, l) in v {
        println!("{id} {n} {l}");
    }
    Ok(())
}
