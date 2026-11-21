//! AMD Gfx10 (RDNA2, Xbox Series) 2D texture layout, as used by the `ttpf`
//! stores. Port of the parts of addrlib (`Gfx10Lib::ComputeSurfaceInfoMacroTiled`)
//! that place mips, for thin 2D surfaces without pipe/bank XOR.
//!
//! * Each swizzle mode is a block (4 KiB or 64 KiB) with a fixed bit order of
//!   x/y element-coordinate bits inside it; blocks are stored row-major.
//! * Mips are stored **smallest first**; mip 0 is last.
//! * Mips that fit in half a block (`w <= block_w / 2`, `h <= block_h`) and are
//!   among the last `max_mips_in_tail` share one "mip tail" block at offset 0,
//!   each at a fixed 2D origin derived from its tail offset.
//!
//! Verified on the stores: tile mode 0 = `SW_64KB_D`, modes 1, 2, 3 =
//! `SW_4KB_S` (mip n+1 equals mip n downsampled, every format checked).

/// Element-coordinate bit order inside a block, low bit first:
/// (is_x, coordinate bit).
fn equation(sw: Swizzle, bpe: u32) -> Option<&'static [(bool, u8)]> {
    const X: bool = true;
    const Y: bool = false;
    Some(match (sw, bpe) {
        (Swizzle::S4K, 16) => &[(Y, 0), (Y, 1), (X, 0), (X, 1), (Y, 2), (X, 2), (Y, 3), (X, 3)],
        (Swizzle::S4K, 8) => &[(X, 0), (Y, 0), (Y, 1), (X, 1), (X, 2), (Y, 2), (X, 3), (Y, 3), (X, 4)],
        (Swizzle::D64K, 16) => &[(X, 0), (Y, 0), (X, 1), (Y, 1), (Y, 2), (X, 2), (Y, 3), (X, 3), (Y, 4), (X, 4), (Y, 5), (X, 5)],
        (Swizzle::D64K, 8) => &[(X, 0), (Y, 0), (X, 1), (X, 2), (Y, 1), (Y, 2), (X, 3), (Y, 3), (X, 4), (Y, 4), (X, 5), (Y, 5), (X, 6)],
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Swizzle {
    /// SW_4KB_S
    S4K,
    /// SW_64KB_D
    D64K,
}

#[derive(Debug, Clone)]
pub struct MipPlace {
    /// Size in elements.
    pub w: u32,
    pub h: u32,
    /// Byte offset of the mip's first block (non-tail) or of the tail block.
    pub offset: u64,
    /// Origin inside the tail block, in elements (tail mips only).
    pub tail: Option<(u32, u32)>,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub eq: &'static [(bool, u8)],
    pub bpe: u32,
    pub block_w: u32,
    pub block_h: u32,
    pub block_bytes: u64,
    pub mips: Vec<MipPlace>,
    /// Bytes of the whole mip chain.
    pub total: u64,
}

impl Layout {
    /// `w`, `h` in elements (BCn blocks), `bpe` bytes per element.
    pub fn new(sw: Swizzle, w: u32, h: u32, mips: u32, bpe: u32) -> Option<Layout> {
        let eq = equation(sw, bpe)?;
        let nx = eq.iter().filter(|e| e.0).count() as u32;
        let (bw, bh) = (1u32 << nx, 1u32 << (eq.len() as u32 - nx));
        let bs = (bw * bh * bpe) as u64;
        let log2 = 63 - bs.leading_zeros();
        let max_tail = if log2 <= 11 { 1 + (1 << (log2 - 9)) } else { log2 - 4 };
        let (tw, th) = (bw / 2, bh);
        let nm = mips.max(1);
        let dims: Vec<(u32, u32)> = (0..nm).map(|i| ((w >> i).max(1), (h >> i).max(1))).collect();
        let first = dims.iter().enumerate().position(|(i, &(mw, mh))| mw <= tw && mh <= th && nm - i as u32 <= max_tail).unwrap_or(nm as usize);
        let mut place: Vec<MipPlace> = dims.iter().map(|&(mw, mh)| MipPlace { w: mw, h: mh, offset: 0, tail: None }).collect();
        let mut off = if first < nm as usize { bs } else { 0 };
        for i in (0..first).rev() {
            let (mw, mh) = dims[i];
            place[i].offset = off;
            off += (mw.div_ceil(bw) * bw) as u64 * (mh.div_ceil(bh) * bh) as u64 * bpe as u64;
        }
        let b256 = if bpe == 16 { (4, 4) } else { (8, 4) };
        for (k, p) in place.iter_mut().enumerate().skip(first) {
            let m = max_tail - 1 - (k - first) as u32;
            let mo: u32 = if m > 6 { 16 << m } else { m << 8 };
            let (mut mx, mut my) = (0u32, 0u32);
            for b in 0..6 {
                mx |= ((mo >> (9 + 2 * b)) & 1) << b;
                my |= ((mo >> (8 + 2 * b)) & 1) << b;
            }
            p.offset = 0;
            p.tail = Some((mx * b256.0, my * b256.1));
        }
        Some(Layout { eq, bpe, block_w: bw, block_h: bh, block_bytes: bs, mips: place, total: off })
    }

    fn inblock(&self, x: u32, y: u32) -> u64 {
        let mut a = 0u64;
        for (k, &(isx, b)) in self.eq.iter().enumerate() {
            let v = if isx { x } else { y };
            a |= (((v >> b) & 1) as u64) << k;
        }
        a
    }

    /// Byte range [start, end) that mip `m` occupies (the whole tail block
    /// for tail mips).
    pub fn mip_span(&self, m: usize) -> (u64, u64) {
        let p = &self.mips[m];
        match p.tail {
            Some(_) => (0, self.block_bytes),
            None => (p.offset, p.offset + (p.w.div_ceil(self.block_w) * self.block_w) as u64 * (p.h.div_ceil(self.block_h) * self.block_h) as u64 * self.bpe as u64),
        }
    }

    /// Byte address of element (x, y) of mip `m`.
    pub fn addr(&self, m: usize, x: u32, y: u32) -> u64 {
        let p = &self.mips[m];
        match p.tail {
            Some((cx, cy)) => self.inblock(x + cx, y + cy) * self.bpe as u64,
            None => {
                let pitch = p.w.div_ceil(self.block_w) as u64;
                let blk = (y / self.block_h) as u64 * pitch + (x / self.block_w) as u64;
                p.offset + blk * self.block_bytes + self.inblock(x % self.block_w, y % self.block_h) * self.bpe as u64
            }
        }
    }

    /// Linear (row-major) elements of mip `m`. Missing source bytes stay zero.
    pub fn untile(&self, src: &[u8], m: usize) -> Vec<u8> {
        let p = &self.mips[m];
        let b = self.bpe as usize;
        let mut out = vec![0u8; (p.w * p.h) as usize * b];
        for y in 0..p.h {
            for x in 0..p.w {
                let s = self.addr(m, x, y) as usize;
                if let Some(e) = src.get(s..s + b) {
                    let d = (y * p.w + x) as usize * b;
                    out[d..d + b].copy_from_slice(e);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sizes_match_store_spacing() {
        // 64x64 BC1 (16x16 elements), 6 mips: whole chain in one 4 KiB tail.
        assert_eq!(Layout::new(Swizzle::S4K, 16, 16, 6, 8).unwrap().total, 4096);
        // 128x128 BC1, 7 mips: tail block + mip 0 (2 blocks).
        let l = Layout::new(Swizzle::S4K, 32, 32, 7, 8).unwrap();
        assert_eq!(l.total, 12288);
        assert_eq!(l.mips[0].offset, 4096);
        assert_eq!(l.mips[1].tail, Some((16, 0)));
        // 128x64 BC7, 6 mips.
        assert_eq!(Layout::new(Swizzle::S4K, 32, 16, 6, 16).unwrap().total, 16384);
    }
}
