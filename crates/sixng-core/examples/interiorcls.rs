//! Research: which placements are interior (MLO) entities, by file kind and LOD level.
//! interiorcls <game> <out jsonl>
use rayon::prelude::*;
use sixng_core::meta7a;
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ws = sixng_core::export::Workspace::new(&a[1], "work");
    let ar = ws.resolve("levels")?;
    let ix = ws.load_index(&ar)?;
    let names = HashMap::new();
    let rows: Vec<(String, serde_json::Value, Vec<(String, String)>)> = ix
        .entries
        .par_iter()
        .filter(|e| e.sub == "7a")
        .filter_map(|e| {
            let r = sixng_core::resource::Resource::load_sys(&ar.path, e).ok()?;
            let mf = meta7a::map_file(&r, &names)?;
            if mf.entities.is_empty() {
                return None;
            }
            let m = meta7a::Meta::parse(&r)?;
            let mlos = meta7a::mlos(&m);
            let has_map = !m.objects(meta7a::MAPDATA).is_empty();
            let group = !m.objects(0x35ab_c248).is_empty();
            let mut owner: HashMap<usize, (u32, Option<u32>, Option<u32>)> = HashMap::new();
            for x in &mlos {
                for &i in &x.entities {
                    owner.entry(i).or_insert((x.name, None, None));
                }
                for (rn, es) in &x.rooms {
                    for &i in es {
                        if let Some(o) = owner.get_mut(&i) {
                            o.1.get_or_insert(*rn);
                        }
                    }
                }
                for (sn, es) in &x.sets {
                    for &i in es {
                        owner.entry(i).or_insert((x.name, None, None)).2 = Some(*sn);
                    }
                }
            }
            let kind = if !mlos.is_empty() { if has_map { "map+interior" } else { "interior" } } else if group { "group" } else if has_map { "map" } else { "other" };
            let lv = |l: &serde_json::Value| l.get("level").and_then(|x| x.as_str()).unwrap_or("?").to_string();
            let mut tally = Vec::new();
            let mut ents = Vec::new();
            for (i, en) in mf.entities.iter().enumerate() {
                let l = lv(&serde_json::to_value(&en.lod).unwrap_or_default());
                let cls = match owner.get(&i) { Some((_, Some(_), _)) => "room", Some((_, _, Some(_))) => "set", Some(_) => "mlo", None => "none" };
                tally.push((format!("{kind}/{cls}"), l));
                if let Some((n, rm, st)) = owner.get(&i) {
                    ents.push(serde_json::json!({"index": i, "interior": format!("{n:08x}"), "room": rm.map(|x| format!("{x:08x}")), "entity_set": st.map(|x| format!("{x:08x}"))}));
                }
            }
            let rooms: Vec<_> = mlos.iter().map(|x| serde_json::json!({"name": format!("{:08x}", x.name), "entities": x.entities.len(), "rooms": x.rooms.iter().map(|r| (format!("{:08x}", r.0), r.1.len())).collect::<Vec<_>>(), "sets": x.sets.iter().map(|s| (format!("{:08x}", s.0), s.1.len())).collect::<Vec<_>>()})).collect();
            Some((e.id.clone(), serde_json::json!({"map": e.id, "kind": kind, "entities": mf.entities.len(), "interiors": rooms, "interior_entities": ents}), tally))
        })
        .collect();
    let mut w = std::io::BufWriter::new(std::fs::File::create(&a[2])?);
    let mut t: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (_, v, tl) in &rows {
        writeln!(w, "{}", v)?;
        for k in tl {
            *t.entry(k.clone()).or_default() += 1;
        }
    }
    for ((k, l), n) in t {
        println!("{k:30} {l:10} {n}");
    }
    Ok(())
}
