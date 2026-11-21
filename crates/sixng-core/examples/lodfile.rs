//! Research: what lod-links reads from given map files.
//! lodfile <game> <entry id>...
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ws = sixng_core::export::Workspace::new(&a[1], "work");
    let ar = ws.resolve("levels")?;
    let ix = ws.load_index(&ar)?;
    for id in &a[2..] {
        let e = ix.entries.iter().find(|e| &e.id == id).unwrap();
        match sixng_core::lodlinks::read_file(&ar, e) {
            None => println!("{id}: none"),
            Some(f) => {
                println!("{id}: ents {} props {} region {:x}", f.ents.len(), f.props.len(), f.region);
                for j in [25usize, 36, 40] {
                    if let Some(p) = f.props.get(j) { println!("  batch {j} arch {:08x} n {} first {:?}", p.0, p.1.len(), p.1.first()); }
                }
            }
        }
    }
    Ok(())
}
