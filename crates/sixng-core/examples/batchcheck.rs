//! Research: prop batches whose decoded instances fall outside the batch bbox.
//! batchcheck <game>
use rayon::prelude::*;
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ws = sixng_core::export::Workspace::new(&a[1], "work");
    let ar = ws.resolve("levels")?;
    let ix = ws.load_index(&ar)?;
    let r: Vec<(String, usize, usize, usize, usize)> = ix.entries.par_iter().filter(|e| e.sub == "7a").filter_map(|e| {
        let rs = sixng_core::resource::Resource::load_sys(&ar.path, e).ok()?;
        let m = sixng_core::meta7a::Meta::parse(&rs)?;
        if m.objects(sixng_core::batches::PROP_BATCH).is_empty() { return None; }
        let rm = sixng_core::resource::Resource::load_meta(&ar.path, e).ok()?;
        let mm = sixng_core::meta7a::Meta::parse(&rm)?;
        let bs = sixng_core::batches::prop_batches(&rm, &mm);
        let mut bad = 0; let mut inst = 0;
        for b in &bs { inst += b.instances.len(); let lo = b.bbox[0]; let hi = b.bbox[1];
            let _ = (lo, hi); if b.instances.iter().any(|i| (0..3).any(|k| !i.p[k].is_finite() || i.p[k].abs() > 20000.0 || (i.p[k] != 0.0 && i.p[k].abs() < 1e-6))) { bad += 1; } }
        Some((e.id.clone(), bs.len(), bad, inst, rm.sys.len()))
    }).collect();
    let files = r.len(); let badf: Vec<_> = r.iter().filter(|x| x.2 > 0).collect();
    let nb: usize = r.iter().map(|x| x.1).sum(); let bb: usize = r.iter().map(|x| x.2).sum();
    println!("files {files} bad files {} batches {nb} bad batches {bb}", badf.len());
    for x in badf.iter().take(8) { println!("{:?}", x); }
    Ok(())
}
