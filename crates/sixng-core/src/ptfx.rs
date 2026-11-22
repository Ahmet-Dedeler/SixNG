//! Particle effect dictionaries (what is read so far).
//!
//! The effect dictionaries are unnamed resources in data.rpf (and the
//! content packs). Each holds the effect and emitter rules of one effect
//! set: `scr_arc_tl` holds `scr_arc_tl_candle_flame_flare`,
//! `scr_arc_tl_shadowy_figure` …, and small keyframe textures
//! (`keyframeTexture`, R16G16B16A16_FLOAT, 16 x 4 texels: property curves
//! over a particle's life, sampled at 16 points). The rules' names are
//! plain strings in the system page. `rules` reads the rule tables: which
//! emitter and particle rules each effect fires (its events), each rule's
//! keyframe properties (id hash and keys over normalised time; the property
//! names are not recovered) and each particle rule's keyframe texture.
//! Behaviour types and most scalar fields are not decoded. The sprite
//! sheets are not embedded, and no 64-bit or 32-bit texture-store hash of
//! any `.ttpf` store occurs in the dictionaries, so how a rule names its
//! sprite is still open.
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


/// One keyframe property: an id hash (property names are not recovered)
/// and its keys over normalised time.
#[derive(Debug, Clone, Serialize)]
pub struct KfProp {
    /// Byte offset of the property inside its rule or behaviour.
    pub at: u32,
    pub id: String,
    /// (time 0..1, value vec4). Two-value properties use x/y as min/max.
    pub keys: Vec<(f32, [f32; 4])>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PtfxEvent {
    pub emitter: Option<String>,
    pub particle: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PtfxEffect {
    pub name: String,
    /// f32 at +0x18 (4.2 on every arcade effect; meaning not known).
    pub f18: f32,
    pub events: Vec<PtfxEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PtfxEmitter {
    pub name: String,
    /// The ten property slots at +0x40 + 0x40·i, in order.
    pub props: Vec<KfProp>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PtfxParticle {
    pub name: String,
    /// Index into this dictionary's keyframe textures (`keyframes` order).
    pub keyframe_texture: Option<usize>,
    /// Behaviours (list at +0x188): the keyframe properties found in each.
    pub behaviours: Vec<Vec<KfProp>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PtfxRules {
    pub effects: Vec<PtfxEffect>,
    pub emitters: Vec<PtfxEmitter>,
    pub particles: Vec<PtfxParticle>,
}

fn names_by_hash(strings: &[String]) -> std::collections::HashMap<u32, String> {
    strings.iter().map(|s| (crate::meta7a::joaat(s), s.clone())).collect()
}

/// A keyframe property at `q`: key list pointer at +0x10, u16 count and
/// capacity at +0x18, id hash stored twice at +0x30. Keys are 0x20 bytes:
/// time at +0, 1 / (time - previous time) at +4, value vec4 at +0x10.
fn kf_prop(r: &Resource, q: usize, at: usize) -> Option<KfProp> {
    let id = r.u32(q + 0x30);
    // the words around the key list and the id are zero in real properties
    // (a float pair such as 1.0, 1.0 can look like a doubled id otherwise)
    if id == 0 || r.u32(q + 0x34) != id || r.u64(q + 0x20) != 0 || r.u64(q + 0x28) != 0 || r.u64(q + 0x38) != 0 {
        return None;
    }
    let c = r.u32(q + 0x18);
    let (n, cap) = ((c & 0xffff) as usize, (c >> 16) as usize);
    if n != cap || n > 256 {
        return None;
    }
    let keys = if n == 0 {
        Vec::new()
    } else {
        let kp = r.sys_off(r.u64(q + 0x10))?;
        (0..n).map(|k| {
            let e = kp + 0x20 * k;
            (r.f32(e), [r.f32(e + 0x10), r.f32(e + 0x14), r.f32(e + 0x18), r.f32(e + 0x1c)])
        }).collect()
    };
    Some(KfProp { at: at as u32, id: format!("{id:08x}"), keys })
}

/// (hash, object offset) entries of a hash/pointer table: hash array at
/// +0x20, u16 count at +0x28, pointer array at +0x30.
fn table(r: &Resource, d: usize) -> Vec<(u32, usize)> {
    let n = (r.u32(d + 0x28) & 0xffff) as usize;
    let (Some(ha), Some(pa)) = (r.sys_off(r.u64(d + 0x20)), r.sys_off(r.u64(d + 0x30))) else { return Vec::new() };
    (0..n.min(4096)).filter_map(|i| Some((r.u32(ha + 4 * i), r.sys_off(r.u64(pa + 8 * i))?))).collect()
}

/// Effect, emitter and particle rules of a dictionary. Root: name at +0x10,
/// tables of effect rules (+0x28), emitter rules (+0x30) and particle rules
/// (+0x38). An effect's events (array at +0x38, u16 count at +0x40) each
/// pair an emitter rule (+0x50) with a particle rule (+0x58). A particle
/// rule points to its keyframe texture at +0x388.
pub fn rules(r: &Resource, info: &DrawableInfo) -> Option<PtfxRules> {
    let names = names_by_hash(&info.strings);
    let tab = |k: usize| r.sys_off(r.u64(0x28 + 8 * k)).map(|d| table(r, d)).unwrap_or_default();
    let (eff, emi, par) = (tab(0), tab(1), tab(2));
    if eff.is_empty() && emi.is_empty() && par.is_empty() {
        return None;
    }
    let name = |h: u32| names.get(&h).cloned().unwrap_or_else(|| format!("{h:08x}"));
    let by_off: std::collections::HashMap<usize, String> = emi.iter().chain(par.iter()).map(|&(h, o)| (o, name(h))).collect();
    let effects = eff
        .iter()
        .map(|&(h, o)| {
            let n = (r.u32(o + 0x40) & 0xffff) as usize;
            let events = r
                .sys_off(r.u64(o + 0x38))
                .map(|ev| {
                    (0..n.min(256))
                        .filter_map(|i| r.sys_off(r.u64(ev + 8 * i)))
                        .map(|e| {
                            let rule = |k: usize| r.sys_off(r.u64(e + k)).and_then(|p| by_off.get(&p).cloned());
                            PtfxEvent { emitter: rule(0x50), particle: rule(0x58) }
                        })
                        .collect()
                })
                .unwrap_or_default();
            PtfxEffect { name: name(h), f18: r.f32(o + 0x18), events }
        })
        .collect();
    let emitters = emi
        .iter()
        .map(|&(h, o)| PtfxEmitter { name: name(h), props: (0..10).filter_map(|i| kf_prop(r, o + 0x40 + 0x40 * i, 0x40 + 0x40 * i)).collect() })
        .collect();
    let kf_tex: Vec<u64> = info.embedded_textures.iter().filter(|t| t.name == KEYFRAME_TEXTURE && t.dxgi_format == 10).map(|t| t.struct_off).collect();
    let particles = par
        .iter()
        .map(|&(h, o)| {
            let keyframe_texture = r.sys_off(r.u64(o + 0x388)).and_then(|p| kf_tex.iter().position(|&s| s as usize == p));
            let nb = (r.u32(o + 0x190) & 0xffff) as usize;
            let behaviours = r
                .sys_off(r.u64(o + 0x188))
                .map(|bl| {
                    (0..nb.min(64))
                        .filter_map(|i| r.sys_off(r.u64(bl + 8 * i)))
                        .map(|b| (0..8).filter_map(|k| kf_prop(r, b + 0x40 * k, 0x40 * k)).collect::<Vec<_>>())
                        .filter(|v: &Vec<KfProp>| !v.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            PtfxParticle { name: name(h), keyframe_texture, behaviours }
        })
        .collect();
    Some(PtfxRules { effects, emitters, particles })
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
    fn keyframe_prop() {
        let mut sys = vec![0u8; 0x200];
        // property at 0x40: keys at 0x100, two keys; id 0xabcd1234 twice
        sys[0x50..0x58].copy_from_slice(&(0x5000_0100u64).to_le_bytes());
        sys[0x58..0x5c].copy_from_slice(&0x0002_0002u32.to_le_bytes());
        sys[0x70..0x74].copy_from_slice(&0xabcd_1234u32.to_le_bytes());
        sys[0x74..0x78].copy_from_slice(&0xabcd_1234u32.to_le_bytes());
        let f = |sys: &mut Vec<u8>, o: usize, v: f32| sys[o..o + 4].copy_from_slice(&v.to_le_bytes());
        f(&mut sys, 0x110, 3.0);
        f(&mut sys, 0x114, 4.0);
        f(&mut sys, 0x120, 1.0);
        f(&mut sys, 0x130, 0.5);
        let r = Resource { sys, gfx: vec![], gfx_base: 0 };
        let p = kf_prop(&r, 0x40, 0x40).unwrap();
        assert_eq!(p.id, "abcd1234");
        assert_eq!(p.keys, vec![(0.0, [3.0, 4.0, 0.0, 0.0]), (1.0, [0.5, 0.0, 0.0, 0.0])]);
        // a float pair is not an id when the padding is not zero
        let mut r = r;
        r.sys[0x60] = 1;
        assert!(kf_prop(&r, 0x40, 0x40).is_none());
    }

    #[test]
    fn dict_names() {
        let strings: Vec<String> = ["scr_arc_tl_candle_flame_flare", "scr_arc_tl_shadowy_figure", "keyframeTexture", "scr_arc_tl_shadow_smoke", "scr_arc_tl", "intensity"].iter().map(|s| s.to_string()).collect();
        let d = names("x", 1, &strings, 0);
        assert_eq!(d.name.as_deref(), Some("scr_arc_tl"));
        assert_eq!(d.rules, ["scr_arc_tl_candle_flame_flare", "scr_arc_tl_shadowy_figure", "scr_arc_tl_shadow_smoke"]);
        assert_eq!(d.other, ["intensity"]);
    }
}
