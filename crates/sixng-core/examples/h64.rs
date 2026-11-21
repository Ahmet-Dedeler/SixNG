//! Research: test 64-bit hash functions of candidate names against ttpf
//! texture hashes (and their halves). h64 <game> <names.txt...>
use std::collections::HashSet;
fn fnv1a(s: &[u8]) -> u64 { let mut h = 0xcbf29ce484222325u64; for &c in s { h ^= c as u64; h = h.wrapping_mul(0x100000001b3); } h }
fn fnv1(s: &[u8]) -> u64 { let mut h = 0xcbf29ce484222325u64; for &c in s { h = h.wrapping_mul(0x100000001b3); h ^= c as u64; } h }
fn oaat64(s: &[u8]) -> u64 { let mut h = 0u64; for &c in s { h = h.wrapping_add(c as u64); h = h.wrapping_add(h << 10); h ^= h >> 6; } h = h.wrapping_add(h << 3); h ^= h >> 11; h.wrapping_add(h << 15) }
fn murmur64a(s: &[u8], seed: u64) -> u64 {
    let m = 0xc6a4a7935bd1e995u64; let r = 47; let mut h = seed ^ (s.len() as u64).wrapping_mul(m);
    let mut ch = s.chunks_exact(8);
    for c in &mut ch { let mut k = u64::from_le_bytes(c.try_into().unwrap()); k = k.wrapping_mul(m); k ^= k >> r; k = k.wrapping_mul(m); h ^= k; h = h.wrapping_mul(m); }
    let t = ch.remainder();
    if !t.is_empty() { for (i, &b) in t.iter().enumerate() { h ^= (b as u64) << (8 * i); } h = h.wrapping_mul(m); }
    h ^= h >> r; h = h.wrapping_mul(m); h ^= h >> r; h
}
const P1: u64 = 11400714785074694791; const P2: u64 = 14029467366897019727; const P3: u64 = 1609587929392839161; const P4: u64 = 9650029242287828579; const P5: u64 = 2870177450012600261;
fn rnd(a: u64, i: u64) -> u64 { a.wrapping_add(i.wrapping_mul(P2)).rotate_left(31).wrapping_mul(P1) }
fn mrg(a: u64, v: u64) -> u64 { (a ^ rnd(0, v)).wrapping_mul(P1).wrapping_add(P4) }
fn xxh64(s: &[u8], seed: u64) -> u64 {
    let n = s.len(); let mut p = 0; let mut h;
    if n >= 32 { let (mut v1, mut v2, mut v3, mut v4) = (seed.wrapping_add(P1).wrapping_add(P2), seed.wrapping_add(P2), seed, seed.wrapping_sub(P1));
        while p + 32 <= n { let r = |o: usize| u64::from_le_bytes(s[o..o + 8].try_into().unwrap()); v1 = rnd(v1, r(p)); v2 = rnd(v2, r(p + 8)); v3 = rnd(v3, r(p + 16)); v4 = rnd(v4, r(p + 24)); p += 32; }
        h = v1.rotate_left(1).wrapping_add(v2.rotate_left(7)).wrapping_add(v3.rotate_left(12)).wrapping_add(v4.rotate_left(18)); h = mrg(mrg(mrg(mrg(h, v1), v2), v3), v4);
    } else { h = seed.wrapping_add(P5); }
    h = h.wrapping_add(n as u64);
    while p + 8 <= n { let k = rnd(0, u64::from_le_bytes(s[p..p + 8].try_into().unwrap())); h ^= k; h = h.rotate_left(27).wrapping_mul(P1).wrapping_add(P4); p += 8; }
    if p + 4 <= n { h ^= (u32::from_le_bytes(s[p..p + 4].try_into().unwrap()) as u64).wrapping_mul(P1); h = h.rotate_left(23).wrapping_mul(P2).wrapping_add(P3); p += 4; }
    while p < n { h ^= (s[p] as u64).wrapping_mul(P5); h = h.rotate_left(11).wrapping_mul(P1); p += 1; }
    h ^= h >> 33; h = h.wrapping_mul(P2); h ^= h >> 29; h = h.wrapping_mul(P3); h ^= h >> 32; h
}
fn crc64(s: &[u8], poly: u64, refl: bool, init: u64, xorout: u64) -> u64 {
    let mut c = init;
    for &b in s {
        if refl { c ^= b as u64; for _ in 0..8 { c = if c & 1 != 0 { (c >> 1) ^ poly } else { c >> 1 }; } }
        else { c ^= (b as u64) << 56; for _ in 0..8 { c = if c >> 63 != 0 { (c << 1) ^ poly } else { c << 1 }; } }
    }
    c ^ xorout
}
fn joaat(s: &[u8]) -> u32 { let mut h = 0u32; for &c in s { h = h.wrapping_add(c as u32); h = h.wrapping_add(h << 10); h ^= h >> 6; } h = h.wrapping_add(h << 3); h ^= h >> 11; h.wrapping_add(h << 15) }
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let set = sixng_core::ttpf::TtpfSet::open_dir(&std::path::Path::new(&a[1]).join("data/ttdb"));
    let full: HashSet<u64> = set.stores.iter().flat_map(|s| s.textures.iter().map(|t| t.hash)).collect();
    let lo: HashSet<u32> = full.iter().map(|h| *h as u32).collect();
    let hi: HashSet<u32> = full.iter().map(|h| (*h >> 32) as u32).collect();
    eprintln!("{} ttpf hashes", full.len());
    let mut names: Vec<String> = Vec::new();
    for f in &a[2..] { names.extend(std::fs::read_to_string(f).unwrap().split_whitespace().map(|s| s.to_string())); }
    let fs: Vec<(&str, Box<dyn Fn(&[u8]) -> u64>)> = vec![
        ("fnv1a", Box::new(fnv1a)), ("fnv1", Box::new(fnv1)), ("oaat64", Box::new(oaat64)), ("murmur64a0", Box::new(|s| murmur64a(s, 0))),
        ("murmur64a_e6546b64", Box::new(|s| murmur64a(s, 0xe6546b64))), ("xxh64", Box::new(|s| xxh64(s, 0))),
        ("crc64ecma", Box::new(|s| crc64(s, 0x42F0E1EBA9EA3693, false, 0, 0))), ("crc64xz", Box::new(|s| crc64(s, 0xC96C5795D7870F42, true, !0, !0))),
        ("crc64iso", Box::new(|s| crc64(s, 0xD800000000000000, true, 0, 0))),
        ("joaat_pair", Box::new(|s| { let j = joaat(s) as u64; let mut t = s.to_vec(); t.reverse(); (j << 32) | joaat(&t) as u64 })),
    ];
    for (n, f) in &fs {
        for case in 0..3 {
            let (mut hf, mut hl, mut hh) = (0, 0, 0);
            for nm in &names {
                let s = match case { 0 => nm.clone(), 1 => nm.to_lowercase(), _ => nm.to_uppercase() };
                let h = f(s.as_bytes());
                if full.contains(&h) { hf += 1; }
                if lo.contains(&(h as u32)) { hl += 1; }
                if hi.contains(&((h >> 32) as u32)) { hh += 1; }
            }
            println!("{n} case{case}: full {hf} lo32 {hl} hi32 {hh}");
        }
    }
    // joaat 32 against halves
    let (mut l, mut h) = (0, 0);
    for nm in &names { let j = joaat(nm.to_lowercase().as_bytes()); if lo.contains(&j) { l += 1; } if hi.contains(&j) { h += 1; } }
    println!("joaat32 vs lo {l} hi {h}; names {}; chance per half ~{:.2}", names.len(), names.len() as f64 * lo.len() as f64 / 4294967296.0);
}
