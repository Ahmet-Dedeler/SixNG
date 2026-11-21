//! Research: decode a PSO file (raw file path) to JSON: root plus every
//! block's objects by struct (64-bit struct hash keys).
//! psodump <file.pso> <out.json>
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let d = std::fs::read(&a[1])?;
    let p = sixng_core::pso::parse(&d).ok_or_else(|| anyhow::anyhow!("not pso"))?;
    eprintln!("structs {} blocks {} root {:?}", p.structs.len(), p.blocks.len(), p.root);
    let m = p.meta();
    let mut objs = serde_json::Map::new();
    let mut hs: Vec<u32> = p.blocks.iter().map(|b| b.0).collect();
    hs.sort();
    hs.dedup();
    for h in hs {
        let v: Vec<_> = m.objects(h).iter().map(|&o| m.object(h, o, 0).unwrap_or_default()).collect();
        let k = p.struct64.get(&h).map(|x| format!("{x:016x}")).unwrap_or(format!("type_{h:08x}"));
        if !v.is_empty() {
            objs.insert(k, serde_json::Value::Array(v));
        }
    }
    let blocks: Vec<_> = p.blocks.iter().map(|b| serde_json::json!([format!("{:08x}", b.0), b.1])).collect();
    let doc = serde_json::json!({"root": p.root_json(0), "blocks": blocks, "objects": objs});
    std::fs::write(&a[2], serde_json::to_vec_pretty(&doc)?)?;
    Ok(())
}
