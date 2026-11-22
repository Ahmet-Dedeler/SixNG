//! Shared ped skeleton for skinning ped clothing.
//!
//! Ped clothing drawables carry only their own `GM_U01_*` girth helper
//! bones. Their vertex joint indices go through the model's bone palette
//! (`drawable::Model::bone_palette`, sorted joaat name hashes) to bones that
//! are either those helpers or bones of the shared ped skeleton. That
//! skeleton (596 bones, `SKEL_ROOT` …) is in the `ambient_femalebody` /
//! `ambient_malebody` fragments of `packs.rpf`; it is loaded once from the
//! packs index in the work directory.
use crate::drawable::{find_skeleton, Bone};
use crate::meta7a::joaat;
use crate::resource::Resource;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

pub struct PedSkeleton {
    /// Name of the fragment the skeleton came from.
    pub source: String,
    pub bones: Vec<Bone>,
    /// joaat(bone name) -> index into `bones`.
    pub by_hash: HashMap<u32, usize>,
}

/// Fragment holding the reference skeleton for a body (female or male).
pub fn source_name(female: bool) -> &'static str {
    if female {
        "ambient_femalebody"
    } else {
        "ambient_malebody"
    }
}

fn packs_index() -> Option<&'static crate::scan::ArchiveIndex> {
    static IX: OnceLock<Option<crate::scan::ArchiveIndex>> = OnceLock::new();
    IX.get_or_init(|| {
        let work = crate::names::work_dir()?;
        crate::scan::load_index(&crate::scan::index_path(&work.join("index"), "packs.rpf")).ok()
    })
    .as_ref()
}

fn load(game_root: &Path, female: bool) -> Option<PedSkeleton> {
    load_fragment(game_root, source_name(female))
}

/// Skeleton of the named fragment in packs.rpf (largest copy).
fn load_fragment(game_root: &Path, name: &str) -> Option<PedSkeleton> {
    let ix = packs_index()?;
    let e = ix.entries.iter().filter(|e| e.name.as_deref() == Some(name) && e.rtype.as_deref() == Some("ft")).max_by_key(|e| e.usize)?;
    let r = Resource::load(&game_root.join("packs.rpf"), e).ok()?;
    let bones = find_skeleton(&r);
    if bones.is_empty() || bones.iter().any(|b| b.bind_matrix.is_none()) {
        return None;
    }
    let by_hash = bones.iter().enumerate().map(|(i, b)| (joaat(&b.name), i)).collect();
    Some(PedSkeleton { source: name.to_string(), bones, by_hash })
}

/// The shared ped skeleton (cached), or None when packs.rpf is not indexed
/// in the work directory or the fragment is missing.
pub fn get(game_root: &Path, female: bool) -> Option<&'static PedSkeleton> {
    static F: OnceLock<Option<PedSkeleton>> = OnceLock::new();
    static M: OnceLock<Option<PedSkeleton>> = OnceLock::new();
    let cell = if female { &F } else { &M };
    cell.get_or_init(|| load(game_root, female)).as_ref()
}

/// The skeleton of an animal part (`crb_eagle_bald_m_00_body_00`): animal
/// parts are skinned to the bones of their root fragment
/// (`crb_eagle_bald_m_00`, which holds only that skeleton and a placeholder
/// plane). The root is the longest `_`-prefix of the name that is a fragment
/// in packs.rpf. Cached per root; None when there is none.
pub fn animal(game_root: &Path, part_name: &str) -> Option<&'static PedSkeleton> {
    use std::sync::Mutex;
    static CACHE: OnceLock<Mutex<HashMap<String, Option<&'static PedSkeleton>>>> = OnceLock::new();
    if !crate::outfit::is_animal(part_name) {
        return None;
    }
    let ix = packs_index()?;
    let toks: Vec<&str> = part_name.split('_').collect();
    let root = (2..toks.len()).rev().map(|k| toks[..k].join("_")).find(|r| ix.entries.iter().any(|e| e.rtype.as_deref() == Some("ft") && e.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(r))))?;
    let cache = CACHE.get_or_init(Default::default);
    if let Some(s) = cache.lock().ok()?.get(&root) {
        return *s;
    }
    let real = ix.entries.iter().find(|e| e.rtype.as_deref() == Some("ft") && e.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(&root)))?.name.clone()?;
    let s: Option<&'static PedSkeleton> = load_fragment(game_root, &real).map(|s| &*Box::leak(Box::new(s)));
    cache.lock().ok()?.insert(root, s);
    s
}

/// Ped part name: has a fit tag (`mr1`, `fr1`, `mk1` …: sex + body type +
/// number) as one of its `_` tokens.
pub fn is_ped_name(name: &str) -> bool {
    name.split('_').any(crate::outfit::is_fit)
}

/// Female rig unless the name has the male rig token (`_mr1`).
pub fn is_female_name(name: &str) -> bool {
    !name.contains("_mr1")
}
