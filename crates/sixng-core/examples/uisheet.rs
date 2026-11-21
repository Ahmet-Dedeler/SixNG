//! Research: contact sheet of a ttpf store. Decodes every `step`-th texture
//! at the mip nearest 128 px and tiles them into one PNG (cell 128 px,
//! 16 columns), plus a JSON list of cell -> hash.
//! Usage: uisheet <store path> <out.png> [step] [first] [min side px]
use sixng_core::ttpf::{decode_rgba_slice, Ttpf};

fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let st = Ttpf::open(std::path::Path::new(&a[1]))?;
    let step: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(40);
    let first: usize = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
    const C: u32 = 128;
    const COLS: u32 = 16;
    let min: u32 = a.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
    let pick: Vec<_> = st.textures.iter().filter(|t| t.width.max(t.height) >= min).skip(first).step_by(step).collect();
    let rows = (pick.len() as u32).div_ceil(COLS);
    let mut sheet = image::RgbaImage::from_pixel(COLS * C, rows * C, image::Rgba([40, 40, 48, 255]));
    let mut cells = Vec::new();
    for (k, t) in pick.iter().enumerate() {
        let (x0, y0) = ((k as u32 % COLS) * C, (k as u32 / COLS) * C);
        // UI textures often have one mip only: decode mip 0 and downscale.
        let res = st.data_ex(t).and_then(|(d, _)| decode_rgba_slice(t, &d, 0, 0));
        let ok = match res {
            Ok((rgba, w, h)) => {
                if let Some(img) = image::RgbaImage::from_raw(w, h, rgba) {
                    let sc = (C as f32 / w.max(h) as f32).min(1.0);
                    let img = image::imageops::thumbnail(&img, ((w as f32 * sc) as u32).max(1), ((h as f32 * sc) as u32).max(1));
                    for (x, y, p) in img.enumerate_pixels() {
                        if x < C && y < C {
                            // composite over a checker so alpha shows
                            let bg = if ((x / 8) + (y / 8)) % 2 == 0 { 90u16 } else { 60 };
                            let al = p[3] as u16;
                            let mix = |c: u8| ((c as u16 * al + bg * (255 - al)) / 255) as u8;
                            sheet.put_pixel(x0 + x, y0 + y, image::Rgba([mix(p[0]), mix(p[1]), mix(p[2]), 255]));
                        }
                    }
                }
                true
            }
            Err(_) => false,
        };
        cells.push(serde_json::json!({"cell": k, "hash": format!("{:016x}", t.hash), "w": t.width, "h": t.height, "fmt": t.dxgi_format, "ok": ok}));
    }
    sheet.save(&a[2])?;
    std::fs::write(format!("{}.json", a[2]), serde_json::to_vec(&cells)?)?;
    println!("{} cells, {} ok", cells.len(), cells.iter().filter(|c| c["ok"] == true).count());
    Ok(())
}
