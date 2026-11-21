//! Research: schema + full dump of one map meta file.
//! mapdump <archive path> <index json> <entry id> <out json>
use sixng_core::{meta7a, resource::Resource, scan};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let e = ix.entries.iter().find(|e| e.id == a[3]).ok_or_else(|| anyhow::anyhow!("no entry"))?;
    let r = Resource::load_sys(std::path::Path::new(&a[1]), e)?;
    let m = meta7a::Meta::parse(&r).ok_or_else(|| anyhow::anyhow!("not meta"))?;
    let mut objs = serde_json::Map::new();
    for (&h, _) in &m.structs {
        let v: Vec<_> = m.objects(h).iter().map(|&o| m.object(h, o, 3).unwrap_or_default()).collect();
        if !v.is_empty() {
            objs.insert(format!("{h:08x}"), serde_json::Value::Array(v));
        }
    }
    let doc = serde_json::json!({"schema": m.schema(), "objects": objs});
    std::fs::write(&a[4], serde_json::to_vec_pretty(&doc)?)?;
    Ok(())
}
