//! Debug: dump every mip of every texture in one TXD entry as PNG.
//! cargo run --release --example mips -- <archive path> <index json> <entry id> <out dir>
use sixng_core::{resource::Resource, scan, texture};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let e = ix.entries.iter().find(|e| e.id == a[3]).expect("entry");
    let r = Resource::load(std::path::Path::new(&a[1]), e)?;
    std::fs::create_dir_all(&a[4])?;
    for t in texture::find_textures(&r).iter().take(12) {
        for m in 0..t.mips {
            if let Ok((fmt, lin, w, h)) = texture::mip_linear(&r, t, m) {
                if let Ok(rgba) = texture::to_rgba(fmt, &lin, w, h) {
                    texture::save_png(&std::path::Path::new(&a[4]).join(format!("{}_m{}.png", t.name, m)), &rgba, w, h)?;
                }
            }
        }
    }
    Ok(())
}
