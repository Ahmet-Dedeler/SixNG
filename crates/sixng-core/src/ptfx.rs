//! Particle effect dictionaries (what is read so far).
//!
//! The effect dictionaries are unnamed resources in data.rpf (and the
//! content packs). Each holds the effect and emitter rules of one effect
//! set: `scr_arc_tl` holds `scr_arc_tl_candle_flame_flare`,
//! `scr_arc_tl_shadowy_figure` …, and small keyframe textures
//! (`keyframeTexture`, R16G16B16A16_FLOAT, 16 x 4 texels: property curves
//! over a particle's life, sampled at 16 points). The rules' names are
//! plain strings in the system page; their records (emitter parameters,
//! sprite references) are not decoded. The sprite sheets are not embedded,
//! and no 64-bit or 32-bit texture-store hash of any `.ttpf` store occurs in
//! the dictionaries, so how a rule names its sprite is still open.
use crate::drawable::DrawableInfo;
use crate::resource::Resource;
use serde::Serialize;

pub const KEYFRAME_TEXTURE: &str = "keyframeTexture";

#[derive(Debug, Clone, Serialize)]
pub struct PtfxDict {
    pub id: String,
    /// The dictionary's own name (`scr_arc_tl`): the shortest name that the
    /// rule names extend with `_…`. None when no string qualifies.
    pub name: Option<String>,
    /// Effect and emitter rule names, in stored order (most are `<name>_…`;
    /// a dictionary can also carry rules of shared sets, `MG3_blood_…`).
    pub rules: Vec<String>,
    /// Other strings (shader variable names such as `intensity`, stray bytes).
    pub other: Vec<String>,
    pub keyframe_textures: usize,
    pub bytes: u64,
}

fn rule_like(s: &str) -> bool {
    s.len() >= 6 && s.as_bytes()[0].is_ascii_alphabetic() && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') && s.matches('_').count() >= 2
}

/// A particle effect dictionary: has keyframe textures and no geometry.
pub fn is_ptfx(info: &DrawableInfo) -> bool {
    info.geometries.is_empty() && info.embedded_textures.iter().any(|t| t.name == KEYFRAME_TEXTURE)
}

/// Names of a dictionary from its strings (see `PtfxDict`).
pub fn describe(id: &str, bytes: u64, info: &DrawableInfo) -> PtfxDict {
    let n = info.embedded_textures.iter().filter(|t| t.name == KEYFRAME_TEXTURE).count();
    names(id, bytes, &info.strings, n)
}

fn names(id: &str, bytes: u64, strings: &[String], keyframe_textures: usize) -> PtfxDict {
    let cand: Vec<&String> = strings.iter().filter(|s| rule_like(s) || (s.len() >= 4 && s.contains('_') && s.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_'))).collect();
    // the dictionary name: the string that prefixes the most others (ties: shortest)
    let name = cand
        .iter()
        .map(|s| (cand.iter().filter(|o| o.len() > s.len() + 1 && o.starts_with(s.as_str()) && o.as_bytes()[s.len()] == b'_').count(), *s))
        .filter(|(n, _)| *n > 0)
        .max_by(|a, b| a.0.cmp(&b.0).then(b.1.len().cmp(&a.1.len())))
        .map(|(_, s)| s.clone());
    let (rules, other): (Vec<String>, Vec<String>) = strings
        .iter()
        .filter(|s| s.as_str() != KEYFRAME_TEXTURE && Some(*s) != name.as_ref())
        .cloned()
        .partition(|s| rule_like(s));
    PtfxDict { id: id.to_string(), name, rules, other, keyframe_textures, bytes }
}

fn half(h: u16) -> f32 {
    let s = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let e = ((h >> 10) & 0x1f) as i32;
    let m = (h & 0x3ff) as f32;
    match e {
        0 => s * m * 2f32.powi(-24),
        31 => {
            if m == 0.0 {
                s * f32::INFINITY
            } else {
                f32::NAN
            }
        }
        _ => s * (1.0 + m / 1024.0) * 2f32.powi(e - 15),
    }
}

/// The keyframe textures as float rows: per texture, `height` rows of
/// `width` RGBA samples (stored order; what each row drives is not known).
pub fn keyframes(r: &Resource, info: &DrawableInfo) -> Vec<Vec<Vec<[f32; 4]>>> {
    let mut out = Vec::new();
    for t in info.embedded_textures.iter().filter(|t| t.name == KEYFRAME_TEXTURE && t.dxgi_format == 10) {
        let Ok((_, lin)) = crate::texture::mip0_linear(r, t, true) else { continue };
        let (w, h) = (t.width as usize, t.height as usize);
        if lin.len() < w * h * 8 {
            continue;
        }
        let rows = (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| {
                        let o = (y * w + x) * 8;
                        let c = |k: usize| half(u16::from_le_bytes([lin[o + 2 * k], lin[o + 2 * k + 1]]));
                        [c(0), c(1), c(2), c(3)]
                    })
                    .collect()
            })
            .collect();
        out.push(rows);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves() {
        assert_eq!(half(0x3c00), 1.0);
        assert_eq!(half(0xc000), -2.0);
        assert_eq!(half(0x3800), 0.5);
        assert_eq!(half(0), 0.0);
    }

    #[test]
    fn names() {
        let strings: Vec<String> = ["scr_arc_tl_candle_flame_flare", "scr_arc_tl_shadowy_figure", "keyframeTexture", "scr_arc_tl_shadow_smoke", "scr_arc_tl", "intensity"].iter().map(|s| s.to_string()).collect();
        let d = names("x", 1, &strings, 0);
        assert_eq!(d.name.as_deref(), Some("scr_arc_tl"));
        assert_eq!(d.rules, ["scr_arc_tl_candle_flame_flare", "scr_arc_tl_shadowy_figure", "scr_arc_tl_shadow_smoke"]);
        assert_eq!(d.other, ["intensity"]);
    }
}
