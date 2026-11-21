//! Research: test whether mesh-blob slots 7 / 9 are a second skin influence
//! set (weights / joint indices) next to slots 6 / 8. Prints weight-sum
//! statistics and which bones the second set references.
//! Usage: skinprobe <archive path> <index json> <entry id> [geometry]
use sixng_core::{drawable, resource::Resource, scan};
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let e = ix.entries.iter().find(|e| e.id == a[3]).expect("entry");
    let gi: usize = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
    let r = Resource::load(std::path::Path::new(&a[1]), e)?;
    let info = drawable::describe(&r);
    let skel = drawable::find_skeleton(&r);
    let g = &info.geometries[gi];
    let b = &info.buffers[g.vertex_buffer];
    let h = b.mesh_blob.as_ref().expect("mesh blob");
    let blob = &r.gfx[b.gfx_off as usize..(b.gfx_off + b.bytes) as usize];
    let v = drawable::decode_positions(blob, h).len();
    let st = |s: u32| h.sections.iter().find(|x| x.0 == s).map(|x| (x.1 as usize, x.2));
    println!("name {:?} bones {} verts {v} sections {:?}", info.name, skel.len(), h.sections);
    let quad = |s: u32| -> Option<Vec<[u8; 4]>> {
        let (o, _) = st(s)?;
        Some(blob.get(o..o + 4 * v)?.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect())
    };
    let (Some(w6), Some(j8)) = (quad(6), quad(8)) else {
        println!("no slot 6/8");
        return Ok(());
    };
    let w7 = quad(7);
    let j9 = quad(9);
    let sum = |w: &[u8; 4]| w.iter().map(|&x| x as u32).sum::<u32>();
    let mut hist6: BTreeMap<u32, usize> = BTreeMap::new();
    let mut hist67: BTreeMap<u32, usize> = BTreeMap::new();
    let mut used7 = 0;
    let mut bones2: BTreeMap<u8, (usize, u32)> = BTreeMap::new();
    let mut bones1: BTreeMap<u8, usize> = BTreeMap::new();
    let mut sorted_ok = 0;
    for i in 0..v {
        *hist6.entry(sum(&w6[i])).or_default() += 1;
        for k in 0..4 {
            if w6[i][k] > 0 {
                *bones1.entry(j8[i][k]).or_default() += 1;
            }
        }
        if let (Some(w7), Some(j9)) = (&w7, &j9) {
            *hist67.entry(sum(&w6[i]) + sum(&w7[i])).or_default() += 1;
            if sum(&w7[i]) > 0 {
                used7 += 1;
            }
            // weights sorted descending across the 8 slots?
            let all: Vec<u8> = w6[i].iter().chain(w7[i].iter()).copied().collect();
            if all.windows(2).all(|p| p[0] >= p[1]) {
                sorted_ok += 1;
            }
            for k in 0..4 {
                if w7[i][k] > 0 {
                    let e = bones2.entry(j9[i][k]).or_default();
                    e.0 += 1;
                    e.1 += w7[i][k] as u32;
                }
            }
        }
    }
    let top = |m: &BTreeMap<u32, usize>| {
        let mut x: Vec<_> = m.iter().map(|(a, b)| (*b, *a)).collect();
        x.sort_by(|a, b| b.cmp(a));
        x.into_iter().take(6).map(|(n, s)| format!("{s}:{n}")).collect::<Vec<_>>().join(" ")
    };
    println!("sum(w6) top: {}", top(&hist6));
    println!("sum(w6)+sum(w7) top: {}", top(&hist67));
    println!("vertices using w7: {used7} / {v}; 8 weights sorted desc: {sorted_ok}");
    let nm = |j: u8| skel.get(j as usize).map(|b| b.name.as_str()).unwrap_or("?");
    let gm1 = bones1.iter().filter(|(j, _)| nm(**j).starts_with("GM_")).count();
    println!("set1 bones: {} (GM_ {gm1})", bones1.len());
    println!("set1 max index {:?}; skeleton[..5] {:?} last {:?}", bones1.keys().max(), skel.iter().take(5).map(|b| (&b.name, b.parent)).collect::<Vec<_>>(), skel.last().map(|b| &b.name));
    println!("set1 out of range: {:?}", bones1.iter().filter(|(j, _)| **j as usize >= skel.len()).collect::<Vec<_>>());
    if std::env::var_os("ALL").is_none() { return Ok(()); }
    println!("set2 bones (count, weight sum):");
    for (j, (n, s)) in &bones2 {
        println!("  {j:3} {:40} {n:6} {s}", nm(*j));
    }
    Ok(())
}
