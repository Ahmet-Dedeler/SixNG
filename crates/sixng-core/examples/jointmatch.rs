//! Research: guess which bones the out-of-skeleton joint indices of a ped
//! clothing mesh address. For every joint index, take the weight-averaged
//! vertex position and list the nearest bone pivots of a reference skeleton
//! (and, for in-range indices, the distance to the file's own bone pivot).
//! Usage: jointmatch <archive> <index json> <clothing id> <skeleton id> [geometry]
use sixng_core::{drawable, resource::Resource, scan};

fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let archive = std::path::Path::new(&a[1]);
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let get = |id: &str| ix.entries.iter().find(|e| e.id == id).expect("entry");
    let gi: usize = a.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
    let r = Resource::load(archive, get(&a[3]))?;
    let info = drawable::describe(&r);
    let own = drawable::find_skeleton(&r);
    let rs = Resource::load(archive, get(&a[4]))?;
    let refsk = drawable::find_skeleton(&rs);
    let g = &info.geometries[gi];
    let b = &info.buffers[g.vertex_buffer];
    let slice = |b: &drawable::GfxBuffer| &r.gfx[b.gfx_off as usize..(b.gfx_off + b.bytes) as usize];
    let m = drawable::decode_mesh(slice(b), b.mesh_blob.as_ref().unwrap(), slice(&info.buffers[g.index_buffer]));
    let (Some(j0), Some(w0)) = (&m.joints, &m.weights) else { anyhow::bail!("not skinned") };
    let mut acc: std::collections::BTreeMap<u16, ([f64; 3], f64)> = Default::default();
    for (i, p) in m.positions.iter().enumerate() {
        let mut add = |j: u16, w: u8| {
            if w == 0 {
                return;
            }
            let e = acc.entry(j).or_default();
            for k in 0..3 {
                e.0[k] += p[k] as f64 * w as f64;
            }
            e.1 += w as f64;
        };
        let set: u32 = std::env::var("SET").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
        if set & 1 != 0 {
            for k in 0..4 {
                add(j0[i][k], w0[i][k]);
            }
        }
        if let (Some(j1), Some(w1), true) = (&m.joints1, &m.weights1, set & 2 != 0) {
            for k in 0..4 {
                add(j1[i][k], w1[i][k]);
            }
        }
    }
    let piv = |b: &drawable::Bone| b.pivot.unwrap_or([f32::NAN; 3]);
    let dist = |p: [f64; 3], q: [f32; 3]| ((p[0] - q[0] as f64).powi(2) + (p[1] - q[1] as f64).powi(2) + (p[2] - q[2] as f64).powi(2)).sqrt();
    println!("own bones {} (binds {}), reference bones {} (binds {})", own.len(), own.iter().filter(|b| b.pivot.is_some()).count(), refsk.len(), refsk.iter().filter(|b| b.pivot.is_some()).count());
    for (j, (s, w)) in &acc {
        let c = [s[0] / w, s[1] / w, s[2] / w];
        let mut near: Vec<(f64, &str)> = refsk.iter().filter(|b| b.pivot.is_some()).map(|b| (dist(c, piv(b)), b.name.as_str())).collect();
        near.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        let ownd = own.get(*j as usize).map(|b| format!("own {} d={:.3}", b.name, dist(c, piv(b)))).unwrap_or_default();
        println!(
            "{j:3} w={:7.0} c=({:+.3},{:+.3},{:+.3}) {ownd} | ref: {}",
            w,
            c[0],
            c[1],
            c[2],
            near.iter().take(3).map(|(d, n)| format!("{n} {d:.3}")).collect::<Vec<_>>().join(", ")
        );
    }
    Ok(())
}
