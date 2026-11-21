//! Research: how often drawables have models outside the main LOD group.
//! lodcensus <archive path> <index json> <every-nth> [name regex]
use rayon::prelude::*;
use sixng_core::{drawable, resource::Resource, scan};
use std::sync::Mutex;
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let nth: usize = a[3].parse()?;
    let re = a.get(4).map(|s| regex::Regex::new(s).unwrap());
    let sel: Vec<_> = ix
        .entries
        .iter()
        .filter(|e| matches!(e.rtype.as_deref(), Some("dr") | Some("ft")))
        .filter(|e| re.as_ref().map_or(true, |r| r.is_match(e.name.as_deref().unwrap_or(""))))
        .enumerate()
        .filter(|(i, _)| i % nth == 0)
        .map(|(_, e)| e)
        .collect();
    let stats = Mutex::new(std::collections::BTreeMap::<String, [usize; 5]>::new());
    let ex = Mutex::new(Vec::new());
    sel.par_iter().for_each(|e| {
        let Ok(r) = Resource::load(std::path::Path::new(&a[1]), e) else { return };
        let info = drawable::describe(&r);
        if info.models.is_empty() {
            return;
        }
        let parts = info.models.iter().filter(|m| m.part).count();
        let nlod = info.models.iter().filter(|m| !m.part).map(|m| m.lod + 1).max().unwrap_or(0);
        let pg: usize = info.models.iter().filter(|m| m.part).map(|m| m.geometries.len()).sum();
        let mg: usize = info.models.iter().filter(|m| !m.part && m.lod == 0).map(|m| m.geometries.len()).sum();
        let k = e.rtype.clone().unwrap_or_default();
        let mut s = stats.lock().unwrap();
        let v = s.entry(k).or_default();
        v[0] += 1;
        v[1] += (parts > 0) as usize;
        v[2] += parts;
        v[3] += (nlod == 0) as usize;
        v[4] += (mg == 0) as usize;
        if parts > 0 {
            ex.lock().unwrap().push(format!("{} {:?} nlod {nlod} lod0 geoms {mg} parts {parts} ({pg} geoms)", e.id, e.name));
        }
    });
    println!("type: [drawables, with parts, parts, no main lod, empty lod0]");
    for (k, v) in stats.into_inner().unwrap() {
        println!("{k}: {v:?}");
    }
    let mut ex = ex.into_inner().unwrap();
    ex.sort();
    for l in ex.iter().take(40) {
        println!("{l}");
    }
    Ok(())
}
