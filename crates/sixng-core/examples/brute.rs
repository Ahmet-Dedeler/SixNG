//! Research: brute-force case-sensitive joaat names from a vocabulary.
//! brute <targets.txt (hex per line)> <vocab.txt> <max words> [prefixes,comma] [suffixes,comma]
//! Styles: CamelCase, lowerCamel, UPPER_SNAKE, lower_snake, raw concat.
use std::collections::HashSet;
fn joaat(s: &[u8]) -> u32 {
    let mut h: u32 = 0;
    for &c in s {
        h = h.wrapping_add(c as u32);
        h = h.wrapping_add(h << 10);
        h ^= h >> 6;
    }
    h = h.wrapping_add(h << 3);
    h ^= h >> 11;
    h.wrapping_add(h << 15)
}
fn cap(w: &str) -> String {
    let mut c = w.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let targets: HashSet<u32> = std::fs::read_to_string(&a[1]).unwrap().lines().filter_map(|l| u32::from_str_radix(l.trim(), 16).ok()).collect();
    let vocab: Vec<String> = std::fs::read_to_string(&a[2]).unwrap().split_whitespace().map(|s| s.to_string()).collect();
    let maxw: usize = a[3].parse().unwrap();
    let pre: Vec<String> = a.get(4).map(|s| s.split(',').map(|x| x.to_string()).collect()).unwrap_or(vec![String::new()]);
    let suf: Vec<String> = a.get(5).map(|s| s.split(',').map(|x| x.to_string()).collect()).unwrap_or(vec![String::new()]);
    let mut found = std::collections::BTreeMap::new();
    let n = vocab.len();
    let mut idx = vec![0usize; maxw];
    for k in 1..=maxw {
        let total = n.pow(k as u32);
        for c in 0..total {
            let mut x = c;
            for i in 0..k {
                idx[i] = x % n;
                x /= n;
            }
            let ws: Vec<&str> = idx[..k].iter().map(|&i| vocab[i].as_str()).collect();
            let camel: String = ws.iter().map(|w| cap(w)).collect();
            let lcamel: String = ws.iter().enumerate().map(|(i, w)| if i == 0 { w.to_lowercase() } else { cap(w) }).collect();
            let up = ws.iter().map(|w| w.to_uppercase()).collect::<Vec<_>>().join("_");
            let low = ws.iter().map(|w| w.to_lowercase()).collect::<Vec<_>>().join("_");
            let raw: String = ws.concat();
            for s in [camel, lcamel, up, low, raw] {
                for p in &pre {
                    for q in &suf {
                        let t = format!("{p}{s}{q}");
                        let h = joaat(t.as_bytes());
                        if targets.contains(&h) {
                            found.entry(h).or_insert_with(Vec::new).push(t);
                        }
                    }
                }
            }
        }
    }
    for (h, v) in found {
        let mut v = v;
        v.sort();
        v.dedup();
        println!("{h:08x} {}", v.join(" | "));
    }
}
