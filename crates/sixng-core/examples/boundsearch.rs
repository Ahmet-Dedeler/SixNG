//! Research: find drawables whose stored bounds match a size (any axis order).
//! boundsearch <archive path> <index json> <sx> <sy> <sz> [tolerance m]
//! Reads only each candidate's system stream header area (load_sys).
use rayon::prelude::*;
use sixng_core::classify::Kind;
use sixng_core::scan;
use sixng_core::resource::Resource;
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let ix = scan::load_index(std::path::Path::new(&a[2]))?;
    let mut want: Vec<f32> = a[3..6].iter().map(|s| s.parse().unwrap()).collect();
    want.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let tol: f32 = a.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.3);
    let es: Vec<_> = ix.entries.iter().filter(|e| e.kind == Kind::RageObj).collect();
    eprintln!("{} drawable-like entries", es.len());
    let mut hits: Vec<(f32, String, String, [[f32; 3]; 2])> = es
        .par_iter()
        .filter_map(|e| {
            let r = Resource::load_sys(std::path::Path::new(&a[1]), e).ok()?;
            if r.sys.len() < 0x4c {
                return None;
            }
            let b = sixng_core::drawable::drawable_bounds(&r)?;
            let mut s: Vec<f32> = (0..3).map(|k| b[1][k] - b[0][k]).collect();
            s.sort_by(|x, y| x.partial_cmp(y).unwrap());
            let err = s.iter().zip(&want).map(|(p, q)| (p - q).abs()).fold(0.0f32, f32::max);
            (err <= tol).then(|| (err, e.id.clone(), e.name.clone().unwrap_or_default(), b))
        })
        .collect();
    hits.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
    for (err, id, name, b) in hits.iter().take(40) {
        println!("{err:.3} {id} {name} {:?}", b);
    }
    eprintln!("{} hits within {tol} m", hits.len());
    Ok(())
}
