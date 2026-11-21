//! Debug: print the stream table and sys/gfx sizes of entries.
//! cargo run --release --example streams -- <archive path> <index json> <entry id> [out prefix]
use sixng_core::{resource::Resource, scan};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let e = ix.entries.iter().find(|e| e.id == a[3]).expect("entry");
    println!("streams {:?}", e.streams);
    let r = Resource::load(std::path::Path::new(&a[1]), e)?;
    println!("sys {} gfx {}", r.sys.len(), r.gfx.len());
    if let Some(p) = a.get(4) {
        std::fs::write(format!("{p}.sys"), &r.sys)?;
        std::fs::write(format!("{p}.gfx"), &r.gfx)?;
    }
    Ok(())
}
