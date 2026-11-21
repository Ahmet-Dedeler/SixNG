//! `ttpf` texture stores (`data/ttdb/**`, magic `RDS5`).
//!
//! ```text
//! +0x00 "RDS5"
//! +0x04 u32 chunk-table entries (the last one is a terminator)
//! +0x08 u32 size of the virtual (uncompressed) space, including a 0x1000 header page
//! +0x0C u32 0x16000000
//! +0x10 u32 ?, +0x14 u32 0
//! +0x18 chunk table, 8 bytes per entry:
//!       u32 a: (a & 0xFFFFFF) << 8 = file offset of a zlib stream; bits 24-29 = flags
//!              (codec: 0x20 zlib; 0x04 BC1, 0x0C/0x10 BC3/BC4?, 0x14 BC5, 0x18 BC6H,
//!              0x1C BC7 = per-format BCPack, the Xbox Series hardware texture codec,
//!              no public decoder; 0x00 stored), bits 30-31 = 4 KiB pages added to the virtual offset
//!       u32 b: virtual offset = 0x1000 + (b & 0xFFFFF) * 0x4000 + (a >> 30) * 0x1000
//!              (the 0x1000 is the header page); b >> 20 = ?
//!       A chunk runs to the next entry, both in the file and in the virtual space.
//!       The last entry is a terminator (file end, end of the virtual space). The
//!       virtual space can exceed 4 GiB (the header's size field is truncated).
//! align_up(0x10 + 8 * entries, 0x100): "ETPF" texture index
//!       +4 u32 version (0x27A), +8 u32 texture count n, +0xC u32 length of the next part
//!       +0x10            n x 16-byte descriptors
//!       +0x10 + 16n      n x u64 texture hashes, sorted (descriptor i <-> hash i)
//!       ...              other tables (u32 list, n x 32-byte digests)
//! descriptor:
//!       u32 d0: bits 0-13 width-1, bits 14-27 height-1, bits 28-31 mip count-1
//!       u32 d1: bits 0-2 flags, bits 3-10 DXGI format, bits 20-23 ?, bits 24-27 tile mode
//!              (0 = SW_64KB_D, 1/2/3 = SW_4KB_S, see `gfx10`)
//!       u64 loc: bits 0-20 first 4 KiB page of the texture, counted after the
//!              0x1000 header page; higher bits ?
//! ```
//! A texture's pixel data is its full mip chain in the AMD Gfx10 layout
//! (`crate::gfx10`): mips smallest first, small mips packed in a mip-tail
//! block, mip 0 last. The hashes are the 64-bit texture hashes referenced by
//! drawable shaders.
use anyhow::{anyhow, Context, Result};
use flate2::read::ZlibDecoder;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct TtpfTex {
    pub hash: u64,
    pub width: u32,
    pub height: u32,
    pub dxgi_format: u32,
    /// Byte offset in the virtual space.
    pub virt: u64,
    /// Bytes of the texture's mip chain (from the Gfx10 layout; textures
    /// can overlap others' padding, so this is not the distance to the next one).
    pub size: u64,
    pub d0: u32,
    pub d1: u32,
    pub loc: u64,
}

pub struct Ttpf {
    pub path: PathBuf,
    /// (file offset, virtual offset, flags byte) per chunk, plus a terminator.
    chunks: Vec<(u64, u64, u8)>,
    pub virt_size: u64,
    pub textures: Vec<TtpfTex>,
    by_hash: HashMap<u64, usize>,
}

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u64le(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}

impl Ttpf {
    pub fn open(path: &Path) -> Result<Ttpf> {
        let mut f = File::open(path).with_context(|| format!("open {}", path.display()))?;
        let mut h = [0u8; 24];
        f.read_exact(&mut h)?;
        if &h[..4] != b"RDS5" {
            return Err(anyhow!("not an RDS5 store"));
        }
        let n = u32le(&h, 4) as usize;
        let virt_size = u32le(&h, 8) as u64;
        let mut t = vec![0u8; 8 * n];
        f.read_exact(&mut t)?;
        let mut chunks = Vec::with_capacity(n);
        for i in 0..n {
            let a = u32le(&t, 8 * i);
            let b = u32le(&t, 8 * i + 4);
            if a == 0 && b == 0 {
                break;
            }
            chunks.push((((a & 0xFF_FFFF) as u64) << 8, 0x1000 + (b & 0xF_FFFF) as u64 * 0x4000 + (a >> 30) as u64 * 0x1000, (a >> 24) as u8));
        }
        let eoff = ((0x10 + 8 * n + 0xFF) & !0xFF) as u64;
        f.seek(SeekFrom::Start(eoff))?;
        let mut e = [0u8; 16];
        f.read_exact(&mut e)?;
        if &e[..4] != b"ETPF" {
            return Err(anyhow!("no ETPF index at {eoff:#x}"));
        }
        let nt = u32le(&e, 8) as usize;
        let mut body = vec![0u8; 24 * nt];
        f.read_exact(&mut body)?;
        let mut textures = Vec::with_capacity(nt);
        for i in 0..nt {
            let d0 = u32le(&body, 16 * i);
            let d1 = u32le(&body, 16 * i + 4);
            let loc = u64le(&body, 16 * i + 8);
            textures.push(TtpfTex {
                hash: u64le(&body, 16 * nt + 8 * i),
                width: (d0 & 0x3FFF) + 1,
                height: ((d0 >> 14) & 0x3FFF) + 1,
                dxgi_format: (d1 >> 3) & 0xFF,
                virt: (loc & 0x1F_FFFF) * 0x1000 + 0x1000,
                size: 0,
                d0,
                d1,
                loc,
            });
        }
        for t in textures.iter_mut() {
            t.size = t.layout().map(|l| l.total).unwrap_or(0);
        }
        let by_hash = textures.iter().enumerate().map(|(i, t)| (t.hash, i)).collect();
        Ok(Ttpf { path: path.to_path_buf(), chunks, virt_size, textures, by_hash })
    }

    pub fn get(&self, hash: u64) -> Option<&TtpfTex> {
        self.by_hash.get(&hash).map(|&i| &self.textures[i])
    }

    /// Read `len` bytes of the virtual space from `virt`, inflating only the
    /// chunks that overlap it.
    pub fn read(&self, virt: u64, len: u64) -> Result<Vec<u8>> {
        Ok(self.read_ex(virt, len)?.0)
    }

    /// Like `read`; also returns how many bytes fell in chunks that are not
    /// zlib (flag bits 0x1C set, another codec, left as zeros).
    pub fn read_ex(&self, virt: u64, len: u64) -> Result<(Vec<u8>, u64)> {
        let mut out = vec![0u8; len as usize];
        let mut skipped = 0u64;
        let end = virt + len;
        let mut f = File::open(&self.path)?;
        for i in 0..self.chunks.len().saturating_sub(1) {
            let (foff, v0, fl) = self.chunks[i];
            let (fnext, v1, _) = self.chunks[i + 1];
            if v1 <= virt || v0 >= end {
                continue;
            }
            if fl & 0x1C != 0 {
                skipped += end.min(v1) - virt.max(v0);
                continue;
            }
            f.seek(SeekFrom::Start(foff))?;
            let mut comp = vec![0u8; (fnext - foff) as usize];
            f.read_exact(&mut comp)?;
            let raw = if fl & 0x3F == 0 {
                comp // stored (5 chunks in all stores; sizes match)
            } else {
                let mut raw = Vec::with_capacity((v1 - v0) as usize);
                ZlibDecoder::new(&comp[..]).read_to_end(&mut raw).with_context(|| format!("chunk {i} at {foff:#x}"))?;
                raw
            };
            let s = virt.max(v0);
            let e = end.min(v1).min(v0 + raw.len() as u64);
            if e > s {
                out[(s - virt) as usize..(e - virt) as usize].copy_from_slice(&raw[(s - v0) as usize..(e - v0) as usize]);
            }
        }
        Ok((out, skipped))
    }

    /// Bytes of [virt, virt + len) that fall in BCPack chunks (no read).
    pub fn bcpack_bytes(&self, virt: u64, len: u64) -> u64 {
        let end = virt + len;
        let mut n = 0;
        for w in self.chunks.windows(2) {
            let ((_, v0, fl), (_, v1, _)) = (w[0], w[1]);
            if v1 > virt && v0 < end && fl & 0x1C != 0 {
                n += end.min(v1) - virt.max(v0);
            }
        }
        n
    }

    /// Virtual ranges [start, end) of the BCPack chunks, sorted and merged
    /// (for fast per-texture checks over a whole store).
    pub fn bcpack_ranges(&self) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self.chunks.windows(2).filter(|w| w[0].2 & 0x1C != 0).map(|w| (w[0].1, w[1].1)).filter(|r| r.1 > r.0).collect();
        v.sort();
        let mut m: Vec<(u64, u64)> = Vec::with_capacity(v.len());
        for r in v {
            match m.last_mut() {
                Some(l) if r.0 <= l.1 => l.1 = l.1.max(r.1),
                _ => m.push(r),
            }
        }
        m
    }

    /// Largest mip of `t` that lies entirely outside BCPack chunks.
    pub fn best_mip(&self, t: &TtpfTex) -> Option<u32> {
        let l = t.layout()?;
        (0..l.mips.len()).find(|&m| {
            let (a, b) = l.mip_span(m);
            self.bcpack_bytes(t.virt + a, b - a) == 0
        }).map(|m| m as u32)
    }

    /// Pixel data of a texture (full mip chain, tiled).
    pub fn data(&self, t: &TtpfTex) -> Result<Vec<u8>> {
        self.read(t.virt, t.size.min(64 << 20))
    }

    /// Pixel data plus the number of bytes that could not be decoded.
    pub fn data_ex(&self, t: &TtpfTex) -> Result<(Vec<u8>, u64)> {
        self.read_ex(t.virt, t.size.min(64 << 20))
    }
}

/// All stores under `<game>/data/ttdb`, opened lazily.
pub struct TtpfSet {
    pub stores: Vec<Ttpf>,
}

impl TtpfSet {
    pub fn open_dir(dir: &Path) -> TtpfSet {
        let mut paths = Vec::new();
        fn walk(d: &Path, out: &mut Vec<PathBuf>) {
            if let Ok(rd) = std::fs::read_dir(d) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        walk(&p, out);
                    } else if p.extension().map(|x| x == "ttpf").unwrap_or(false) {
                        out.push(p);
                    }
                }
            }
        }
        walk(dir, &mut paths);
        paths.sort();
        TtpfSet { stores: paths.iter().filter_map(|p| Ttpf::open(p).ok()).collect() }
    }

    pub fn get(&self, hash: u64) -> Option<(&Ttpf, &TtpfTex)> {
        self.stores.iter().find_map(|s| s.get(hash).map(|t| (s, t)))
    }
}

impl TtpfTex {
    /// Mip count: d0 bits 28-31 + 1 (UI textures have 1-2, a 1024 map
    /// texture 7, i.e. down to 16 px).
    pub fn mips(&self) -> u32 {
        (self.d0 >> 28) + 1
    }
    /// A `TexInfo` describing this texture, with `data_off` = 0 relative to
    /// the bytes returned by `Ttpf::data`.
    pub fn tex_info(&self, name: &str) -> crate::texture::TexInfo {
        crate::texture::TexInfo {
            name: name.to_string(),
            width: self.width,
            height: self.height,
            depth: 1,
            mips: self.mips(),
            dxgi_format: self.dxgi_format,
            format: crate::texture::dxgi_name(self.dxgi_format),
            struct_off: 0,
            data_off: 0,
            size_field: 0,
            swizzle: crate::texture::Fmt::from_dxgi(self.dxgi_format).swizzle_status().into(),
        }
    }
}

/// Decode mip `m` of a ttpf texture to RGBA8. Returns (rgba, width, height).
pub fn decode_rgba(t: &TtpfTex, data: &[u8], m: u32) -> Result<(Vec<u8>, u32, u32)> {
    use crate::texture as tx;
    let ti = t.tex_info("");
    let fmt = tx::Fmt::from_dxgi(t.dxgi_format);
    if fmt == tx::Fmt::Unknown {
        return Err(anyhow!("unsupported format {}", ti.format));
    }
    let (mw, mh) = ((t.width >> m).max(1), (t.height >> m).max(1));
    let l = t.layout().ok_or_else(|| anyhow!("tile mode {} / {} not supported", t.tile_mode(), ti.format))?;
    if m as usize >= l.mips.len() {
        return Err(anyhow!("mip {m} out of range"));
    }
    let lin = l.untile(data, m as usize);
    Ok((tx::to_rgba(fmt, &lin, mw, mh)?, mw, mh))
}

impl TtpfTex {
    /// Tiling mode (d1 bits 24-27): 0 = SW_64KB_D, 1/2/3 = SW_4KB_S
    /// (Gfx10 swizzle modes; all verified). Other values occur on a few
    /// special formats and are not decoded.
    pub fn tile_mode(&self) -> u32 {
        (self.d1 >> 24) & 0xF
    }
    pub fn swizzle(&self) -> Option<crate::gfx10::Swizzle> {
        match self.tile_mode() {
            0 => Some(crate::gfx10::Swizzle::D64K),
            1..=3 => Some(crate::gfx10::Swizzle::S4K),
            _ => None,
        }
    }
    /// Mip-chain layout (`None` for unsupported tile modes or element sizes:
    /// only 8- and 16-byte BCn elements are mapped).
    pub fn layout(&self) -> Option<crate::gfx10::Layout> {
        let fmt = crate::texture::Fmt::from_dxgi(self.dxgi_format);
        if fmt == crate::texture::Fmt::Unknown {
            return None;
        }
        let bd = fmt.block_dim();
        crate::gfx10::Layout::new(self.swizzle()?, self.width.div_ceil(bd), self.height.div_ceil(bd), self.mips(), fmt.bytes_per_elem())
    }
    pub fn decodable(&self) -> bool {
        self.layout().is_some()
    }
}

/// The stores under `<game root>/data/ttdb`, opened once per process.
pub fn global(game_root: &Path) -> &'static TtpfSet {
    static SET: std::sync::OnceLock<TtpfSet> = std::sync::OnceLock::new();
    SET.get_or_init(|| TtpfSet::open_dir(&game_root.join("data").join("ttdb")))
}

/// Mean colour of a decoded texture and whether it looks like a tangent-space
/// normal map (mean close to (128, 128, 255)).
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct PngStats {
    pub width: u32,
    pub height: u32,
    pub mean_rgba: [f32; 4],
    pub normal_like: bool,
    /// Mip level written (0 = full size). Higher when the top mips are in
    /// BCPack chunks and only lower mips could be decoded.
    #[serde(default)]
    pub mip: u32,
}

fn stats(rgba: &[u8], w: u32, h: u32) -> PngStats {
    let mut s = [0f64; 4];
    for px in rgba.chunks_exact(4) {
        for k in 0..4 {
            s[k] += px[k] as f64;
        }
    }
    let n = (rgba.len() / 4).max(1) as f64;
    let m = s.map(|x| (x / n) as f32);
    let normal_like = m[2] > 200.0 && (100.0..160.0).contains(&m[0]) && (100.0..160.0).contains(&m[1]);
    PngStats { width: w, height: h, mean_rgba: m, normal_like, mip: 0 }
}

/// Decode mip 0 of a texture to `<dir>/<hash>.png` (plus a `.json` with
/// its stats) unless it exists. Returns the file path and the stats.
pub fn write_png(store: &Ttpf, t: &TtpfTex, dir: &Path) -> Result<(PathBuf, PngStats)> {
    let p = dir.join(format!("{:016x}.png", t.hash));
    let js = dir.join(format!("{:016x}.json", t.hash));
    if p.exists() {
        if let Some(st) = std::fs::read(&js).ok().and_then(|b| serde_json::from_slice(&b).ok()) {
            return Ok((p, st));
        }
    }
    if !t.decodable() {
        return Err(anyhow!("tile mode {} / format {} not supported", t.tile_mode(), t.dxgi_format));
    }
    let (data, skipped) = store.data_ex(t)?;
    // Top mips in BCPack chunks: fall back to the largest mip stored as zlib.
    let mip = if skipped == 0 {
        0
    } else {
        match store.best_mip(t) {
            Some(m) => m,
            None => return Err(anyhow!("{skipped} bytes in BCPack chunks, no mip outside them (Xbox hardware texture codec, no public decoder)")),
        }
    };
    let (rgba, w, h) = decode_rgba(t, &data, mip)?;
    let mut st = stats(&rgba, w, h);
    st.mip = mip;
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{:016x}.{:?}.tmp.png", t.hash, std::thread::current().id()).replace(['(', ')'], ""));
    crate::texture::save_png(&tmp, &rgba, w, h)?;
    std::fs::rename(&tmp, &p)?;
    std::fs::write(&js, serde_json::to_vec(&st)?)?;
    Ok((p, st))
}
