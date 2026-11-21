//! Research: lowercase joaat brute force with prefix-state reuse.
//! jbrute <targets.txt> <vocab.txt> <max words> [seps, e.g. "_,"] [prefixes] [suffixes]
//! Targets: first hex token per line. Prints "hash name".
use std::collections::HashSet;
fn step(mut h: u32, s: &[u8]) -> u32 {
    for &c in s {
        h = h.wrapping_add(c.to_ascii_lowercase() as u32);
        h = h.wrapping_add(h << 10);
        h ^= h >> 6;
    }
    h
}
fn fin(mut h: u32) -> u32 {
    h = h.wrapping_add(h << 3);
    h ^= h >> 11;
    h.wrapping_add(h << 15)
}
struct C<'a> { t: &'a HashSet<u32>, v: &'a [String], seps: &'a [String], suf: &'a [String], maxw: usize }
fn rec(c: &C, h: u32, depth: usize, name: &mut String) {
    for w in c.v {
        for sp in c.seps {
            let h1 = step(step(h, sp.as_bytes()), w.as_bytes());
            let l = name.len();
            name.push_str(sp);
            name.push_str(w);
            for q in c.suf {
                let hf = fin(step(h1, q.as_bytes()));
                if c.t.contains(&hf) {
                    println!("{hf:08x} {name}{q}");
                }
            }
            if depth + 1 < c.maxw {
                rec(c, h1, depth + 1, name);
            }
            name.truncate(l);
        }
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let t: HashSet<u32> = std::fs::read_to_string(&a[1]).unwrap().lines().filter_map(|l| l.split_whitespace().next().and_then(|x| u32::from_str_radix(x, 16).ok())).collect();
    let mut v: Vec<String> = std::fs::read_to_string(&a[2]).unwrap().split_whitespace().map(|s| s.to_lowercase()).collect();
    v.sort();
    v.dedup();
    let maxw: usize = a[3].parse().unwrap();
    let sp = |i: usize| -> Vec<String> { a.get(i).map(|s| s.split(',').map(|x| x.to_string()).collect()).unwrap_or(vec![String::new()]) };
    let seps = sp(4);
    let pre = sp(5);
    let suf = sp(6);
    eprintln!("{} targets, {} words", t.len(), v.len());
    use rayon::prelude::*;
    let c = C { t: &t, v: &v, seps: &seps, suf: &suf, maxw };
    for p in &pre {
        v.par_iter().for_each(|w| {
            let h = step(step(0, p.as_bytes()), w.as_bytes());
            let mut name = format!("{p}{w}");
            for q in c.suf {
                let hf = fin(step(h, q.as_bytes()));
                if c.t.contains(&hf) {
                    println!("{hf:08x} {name}{q}");
                }
            }
            if maxw > 1 {
                rec(&c, h, 1, &mut name);
            }
        });
    }
}
