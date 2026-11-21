//! Research: tile every PNG under a folder into one contact sheet (cell
//! 128 px, 16 columns, alpha over a checker) + a JSON list of cell -> file.
//! Usage: pngsheet <dir> <out.png> [cell px]
fn walk(d: &std::path::Path, v: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(d) else { return };
    let mut es: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    es.sort();
    for p in es {
        if p.is_dir() {
            walk(&p, v);
        } else if p.extension().is_some_and(|e| e == "png") {
            v.push(p);
        }
    }
}

fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let c: u32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(128);
    const COLS: u32 = 16;
    let mut files = Vec::new();
    walk(std::path::Path::new(&a[1]), &mut files);
    let rows = (files.len() as u32).div_ceil(COLS).max(1);
    let mut sheet = image::RgbaImage::from_pixel(COLS * c, rows * c, image::Rgba([40, 40, 48, 255]));
    let mut cells = Vec::new();
    for (k, f) in files.iter().enumerate() {
        let (x0, y0) = ((k as u32 % COLS) * c, (k as u32 / COLS) * c);
        let Ok(img) = image::open(f) else { continue };
        let img = img.to_rgba8();
        let (w, h) = img.dimensions();
        let sc = (c as f32 / w.max(h) as f32).min(1.0);
        let t = image::imageops::thumbnail(&img, ((w as f32 * sc) as u32).max(1), ((h as f32 * sc) as u32).max(1));
        for (x, y, p) in t.enumerate_pixels() {
            let bg = if ((x / 8) + (y / 8)) % 2 == 0 { 90u16 } else { 60 };
            let al = p[3] as u16;
            let mix = |v: u8| ((v as u16 * al + bg * (255 - al)) / 255) as u8;
            sheet.put_pixel(x0 + x, y0 + y, image::Rgba([mix(p[0]), mix(p[1]), mix(p[2]), 255]));
        }
        cells.push(serde_json::json!({"cell": k, "file": f, "w": w, "h": h}));
    }
    sheet.save(&a[2])?;
    std::fs::write(format!("{}.json", a[2]), serde_json::to_vec(&cells)?)?;
    println!("{} images", cells.len());
    Ok(())
}
