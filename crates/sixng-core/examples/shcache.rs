//! Research: harvest name-hash records from shaders.rpf, streamed.
//! Variables: {u32 h, u32 h, 0x30 ..}; textures: {u32 a, u32 b, 02 01 xx xx, 00 80 00 00}.
//! shcache <shaders.rpf> <out.json>
use std::io::Read;
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let mut f = std::fs::File::open(&a[1])?;
    let mut var = std::collections::BTreeMap::<u32, u32>::new();
    let mut tex = std::collections::BTreeMap::<(u32, u32), u32>::new();
    let step = 1 << 22;
    let mut buf = vec![0u8; step + 32];
    let mut carry = 0usize;
    loop {
        let n = f.read(&mut buf[carry..carry + step])?;
        let len = carry + n;
        if len < 24 { break; }
        let u = |o: usize| u32::from_le_bytes(buf[o..o + 4].try_into().unwrap());
        for o in 0..len - 23 {
            let x = u(o);
            if x == 0 || x == u32::MAX { continue; }
            if u(o + 4) == x && buf[o + 8] == 0x30 {
                *var.entry(x).or_default() += 1;
            } else if buf[o + 8] == 2 && buf[o + 9] == 1 && u(o + 12) == 0x8000 {
                *tex.entry((x, u(o + 4))).or_default() += 1;
            }
        }
        if n == 0 { break; }
        buf.copy_within(len - 23..len, 0);
        carry = 23;
    }
    let j = serde_json::json!({
        "var": var.iter().map(|(h, c)| format!("{h:08x} {c}")).collect::<Vec<_>>(),
        "tex": tex.iter().map(|((x, y), c)| format!("{x:08x} {y:08x} {c}")).collect::<Vec<_>>(),
    });
    std::fs::write(&a[2], serde_json::to_string_pretty(&j)?)?;
    println!("{} vars, {} tex pairs", var.len(), tex.len());
    Ok(())
}
