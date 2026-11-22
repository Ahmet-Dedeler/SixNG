//! Whole animals and characters from their parts.
//!
//! An animal's fragment (`crr_iguana_green_m_00`) holds the skeleton and a
//! placeholder plane; its textured body, eyes, fur shells, head and so on are
//! separate drawables named `<root>_<part>_00` (variants use other numbers or
//! suffixes: `_body_08_collarharness`). Named characters (`cs_ali`,
//! `u_m_m_farm_monster_man`, `player_story000_01`) are sets of clothing and
//! body drawables `<prefix>_<item>_<fit>_…` with fit tags like `mr1` / `fr1`.
//! Both are put together here for the viewer: the default parts, one copy each
//! (the largest copy, LOD0), in their bind pose.

use sixng_core::outfit::is_fit;
pub use sixng_core::outfit::{
    animal_options, animal_parts, is_animal, is_ped_part, is_whole_ped, outfit_options,
    outfit_parts, outfit_prefix, PartOpt,
};

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
        let i = t
            .iter()
            .position(|x| *x == "m" || *x == "f")
            .unwrap_or(t.len());
        let sex = t.get(i).map(|x| if *x == "m" { "male" } else { "female" });
        let mut after: Vec<&str> = t.get(i + 1..).map(|x| x.to_vec()).unwrap_or_default();
        let mut num = None;
        if after
            .first()
            .map_or(false, |x| x.chars().all(|c| c.is_ascii_digit()))
        {
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
        return Some(format!(
            "Player (story {}, outfit {})",
            t.first().unwrap_or(&""),
            t.get(1).unwrap_or(&"")
        ));
    }
    if !n.contains('_') || n.split('_').any(is_fit) {
        // ped part: words before the fit tag + sex + number
        let t: Vec<&str> = n.split('_').collect();
        let i = t.iter().position(|x| is_fit(x))?;
        let words = &t[..i];
        let sex = if t[i].starts_with('m') {
            "male"
        } else {
            "female"
        };
        let num = t
            .get(i + 1)
            .filter(|x| x.chars().all(|c| c.is_ascii_digit()))
            .map(|x| format!(" {x}"))
            .unwrap_or_default();
        let head = words
            .iter()
            .position(|w| *w != "cs" && *w != "u" && *w != "sc" && *w != "hc")
            .unwrap_or(0);
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
        if m.base.is_some()
            || matches!(
                m.class.as_str(),
                "decal" | "glass" | "window" | "emissive" | "water" | "car_paint"
            )
        {
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
        assert_eq!(
            pretty_name("crr_iguana_green_m_00").unwrap(),
            "Iguana (green, male)"
        );
        assert_eq!(
            pretty_name("crm_dog_rottweiler_f_01").unwrap(),
            "Dog (rottweiler, female) 2"
        );
        assert_eq!(pretty_name("cs_ali").unwrap(), "Ali");
        assert_eq!(
            pretty_name("u_m_m_farm_monster_man").unwrap(),
            "Farm monster man"
        );
        assert_eq!(
            pretty_name("tshirtcrewneck_mr1_019_compression_cc_un_short_cull_00").unwrap(),
            "Tshirtcrewneck (male) 019"
        );
        assert_eq!(
            pretty_name("crr_iguana_green_m_00_body_00").unwrap(),
            "Iguana (green, male) – body"
        );
        assert!(pretty_name("prop_bench_01a").is_none());
    }
}
