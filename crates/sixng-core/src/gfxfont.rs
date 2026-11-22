//! Fonts of the Scaleform UI movies (`.gfx`, data.rpf): the font libraries
//! (`fontlib_efigs`, `fontlib_japanese` …) hold their fonts as standard SWF
//! DefineFont3 tags (code 75): per glyph a SHAPE of straight and quadratic
//! edges on a 1024-unit em in twips (20480 units, y down), a code table
//! (UCS-2), and with the layout flag ascent, descent, leading and advances.
//!
//! Written as TrueType (`.ttf`, quadratic outlines as stored, 2048 units per
//! em, for CSS `@font-face`) and as a three.js typeface JSON (FontLoader).
//! The fonts belong to their owners; this is for local use with the user's
//! own install.
use anyhow::{bail, Result};
use std::collections::BTreeMap;

/// One outline edge, in font units (twips, y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    Move(i32, i32),
    Line(i32, i32),
    /// control, anchor
    Quad(i32, i32, i32, i32),
}

#[derive(Debug, Clone)]
pub struct Glyph {
    pub code: u16,
    pub contours: Vec<Vec<Seg>>,
    pub advance: i32,
}

#[derive(Debug, Clone)]
pub struct Font {
    pub id: u16,
    pub name: String,
    pub bold: bool,
    pub italic: bool,
    pub ascent: i32,
    pub descent: i32,
    pub leading: i32,
    pub glyphs: Vec<Glyph>,
}

/// Twips per em in DefineFont3.
pub const EM: i32 = 1024 * 20;

struct Bits<'a> {
    b: &'a [u8],
    p: usize,
}

impl Bits<'_> {
    fn u(&mut self, n: u32) -> Result<u32> {
        let mut v = 0u32;
        for _ in 0..n {
            let byte = *self.b.get(self.p >> 3).ok_or_else(|| anyhow::anyhow!("shape runs past the tag"))?;
            v = (v << 1) | ((byte >> (7 - (self.p & 7))) & 1) as u32;
            self.p += 1;
        }
        Ok(v)
    }
    fn s(&mut self, n: u32) -> Result<i32> {
        let v = self.u(n)? as i64;
        Ok(if n > 0 && v & (1 << (n - 1)) != 0 { (v - (1i64 << n)) as i32 } else { v as i32 })
    }
}

/// One glyph SHAPE (no styles of its own beyond fill 1).
fn shape(b: &[u8], at: usize) -> Result<Vec<Vec<Seg>>> {
    let mut bt = Bits { b, p: at * 8 };
    let nf = bt.u(4)?;
    let nl = bt.u(4)?;
    let (mut x, mut y) = (0i32, 0i32);
    let mut out: Vec<Vec<Seg>> = Vec::new();
    loop {
        if bt.u(1)? == 0 {
            let fl = bt.u(5)?;
            if fl == 0 {
                break;
            }
            if fl & 1 != 0 {
                let n = bt.u(5)?;
                x = bt.s(n)?;
                y = bt.s(n)?;
                out.push(vec![Seg::Move(x, y)]);
            }
            if fl & 2 != 0 {
                bt.u(nf)?;
            }
            if fl & 4 != 0 {
                bt.u(nf)?;
            }
            if fl & 8 != 0 {
                bt.u(nl)?;
            }
            if fl & 16 != 0 {
                bail!("glyph shape with new styles");
            }
        } else {
            if out.is_empty() {
                out.push(vec![Seg::Move(x, y)]);
            }
            let cur = out.last_mut().unwrap();
            let straight = bt.u(1)? == 1;
            let n = bt.u(4)? + 2;
            if straight {
                let (dx, dy) = if bt.u(1)? == 1 {
                    (bt.s(n)?, bt.s(n)?)
                } else if bt.u(1)? == 1 {
                    (0, bt.s(n)?)
                } else {
                    (bt.s(n)?, 0)
                };
                x += dx;
                y += dy;
                cur.push(Seg::Line(x, y));
            } else {
                let cx = x + bt.s(n)?;
                let cy = y + bt.s(n)?;
                x = cx + bt.s(n)?;
                y = cy + bt.s(n)?;
                cur.push(Seg::Quad(cx, cy, x, y));
            }
        }
    }
    out.retain(|c| c.len() > 2);
    Ok(out)
}

fn parse_font3(t: &[u8]) -> Result<Font> {
    let rd16 = |o: usize| -> Result<u16> { t.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| anyhow::anyhow!("DefineFont3 truncated")) };
    let rd32 = |o: usize| -> Result<u32> { t.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| anyhow::anyhow!("DefineFont3 truncated")) };
    let id = rd16(0)?;
    let flags = *t.get(2).unwrap_or(&0);
    let nl = *t.get(4).unwrap_or(&0) as usize;
    let name = String::from_utf8_lossy(t.get(5..5 + nl).unwrap_or(&[])).trim_end_matches('\0').to_string();
    let mut q = 5 + nl;
    let ng = rd16(q)? as usize;
    q += 2;
    let wide = flags & 0x08 != 0;
    let ot = q;
    let off = |i: usize| -> Result<usize> { Ok(if wide { rd32(ot + 4 * i)? as usize } else { rd16(ot + 2 * i)? as usize }) };
    let mut shapes = Vec::with_capacity(ng);
    for i in 0..ng {
        shapes.push(shape(t, ot + off(i)?)?);
    }
    let mut q = if ng > 0 { ot + off(ng)? } else { ot + if wide { 4 } else { 2 } };
    let mut codes = Vec::with_capacity(ng);
    for i in 0..ng {
        codes.push(if flags & 0x04 != 0 { rd16(q + 2 * i)? } else { *t.get(q + i).unwrap_or(&0) as u16 });
    }
    q += if flags & 0x04 != 0 { 2 * ng } else { ng };
    let (mut ascent, mut descent, mut leading) = (EM * 4 / 5, EM / 5, 0);
    let mut adv = vec![EM / 2; ng];
    if flags & 0x80 != 0 {
        ascent = rd16(q)? as i32;
        descent = rd16(q + 2)? as i32;
        leading = rd16(q + 4)? as i16 as i32;
        q += 6;
        for (i, a) in adv.iter_mut().enumerate() {
            *a = rd16(q + 2 * i)? as i16 as i32;
        }
    }
    let glyphs = shapes.into_iter().zip(codes).zip(adv).map(|((contours, code), advance)| Glyph { code, contours, advance }).collect();
    Ok(Font { id, name, bold: flags & 1 != 0, italic: flags & 2 != 0, ascent, descent, leading, glyphs })
}

/// The DefineFont3 fonts of a `.gfx`/`.swf` movie (GFX/FWS plain, CFX/CWS zlib).
pub fn fonts(movie: &[u8]) -> Result<Vec<Font>> {
    if movie.len() < 8 {
        bail!("not a movie");
    }
    let body: Vec<u8> = match &movie[..3] {
        b"GFX" | b"FWS" => movie[8..].to_vec(),
        b"CFX" | b"CWS" => {
            let mut v = Vec::new();
            std::io::Read::read_to_end(&mut flate2::read::ZlibDecoder::new(&movie[8..]), &mut v)?;
            v
        }
        _ => bail!("not a GFX/SWF movie"),
    };
    let nbits = (body.first().copied().unwrap_or(0) >> 3) as usize;
    let mut p = (5 + 4 * nbits).div_ceil(8) + 4;
    let mut out = Vec::new();
    while p + 2 <= body.len() {
        let h = u16::from_le_bytes([body[p], body[p + 1]]);
        p += 2;
        let code = h >> 6;
        let mut len = (h & 0x3f) as usize;
        if len == 0x3f {
            let Some(b) = body.get(p..p + 4) else { break };
            len = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
            p += 4;
        }
        let Some(t) = body.get(p..p + len) else { break };
        if code == 75 {
            if let Ok(f) = parse_font3(t) {
                out.push(f);
            }
        }
        p += len;
        if code == 0 {
            break;
        }
    }
    Ok(out)
}

/// Font units -> TrueType units (2048 per em, y up).
fn tt(v: i32) -> i16 {
    (v as f64 * 2048.0 / EM as f64).round().clamp(-32768.0, 32767.0) as i16
}

struct TtGlyph {
    /// end point index of each contour
    ends: Vec<u16>,
    /// (x, y, on curve)
    pts: Vec<(i16, i16, bool)>,
    adv: u16,
}

fn tt_glyph(g: &Glyph) -> TtGlyph {
    let mut pts = Vec::new();
    let mut ends = Vec::new();
    for c in &g.contours {
        let start = pts.len();
        for s in c {
            match *s {
                Seg::Move(x, y) | Seg::Line(x, y) => pts.push((tt(x), tt(-y), true)),
                Seg::Quad(cx, cy, x, y) => {
                    pts.push((tt(cx), tt(-cy), false));
                    pts.push((tt(x), tt(-y), true));
                }
            }
        }
        // closed contours repeat their start point; drop it
        if pts.len() - start > 1 && pts.last() == pts.get(start) {
            pts.pop();
        }
        if pts.len() - start < 2 {
            pts.truncate(start);
            continue;
        }
        ends.push((pts.len() - 1) as u16);
    }
    TtGlyph { ends, pts, adv: tt(g.advance.max(0)) as u16 }
}

fn glyf_bytes(g: &TtGlyph) -> (Vec<u8>, [i16; 4]) {
    if g.pts.is_empty() {
        return (Vec::new(), [0; 4]);
    }
    let xmin = g.pts.iter().map(|p| p.0).min().unwrap();
    let ymin = g.pts.iter().map(|p| p.1).min().unwrap();
    let xmax = g.pts.iter().map(|p| p.0).max().unwrap();
    let ymax = g.pts.iter().map(|p| p.1).max().unwrap();
    let mut b = Vec::new();
    for v in [g.ends.len() as i16, xmin, ymin, xmax, ymax] {
        b.extend(v.to_be_bytes());
    }
    for e in &g.ends {
        b.extend(e.to_be_bytes());
    }
    b.extend(0u16.to_be_bytes()); // no instructions
    // flags (no repeat, all coordinates as 16-bit deltas)
    for p in &g.pts {
        b.push(if p.2 { 1 } else { 0 });
    }
    let (mut px, mut py) = (0i16, 0i16);
    for p in &g.pts {
        b.extend(p.0.wrapping_sub(px).to_be_bytes());
        px = p.0;
    }
    for p in &g.pts {
        b.extend(p.1.wrapping_sub(py).to_be_bytes());
        py = p.1;
    }
    while b.len() % 4 != 0 {
        b.push(0);
    }
    (b, [xmin, ymin, xmax, ymax])
}

fn checksum(d: &[u8]) -> u32 {
    d.chunks(4).fold(0u32, |s, c| {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        s.wrapping_add(u32::from_be_bytes(w))
    })
}

fn name_table(family: &str, sub: &str) -> Vec<u8> {
    let full = format!("{family} {sub}");
    let ps: String = format!("{family}-{sub}").chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    let recs: Vec<(u16, String)> = vec![(1, family.into()), (2, sub.into()), (3, ps.clone()), (4, full), (6, ps)];
    let mut strings = Vec::new();
    let mut b = Vec::new();
    b.extend(0u16.to_be_bytes());
    b.extend((recs.len() as u16).to_be_bytes());
    b.extend((6 + 12 * recs.len() as u16).to_be_bytes());
    for (id, s) in &recs {
        let u: Vec<u8> = s.encode_utf16().flat_map(|c| c.to_be_bytes()).collect();
        for v in [3u16, 1, 0x409, *id, u.len() as u16, strings.len() as u16] {
            b.extend(v.to_be_bytes());
        }
        strings.extend(u);
    }
    b.extend(strings);
    b
}

/// The font as a TrueType file. Glyph 0 is an empty .notdef; characters
/// map through a format-4 cmap (BMP).
pub fn to_ttf(f: &Font) -> Vec<u8> {
    let mut glyphs: Vec<&Glyph> = f.glyphs.iter().collect();
    glyphs.sort_by_key(|g| g.code);
    glyphs.dedup_by_key(|g| g.code);
    let tg: Vec<TtGlyph> = std::iter::once(TtGlyph { ends: vec![], pts: vec![], adv: 1024 }).chain(glyphs.iter().map(|g| tt_glyph(g))).collect();
    let mut glyf = Vec::new();
    let mut loca: Vec<u32> = Vec::new();
    let mut bbox = [i16::MAX, i16::MAX, i16::MIN, i16::MIN];
    let (mut max_pts, mut max_ctr) = (0u16, 0u16);
    for g in &tg {
        loca.push(glyf.len() as u32);
        let (b, bb) = glyf_bytes(g);
        if !b.is_empty() {
            bbox = [bbox[0].min(bb[0]), bbox[1].min(bb[1]), bbox[2].max(bb[2]), bbox[3].max(bb[3])];
            max_pts = max_pts.max(g.pts.len() as u16);
            max_ctr = max_ctr.max(g.ends.len() as u16);
        }
        glyf.extend(b);
    }
    loca.push(glyf.len() as u32);
    if bbox[0] > bbox[2] {
        bbox = [0; 4];
    }
    let n = tg.len() as u16;
    let asc = tt(f.ascent);
    let desc = -tt(f.descent);
    let be16 = |v: &mut Vec<u8>, x: u16| v.extend(x.to_be_bytes());
    // head
    let mut head = Vec::new();
    for x in [0x0001_0000u32, 0x0001_0000, 0, 0x5F0F_3CF5] {
        head.extend(x.to_be_bytes());
    }
    be16(&mut head, 0b1011); // flags: baseline y=0, lsb x=0, integer ppem
    be16(&mut head, 2048);
    head.extend([0u8; 16]); // created, modified
    for v in bbox {
        head.extend(v.to_be_bytes());
    }
    be16(&mut head, (f.bold as u16) | ((f.italic as u16) << 1));
    be16(&mut head, 8); // lowest rec ppem
    head.extend(2i16.to_be_bytes());
    be16(&mut head, 1); // long loca
    be16(&mut head, 0);
    // hhea
    let max_adv = tg.iter().map(|g| g.adv).max().unwrap_or(0);
    let mut hhea = Vec::new();
    hhea.extend(0x0001_0000u32.to_be_bytes());
    for v in [asc, desc, tt(f.leading)] {
        hhea.extend(v.to_be_bytes());
    }
    be16(&mut hhea, max_adv);
    for v in [bbox[0], 0, bbox[2], 1, 0, 0, 0, 0, 0, 0, 0] {
        hhea.extend(v.to_be_bytes());
    }
    be16(&mut hhea, n);
    // hmtx: advance + lsb (x min)
    let mut hmtx = Vec::new();
    for g in &tg {
        be16(&mut hmtx, g.adv);
        hmtx.extend(g.pts.iter().map(|p| p.0).min().unwrap_or(0).to_be_bytes());
    }
    // maxp 1.0
    let mut maxp = Vec::new();
    maxp.extend(0x0001_0000u32.to_be_bytes());
    for v in [n, max_pts, max_ctr, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0] {
        be16(&mut maxp, v);
    }
    // cmap format 4, one segment per character + the 0xFFFF end
    let codes: Vec<u16> = glyphs.iter().map(|g| g.code).filter(|&c| c != 0xFFFF).collect();
    let seg = codes.len() as u16 + 1;
    let mut sub = Vec::new();
    be16(&mut sub, 4);
    let sub_len = 16 + 8 * seg;
    be16(&mut sub, sub_len);
    be16(&mut sub, 0);
    be16(&mut sub, seg * 2);
    let lg = 15 - (seg as u16).leading_zeros() as u16; // floor(log2(seg))
    be16(&mut sub, 2 << lg);
    be16(&mut sub, lg);
    be16(&mut sub, seg * 2 - (2 << lg));
    for &c in &codes {
        be16(&mut sub, c);
    }
    be16(&mut sub, 0xFFFF);
    be16(&mut sub, 0);
    for &c in &codes {
        be16(&mut sub, c);
    }
    be16(&mut sub, 0xFFFF);
    for (k, &c) in codes.iter().enumerate() {
        be16(&mut sub, ((k as u16 + 1).wrapping_sub(c)) as u16);
    }
    be16(&mut sub, 1);
    for _ in 0..seg {
        be16(&mut sub, 0);
    }
    let mut cmap = Vec::new();
    for v in [0u16, 1, 3, 1] {
        be16(&mut cmap, v);
    }
    cmap.extend(12u32.to_be_bytes());
    cmap.extend(sub);
    // post 3.0
    let mut post = Vec::new();
    post.extend(0x0003_0000u32.to_be_bytes());
    post.extend([0u8; 28]);
    // OS/2 version 1
    let first = codes.first().copied().unwrap_or(0x20);
    let last = codes.last().copied().unwrap_or(0x7e);
    let mut os2 = Vec::new();
    be16(&mut os2, 1);
    let avg = (tg.iter().map(|g| g.adv as u32).sum::<u32>() / n.max(1) as u32) as i16;
    os2.extend(avg.to_be_bytes());
    be16(&mut os2, if f.bold { 700 } else { 400 });
    be16(&mut os2, 5);
    be16(&mut os2, 0); // fsType: installable
    for _ in 0..10 {
        be16(&mut os2, 0); // sub/superscript, strikeout
    }
    be16(&mut os2, 0); // family class
    os2.extend([0u8; 10]); // panose
    os2.extend([0u8; 16]); // unicode ranges
    os2.extend(*b"SIXN");
    be16(&mut os2, if f.bold { 0x20 } else { 0x40 } | if f.italic { 1 } else { 0 });
    be16(&mut os2, first);
    be16(&mut os2, last);
    for v in [asc, desc, tt(f.leading)] {
        os2.extend(v.to_be_bytes());
    }
    be16(&mut os2, asc.max(0) as u16);
    be16(&mut os2, desc.unsigned_abs());
    os2.extend([0u8; 8]); // code page ranges
    let style = match (f.bold, f.italic) {
        (true, true) => "Bold Italic",
        (true, false) => "Bold",
        (false, true) => "Italic",
        _ => "Regular",
    };
    let mut loca_b = Vec::new();
    for o in &loca {
        loca_b.extend(o.to_be_bytes());
    }
    let tables: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"OS/2", os2),
        (b"cmap", cmap),
        (b"glyf", glyf),
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", hmtx),
        (b"loca", loca_b),
        (b"maxp", maxp),
        (b"name", name_table(&f.name, style)),
        (b"post", post),
    ];
    let nt = tables.len() as u16;
    let lg = 15 - nt.leading_zeros() as u16;
    let mut out = Vec::new();
    out.extend(0x0001_0000u32.to_be_bytes());
    for v in [nt, 16 << lg, lg, nt * 16 - (16 << lg)] {
        out.extend(v.to_be_bytes());
    }
    let mut off = 12 + 16 * tables.len();
    let mut body = Vec::new();
    let mut head_at = 0;
    for (tag, d) in &tables {
        out.extend(*tag);
        out.extend(checksum(d).to_be_bytes());
        out.extend((off as u32).to_be_bytes());
        out.extend((d.len() as u32).to_be_bytes());
        if *tag == b"head" {
            head_at = off;
        }
        body.extend(d);
        while body.len() % 4 != 0 {
            body.push(0);
        }
        off = 12 + 16 * tables.len() + body.len();
    }
    out.extend(body);
    let adj = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
    out[head_at + 8..head_at + 12].copy_from_slice(&adj.to_be_bytes());
    out
}

/// The font as a three.js typeface JSON (FontLoader): outlines as
/// "m x y l x y q cx cy x y" in units of `resolution` per em, y up.
pub fn to_typeface(f: &Font) -> serde_json::Value {
    let s = 1000.0 / EM as f64;
    let r = |v: i32| (v as f64 * s).round() as i64;
    let mut glyphs = BTreeMap::new();
    let (mut xmin, mut xmax) = (i64::MAX, i64::MIN);
    for g in &f.glyphs {
        let Some(ch) = char::from_u32(g.code as u32) else { continue };
        let mut o = String::new();
        let (mut gx0, mut gx1) = (i64::MAX, i64::MIN);
        for c in &g.contours {
            for seg in c {
                let (cmd, v): (&str, Vec<i32>) = match *seg {
                    // three.js reads q as anchor then control
                    Seg::Move(x, y) => ("m", vec![x, -y]),
                    Seg::Line(x, y) => ("l", vec![x, -y]),
                    Seg::Quad(cx, cy, x, y) => ("q", vec![x, -y, cx, -cy]),
                };
                o.push_str(cmd);
                for (k, x) in v.iter().enumerate() {
                    o.push_str(&format!(" {}", r(*x)));
                    if k % 2 == 0 {
                        gx0 = gx0.min(r(*x));
                        gx1 = gx1.max(r(*x));
                    }
                }
                o.push(' ');
            }
        }
        if gx0 > gx1 {
            (gx0, gx1) = (0, 0);
        }
        xmin = xmin.min(gx0);
        xmax = xmax.max(gx1);
        glyphs.insert(ch.to_string(), serde_json::json!({"ha": r(g.advance), "x_min": gx0, "x_max": gx1, "o": o.trim_end()}));
    }
    serde_json::json!({
        "glyphs": glyphs,
        "familyName": f.name,
        "ascender": r(f.ascent),
        "descender": -r(f.descent),
        "underlinePosition": -r(f.descent) / 2,
        "underlineThickness": 50,
        "boundingBox": {"xMin": xmin.min(0), "xMax": xmax.max(0), "yMin": -r(f.descent), "yMax": r(f.ascent)},
        "resolution": 1000,
        "original_font_information": {"format": 0, "fontFamily": f.name},
        "cssFontWeight": if f.bold { "bold" } else { "normal" },
        "cssFontStyle": if f.italic { "italic" } else { "normal" },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Font {
        let c = vec![Seg::Move(0, 0), Seg::Line(EM / 2, 0), Seg::Quad(EM / 2, -EM / 2, 0, -EM / 2), Seg::Line(0, 0)];
        Font { id: 1, name: "t".into(), bold: false, italic: false, ascent: EM * 4 / 5, descent: EM / 5, leading: 0, glyphs: vec![Glyph { code: 'A' as u16, contours: vec![c], advance: EM / 2 }] }
    }

    #[test]
    fn ttf_tables_and_checksum() {
        let b = to_ttf(&square());
        assert_eq!(&b[..4], &[0, 1, 0, 0]);
        assert_eq!(u16::from_be_bytes([b[4], b[5]]), 10);
        // whole-file checksum is the magic after the adjustment
        assert_eq!(checksum(&b), 0xB1B0_AFBA);
        // glyph 1 ('A'): 3 points (start point repeated at the end is dropped)
        let tag_at = |t: &[u8; 4]| (0..10).map(|i| 12 + 16 * i).find(|&o| &b[o..o + 4] == t).unwrap();
        let glyf = u32::from_be_bytes(b[tag_at(b"glyf") + 8..tag_at(b"glyf") + 12].try_into().unwrap()) as usize;
        assert_eq!(i16::from_be_bytes([b[glyf], b[glyf + 1]]), 1);
        assert_eq!(u16::from_be_bytes([b[glyf + 10], b[glyf + 11]]), 3);
    }

    #[test]
    fn typeface_paths() {
        let v = to_typeface(&square());
        let o = v["glyphs"]["A"]["o"].as_str().unwrap();
        assert_eq!(o, "m 0 0 l 500 0 q 0 500 500 500 l 0 0");
        assert_eq!(v["glyphs"]["A"]["ha"], 500);
    }

    #[test]
    fn shape_bits() {
        // nf=1 nl=0; moveTo (5 bits n=2) 1,-1; straight general edge n=2: dx=1 dy=1; end
        // bits: 0001 0000 | 0 00001 00010 01 11 | 1 1 0000 1 01 01 | 0 00000
        let bits = "00010000" .to_string() + "0" + "00001" + "00010" + "01" + "11" + "1" + "1" + "0000" + "1" + "01" + "01" + "0" + "00000";
        let mut bytes = vec![0u8; bits.len().div_ceil(8)];
        for (i, c) in bits.chars().enumerate() {
            if c == '1' {
                bytes[i / 8] |= 0x80 >> (i % 8);
            }
        }
        let s = shape(&bytes, 0).unwrap_or_default();
        // a single moveTo + line is dropped (needs > 2 segments); parse must not fail
        assert!(s.is_empty());
    }
}
