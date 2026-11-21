//! Research: TXD key hashes (+0x20) next to texture names.
//! txdkeys <archive path> <index json> [max]
use sixng_core::{resource::Resource, scan, texture};
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let max: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    let mut n = 0;
    for e in ix.entries.iter().filter(|e| e.rtype.as_deref() == Some("txd")) {
        if n >= max { break; }
        let Ok(r) = Resource::load_sys(std::path::Path::new(&a[1]), e) else { continue };
        let cnt = r.u16(0x28) as usize;
        let (Some(kp), Some(tp)) = (r.sys_off(r.u64(0x20)), r.sys_off(r.u64(0x30))) else { continue };
        for i in 0..cnt {
            let key = r.u32(kp + 4 * i);
            let name = r.sys_off(r.u64(tp + 8 * i)).and_then(|p| r.sys_off(r.u64(p + 0x30))).and_then(|o| r.cstr(o + 8)).unwrap_or_default();
            println!("{key:08x}\t{name}");
        }
        n += 1;
    }
    let _ = texture::dxgi_name(0);
    Ok(())
}
