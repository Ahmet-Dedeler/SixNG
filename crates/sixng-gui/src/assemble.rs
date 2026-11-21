//! Whole animals and characters from their parts.
//!
//! An animal's fragment (`crr_iguana_green_m_00`) holds the skeleton and a
//! placeholder plane; its textured body, eyes, fur shells, head and so on are
//! separate drawables named `<root>_<part>_00` (variants use other numbers or
//! suffixes: `_body_08_collarharness`). Named characters (`cs_ali`,
//! `u_m_m_farm_monster_man`, `player_story000_01`) are sets of clothing and
//! body drawables `<prefix>_<item>_<fit>_…` with fit tags like `mr1` / `fr1`.
//! Both are put together here for the viewer: the default parts, one copy each
//! (the first in the archive), in their bind pose.

use sixng_core::scan::{ArchiveIndex, Entry};
use std::collections::BTreeMap;

/// Animal roots: `cr` + class letter (m mammal, b bird, f fish, r reptile,
/// a amphibian, c crustacean, i insect).
pub fn is_animal(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() > 4 && name.starts_with("cr") && b[2].is_ascii_lowercase() && b[3] == b'_'
}

/// Fit tag (`mr1`, `fr1`, `mk1`, `fb1` …): sex + body type + number.
fn is_fit(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 3 && (b[0] == b'm' || b[0] == b'f') && b[1].is_ascii_lowercase() && b[2].is_ascii_digit()
}

/// Character prefix of a ped part: the tokens before `<item>_<fit>`, when
/// there are at least two (`cs_ali_head_fr1_…` → `cs_ali`). Generic items
/// (`tshirtcrewneck_mr1_…`) and the watch set `s01_` have none.
pub fn outfit_prefix(name: &str) -> Option<String> {
    let t: Vec<&str> = name.split('_').collect();
    if t[0] == "char" || t[0] == "fchar" || t[0] == "s01" || is_animal(name) {
        return None;
    }
    let i = t.iter().position(|x| is_fit(x))?;
    if i < 3 {
        return None;
    }
    Some(t[..i - 1].join("_"))
}

/// Ped item that is a whole body on its own (`legacypedworld_mr1_023_fullbody…`).
pub fn is_whole_ped(name: &str) -> bool {
    name.starts_with("legacypedworld_") && name.contains("_fullbody")
}

/// Ped part (has a fit tag), shown under People.
pub fn is_ped_part(name: &str) -> bool {
    name.split('_').any(is_fit) || name.starts_with("char_") || name.starts_with("fchar_")
}

const ANIMAL_SKIP: &[&str] = &[
    "generic", "costume", "collar", "harness", "hat", "badge", "ribbon", "bandana", "cape", "lace", "flower", "knife", "vest", "rope", "tranquilizer", "loop", "shadesaviators", "accessories", "human", "stick", "corpse", "roadkill", "gut", "gonad", "bone",
];

/// Default parts of an animal root: `<root>_<words>_00` without accessories.
/// Roots without such parts fall back to `<root minus _NN>_<words>_00`.
pub fn animal_parts<'a>(ix: &'a ArchiveIndex, root: &str) -> Vec<&'a Entry> {
    let root = root.to_ascii_lowercase();
    let mut prefixes = vec![format!("{root}_")];
    if let Some((stem, n)) = root.rsplit_once('_') {
        if n.chars().all(|c| c.is_ascii_digit()) {
            prefixes.push(format!("{stem}_"));
        }
    }
    for p in prefixes {
        let mut by_name: BTreeMap<String, &Entry> = BTreeMap::new();
        for e in &ix.entries {
            if e.rtype.as_deref() != Some("dr") {
                continue;
            }
            let Some(n) = &e.name else { continue };
            let n = n.to_ascii_lowercase();
            let Some(rest) = n.strip_prefix(&p) else { continue };
            let Some(words) = rest.strip_suffix("_00") else { continue };
            let toks: Vec<&str> = words.split('_').collect();
            if toks.is_empty() || toks.iter().any(|t| t.is_empty() || !t.chars().all(|c| c.is_ascii_lowercase()) || ANIMAL_SKIP.contains(t)) {
                continue;
            }
            by_name.entry(n.clone()).and_modify(|x| if e.off < x.off { *x = e }).or_insert(e);
        }
        if !by_name.is_empty() {
            return by_name.into_values().collect();
        }
    }
    Vec::new()
}

/// Outfit slot of a ped item word (`tshirtcrewneck` → top); None = left out
/// (hats, bags, watches, jewellery other than piercings, masks …).
fn slot(item: &str) -> Option<&'static str> {
    const SLOTS: &[(&str, &str)] = &[
        ("hair", "hair"),
        ("beard", "beard"),
        ("brows", "brows"),
        ("lashes", "lashes"),
        ("eyes", "eyes"),
        ("teeth", "teeth"),
        ("bodyupper", "body_upper"),
        ("bodylower", "body_lower"),
        ("fullbody", "full"),
        ("dress", "full"),
        ("jumpsuit", "full"),
        ("overalls", "full"),
        ("jacket", "outer"),
        ("coat", "outer"),
        ("blazer", "outer"),
        ("cardigan", "outer"),
        ("tshirt", "top"),
        ("shirt", "top"),
        ("sweatshirt", "top"),
        ("hoodie", "top"),
        ("tanktop", "top"),
        ("blouse", "top"),
        ("bra", "top"),
        ("pants", "bottom"),
        ("shorts", "bottom"),
        ("skirt", "bottom"),
        ("undies", "bottom"),
        ("leggings", "bottom"),
        ("boots", "feet"),
        ("shoes", "feet"),
        ("sneakers", "feet"),
        ("sandals", "feet"),
        ("heels", "feet"),
        ("piercing", "piercing"),
    ];
    if item == "head" {
        return Some("head");
    }
    // longest matching word first (sweatshirt before shirt)
    SLOTS.iter().filter(|(w, _)| item.starts_with(w)).max_by_key(|(w, _)| w.len()).map(|(_, s)| *s)
}

/// One part per outfit slot for a character prefix (the first in name order),
/// plus the eyes mesh that goes with the head.
pub fn outfit_parts<'a>(ix: &'a ArchiveIndex, prefix: &str) -> Vec<&'a Entry> {
    let p = format!("{}_", prefix.to_ascii_lowercase());
    let mut by_name: BTreeMap<String, &Entry> = BTreeMap::new();
    for e in &ix.entries {
        if e.rtype.as_deref() != Some("dr") {
            continue;
        }
        let Some(n) = &e.name else { continue };
        let n = n.to_ascii_lowercase();
        if !n.starts_with(&p) || outfit_prefix(&n).as_deref() != Some(prefix) {
            continue;
        }
        if ["_cull", "_alt_", "_fp", "eyecap", "tattooprojection", "holster"].iter().any(|x| n.contains(x)) {
            continue;
        }
        by_name.entry(n.clone()).and_modify(|x| if e.off < x.off { *x = e }).or_insert(e);
    }
    let mut chosen: BTreeMap<String, &Entry> = BTreeMap::new();
    for (n, e) in &by_name {
        let item = n[p.len()..].split('_').next().unwrap_or("");
        let Some(s) = slot(item) else { continue };
        let key = if s == "head" && n.contains("_eyes") { "head_eyes".to_string() } else { s.to_string() };
        chosen.entry(key).or_insert(e);
    }
    // a one-piece outfit replaces top and bottom
    if chosen.contains_key("full") {
        chosen.remove("top");
        chosen.remove("bottom");
    }
    chosen.into_values().collect()
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Readable names for animals and people; None for other models.
/// `crr_iguana_green_m_00` → "Iguana (green, male)",
/// `crm_dog_rottweiler_f_01` → "Dog (rottweiler, female) 2",
/// `cs_ali` → "Ali", `u_m_m_farm_monster_man` → "Farm monster man",
/// `tshirtcrewneck_mr1_019_…` → "Tshirtcrewneck (male) 019".
pub fn pretty_name(name: &str) -> Option<String> {
    let n = name.to_ascii_lowercase();
    if is_animal(&n) {
        let t: Vec<&str> = n[4..].split('_').collect();
        let i = t.iter().position(|x| *x == "m" || *x == "f").unwrap_or(t.len());
        let sex = t.get(i).map(|x| if *x == "m" { "male" } else { "female" });
        let mut after: Vec<&str> = t.get(i + 1..).map(|x| x.to_vec()).unwrap_or_default();
        let mut num = None;
        if after.first().map_or(false, |x| x.chars().all(|c| c.is_ascii_digit())) {
            num = after.remove(0).parse::<u32>().ok();
        }
        if after.last() == Some(&"00") {
            after.pop();
        }
        let species = t.first()?;
        let mut extra: Vec<String> = t[1..i.min(t.len())].iter().map(|s| s.to_string()).collect();
        if let Some(s) = sex {
            extra.push(s.into());
        }
        let mut out = cap(species);
        if !extra.is_empty() {
            out += &format!(" ({})", extra.join(", "));
        }
        if let Some(k) = num.filter(|k| *k > 0) {
            out += &format!(" {}", k + 1);
        }
        if !after.is_empty() {
            out += &format!(" – {}", after.join(" "));
        }
        return Some(out);
    }
    if let Some(p) = n.strip_prefix("player_story") {
        let t: Vec<&str> = p.split('_').collect();
        return Some(format!("Player (story {}, outfit {})", t.first().unwrap_or(&""), t.get(1).unwrap_or(&"")));
    }
    if !n.contains('_') || n.split('_').any(is_fit) {
        // ped part: words before the fit tag + sex + number
        let t: Vec<&str> = n.split('_').collect();
        let i = t.iter().position(|x| is_fit(x))?;
        let words = &t[..i];
        let sex = if t[i].starts_with('m') { "male" } else { "female" };
        let num = t.get(i + 1).filter(|x| x.chars().all(|c| c.is_ascii_digit())).map(|x| format!(" {x}")).unwrap_or_default();
        let head = words.iter().position(|w| *w != "cs" && *w != "u" && *w != "sc" && *w != "hc").unwrap_or(0);
        return Some(format!("{} ({sex}){num}", cap(&words[head..].join(" "))));
    }
    // character prefixes
    let t: Vec<&str> = n.split('_').collect();
    match t[0] {
        "cs" | "sc" | "ig" => Some(cap(&t[1..].join(" "))),
        "hc" => Some(format!("{} heads", cap(&t[1..].join(" ")))),
        // u_<sex>_<age>_<name…>
        "u" | "a" | "s" if t.len() > 3 => Some(cap(&t[3..].join(" "))),
        _ => None,
    }
}

/// Label materials that have no colour texture (BCPack-only, or skin the
/// game builds at runtime): they render plain white, and the material list
/// says so instead of inventing a colour.
pub fn mark_missing_colour(model: &mut crate::glb::Model) {
    for m in model.materials.iter_mut() {
        if m.base.is_some() || matches!(m.class.as_str(), "decal" | "glass" | "window" | "emissive" | "water" | "car_paint") {
            continue;
        }
        if m.gap.is_empty() || m.gap == "ok" {
            m.gap = "no colour texture".into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names() {
        assert_eq!(pretty_name("crr_iguana_green_m_00").unwrap(), "Iguana (green, male)");
        assert_eq!(pretty_name("crm_dog_rottweiler_f_01").unwrap(), "Dog (rottweiler, female) 2");
        assert_eq!(pretty_name("cs_ali").unwrap(), "Ali");
        assert_eq!(pretty_name("u_m_m_farm_monster_man").unwrap(), "Farm monster man");
        assert_eq!(pretty_name("tshirtcrewneck_mr1_019_compression_cc_un_short_cull_00").unwrap(), "Tshirtcrewneck (male) 019");
        assert_eq!(pretty_name("crr_iguana_green_m_00_body_00").unwrap(), "Iguana (green, male) – body");
        assert_eq!(outfit_prefix("cs_ali_head_fr1_afr_000_m_full").as_deref(), Some("cs_ali"));
        assert_eq!(outfit_prefix("tshirtcrewneck_mr1_004_loose").as_deref(), None);
        assert_eq!(outfit_prefix("player_story000_01_tshirtcrewneck_mr1_000").as_deref(), Some("player_story000_01"));
        assert_eq!(slot("sweatshirtcrewneck"), Some("top"));
        assert_eq!(slot("jacketfield"), Some("outer"));
        assert!(pretty_name("prop_bench_01a").is_none());
    }
}
