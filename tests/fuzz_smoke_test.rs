//! M3 deterministic stress harness (see docs/maturity-experiments.md).
//!
//! Runs seeded hostile inputs through lexer, parser and the full compile pipeline,
//! each call under `catch_unwind` and a 1 s watchdog. Default run: ~2000 inputs.
//!
//! Env vars:
//!   LOOM_FUZZ_N      number of inputs (default 2000)
//!   LOOM_FUZZ_SEED   PRNG seed (default 0x4C4F4F4D00000003)
//!   LOOM_FUZZ_START  first input index (to resume after an abort)
//!   LOOM_FUZZ_DUMP   directory to write failing inputs to (default: none)
//!
//! Full run: `LOOM_FUZZ_N=1000000 cargo test --release --test fuzz_smoke_test -- --nocapture`
//! Input `i` is a pure function of (seed, i), so any failure is reproducible.
//! Each input is run through lex, lex+parse, and compile (3 calls per input).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

const DEFAULT_SEED: u64 = 0x4C4F_4F4D_0000_0003;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

fn load_corpus() -> Vec<String> {
    fn walk(dir: &std::path::Path, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().map_or(false, |e| e == "loom") {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    out.push(s);
                }
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut v = Vec::new();
    walk(&root.join("corpus"), &mut v);
    walk(&root.join("examples"), &mut v);
    v
}

/// Split into word / whitespace / punctuation chunks, preserving all text.
fn chunks(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut prev: Option<u8> = None; // 0 word, 1 space, 2 punct
    for (i, c) in s.char_indices() {
        let k = if c.is_alphanumeric() || c == '_' {
            0
        } else if c.is_whitespace() {
            1
        } else {
            2
        };
        if prev != Some(k) || k == 2 {
            if i > start {
                out.push(&s[start..i]);
            }
            start = i;
        }
        prev = Some(k);
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

const SNIPPETS: &[&str] = &[
    "module", "fn", "end", "being", "match", "->", "::", "{", "}", "(", ")", "[", "]", "|", "<", ">",
    "\"", "'", "//", "/*", "0x", "1e999999", "-", "..", "@", "#", "\\", "\0", "let", "in", "if",
    "then", "else", "require:", "ensure:", "forall", "type", "enum", "effect", "provenance",
    "refined", "where", "∀", "→", "\u{202e}",
];

fn gen_input(seed: u64, i: u64, corpus: &[String]) -> String {
    let mut r = Rng(seed ^ i.wrapping_mul(0xD6E8_FEB8_6659_FD93));
    r.next();
    let kind = r.below(10);
    match kind {
        0 => {
            let n = r.below(512);
            let b: Vec<u8> = (0..n).map(|_| r.next() as u8).collect();
            String::from_utf8_lossy(&b).into_owned()
        }
        1 => {
            let n = r.below(256);
            (0..n)
                .map(|_| match r.below(4) {
                    0 => char::from_u32(r.below(0x80) as u32).unwrap(),
                    1 => char::from_u32(0x80 + r.below(0x780) as u32).unwrap(),
                    2 => char::from_u32(0x800 + r.below(0xD000) as u32).unwrap_or('?'),
                    _ => char::from_u32(0x10000 + r.below(0x10000) as u32).unwrap_or('?'),
                })
                .collect()
        }
        2 | 3 | 4 => {
            let src = corpus[r.below(corpus.len())].clone();
            let mut ch: Vec<String> = chunks(&src).into_iter().map(String::from).collect();
            for _ in 0..1 + r.below(4) {
                if ch.is_empty() {
                    break;
                }
                let a = r.below(ch.len());
                match r.below(5) {
                    0 => {
                        ch.remove(a);
                    }
                    1 => {
                        let c = ch[a].clone();
                        ch.insert(a, c);
                    }
                    2 => {
                        let b = r.below(ch.len());
                        ch.swap(a, b);
                    }
                    3 => {
                        let s = SNIPPETS[r.below(SNIPPETS.len())].to_string();
                        ch.insert(a, s);
                    }
                    _ => {
                        let s = SNIPPETS[r.below(SNIPPETS.len())].to_string();
                        ch[a] = s;
                    }
                }
            }
            ch.concat()
        }
        5 | 6 => {
            let src = &corpus[r.below(corpus.len())];
            let mut cut = r.below(src.len() + 1);
            while !src.is_char_boundary(cut) {
                cut -= 1;
            }
            src[..cut].to_string()
        }
        7 => {
            // deep nesting, up to 10000
            let depth = 1 + r.below(10_000);
            let pairs = [
                ("(", ")"),
                ("{", "}"),
                ("[", "]"),
                ("<", ">"),
                ("fn f() -> ", ""),
                ("match x { _ => ", "}"),
                ("-", ""),
                ("(", ""),
            ];
            let (o, c) = pairs[r.below(pairs.len())];
            let prefix = if r.below(2) == 0 { "module M\n  fn f() -> Int\n    " } else { "" };
            let mut s = String::from(prefix);
            for _ in 0..depth {
                s.push_str(o);
            }
            s.push('1');
            if r.below(3) != 0 {
                for _ in 0..depth {
                    s.push_str(c);
                }
            }
            s
        }
        8 => {
            // huge atoms
            let n = 1 + r.below(200_000);
            let atom = match r.below(5) {
                0 => "a".repeat(n),
                1 => format!("\"{}\"", "x".repeat(n)),
                2 => "9".repeat(n),
                3 => format!(
                    "{}.{}e{}",
                    "9".repeat(n.min(5000)),
                    "9".repeat(n.min(5000)),
                    "9".repeat(n.min(400))
                ),
                _ => format!("\"{}", "\\".repeat(n)),
            };
            let pre = ["", "module M\n  fn f() -> Int\n    ", "module M\n  type T = ", "module "]
                [r.below(4)];
            format!("{pre}{atom}")
        }
        _ => {
            let a = &corpus[r.below(corpus.len())];
            let b = &corpus[r.below(corpus.len())];
            let mut ca = (a.len() / 2 + r.below(a.len() / 2 + 1)).min(a.len());
            while !a.is_char_boundary(ca) {
                ca -= 1;
            }
            let mut cb = r.below(b.len() + 1);
            while !b.is_char_boundary(cb) {
                cb -= 1;
            }
            format!("{}{}", &a[..ca], &b[cb..])
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Stage {
    Lex,
    Parse,
    Compile,
}
const STAGES: [Stage; 3] = [Stage::Lex, Stage::Parse, Stage::Compile];

fn run_stage(stage: Stage, s: &str) {
    match stage {
        Stage::Lex => {
            let _ = loom::lexer::Lexer::tokenize(s);
        }
        Stage::Parse => {
            if let Ok(t) = loom::lexer::Lexer::tokenize(s) {
                let _ = loom::parser::Parser::new(&t).parse_module();
            }
        }
        Stage::Compile => {
            let _ = loom::compile(s);
        }
    }
}

#[derive(Debug)]
struct Failure {
    index: u64,
    stage: Stage,
    kind: &'static str,
    detail: String,
}

/// Returns (failures, calls). After a hang the stuck worker is abandoned and a new one resumes.
fn run_fuzz(seed: u64, start: u64, n: u64, dump: Option<String>) -> (Vec<Failure>, u64) {
    let corpus = Arc::new(load_corpus());
    assert!(!corpus.is_empty(), "no corpus found");
    std::panic::set_hook(Box::new(|_| {})); // panics are collected via catch_unwind
    let mut failures = Vec::new();
    let mut calls = 0u64;
    let mut next = start;
    let end = start + n;
    let t0 = Instant::now();
    let mut last_report = Instant::now();
    while next < end {
        let cur_idx = Arc::new(AtomicU64::new(u64::MAX));
        let cur_stage = Arc::new(AtomicU64::new(0));
        let cur_start = Arc::new(AtomicU64::new(0)); // ms since t0 + 1; 0 = idle
        let (tx, rx) = mpsc::channel::<Failure>();
        let (done_tx, done_rx) = mpsc::channel::<u64>();
        let (c2, i2, s2, st2, d) =
            (corpus.clone(), cur_idx.clone(), cur_stage.clone(), cur_start.clone(), dump.clone());
        let from = next;
        let trace = std::env::var("LOOM_FUZZ_TRACE").is_ok();
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                for i in from..end {
                    let input = gen_input(seed, i, &c2);
                    if trace {
                        eprintln!("[trace] {i} len={}", input.len());
                        if let Some(dir) = &d {
                            let _ = std::fs::create_dir_all(dir);
                            let _ = std::fs::write(format!("{dir}/last-input.loom"), &input);
                        }
                    }
                    for (k, stage) in STAGES.into_iter().enumerate() {
                        i2.store(i, Ordering::SeqCst);
                        s2.store(k as u64, Ordering::SeqCst);
                        st2.store(t0.elapsed().as_millis() as u64 + 1, Ordering::SeqCst);
                        let r = std::panic::catch_unwind(|| run_stage(stage, &input));
                        st2.store(0, Ordering::SeqCst);
                        if let Err(p) = r {
                            let msg = p
                                .downcast_ref::<String>()
                                .cloned()
                                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                                .unwrap_or_else(|| "<non-string panic>".into());
                            if let Some(dir) = &d {
                                let _ = std::fs::create_dir_all(dir);
                                let _ = std::fs::write(
                                    format!("{dir}/panic-{i}-{stage:?}.loom"),
                                    &input,
                                );
                            }
                            let _ = tx.send(Failure { index: i, stage, kind: "panic", detail: msg });
                        }
                    }
                    let _ = done_tx.send(i);
                }
            })
            .unwrap();
        let mut last_seen = from;
        loop {
            match done_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(i) => {
                    last_seen = i + 1;
                    calls += 3;
                    if last_seen >= end {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let st = cur_start.load(Ordering::SeqCst);
                    if st != 0 {
                        let now = t0.elapsed().as_millis() as u64 + 1;
                        if now.saturating_sub(st) > 1000 {
                            let idx = cur_idx.load(Ordering::SeqCst);
                            let stage = STAGES[cur_stage.load(Ordering::SeqCst) as usize];
                            if let Some(dir) = &dump {
                                let _ = std::fs::create_dir_all(dir);
                                let _ = std::fs::write(
                                    format!("{dir}/hang-{idx}-{stage:?}.loom"),
                                    gen_input(seed, idx, &corpus),
                                );
                            }
                            failures.push(Failure { index: idx, stage, kind: "hang", detail: ">1s".into() });
                            last_seen = idx + 1;
                            calls += 3;
                            break; // abandon stuck worker, resume after idx
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            if last_report.elapsed() > Duration::from_secs(30) {
                last_report = Instant::now();
                eprintln!("[fuzz] ~{last_seen}/{end} elapsed {:?} failures {}", t0.elapsed(), failures.len());
            }
        }
        while let Ok(f) = rx.try_recv() {
            failures.push(f);
        }
        next = last_seen;
    }
    (failures, calls)
}

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k)
        .ok()
        .and_then(|v| {
            let v = v.trim();
            if let Some(h) = v.strip_prefix("0x") {
                u64::from_str_radix(h, 16).ok()
            } else {
                v.parse().ok()
            }
        })
        .unwrap_or(d)
}

#[test]
fn fuzz_smoke() {
    let n = env_u64("LOOM_FUZZ_N", 2000);
    let seed = env_u64("LOOM_FUZZ_SEED", DEFAULT_SEED);
    let start = env_u64("LOOM_FUZZ_START", 0);
    let dump = std::env::var("LOOM_FUZZ_DUMP").ok();
    let t = Instant::now();
    let (failures, calls) = run_fuzz(seed, start, n, dump);
    eprintln!(
        "[fuzz] inputs={n} calls={calls} seed={seed:#x} wall={:?} failures={}",
        t.elapsed(),
        failures.len()
    );
    let mut distinct = std::collections::BTreeMap::<String, (u64, usize)>::new();
    for f in &failures {
        let e = distinct
            .entry(format!("{} {:?}: {}", f.kind, f.stage, f.detail))
            .or_insert((f.index, 0));
        e.1 += 1;
    }
    for (k, (first, c)) in &distinct {
        eprintln!("[fuzz] DISTINCT x{c} first@{first}: {k}");
    }
    assert!(failures.is_empty(), "{} failures, {} distinct", failures.len(), distinct.len());
}

// ---------------------------------------------------------------------------
// Minimiser: `LOOM_FUZZ_MINIMIZE=<file> cargo test --release --test fuzz_smoke_test minimize -- --ignored --nocapture`
// Greedy delta debugging (lines, then chunks) while the failure (panic/hang) persists.
// ---------------------------------------------------------------------------

fn fails(input: &str) -> Option<String> {
    let input = input.to_string();
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            for stage in STAGES {
                if let Err(p) = std::panic::catch_unwind(|| run_stage(stage, &input)) {
                    let m = p
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default();
                    let _ = tx.send(format!("panic {stage:?}: {m}"));
                    return;
                }
            }
            let _ = tx.send(String::new());
        })
        .unwrap();
    match rx.recv_timeout(Duration::from_millis(1000)) {
        Ok(s) if s.is_empty() => None,
        Ok(s) => Some(s),
        Err(_) => Some("hang".into()),
    }
}

// ---------------------------------------------------------------------------
// Regression tests for bugs found by the M3 fuzz run (one per fix).
// ---------------------------------------------------------------------------

/// Bug M3-1: an unterminated `produces: [` list in a `morphogen:` section looped forever
/// at end of input (parse_being_morphogen_section never advanced). Found at input #45933.
#[test]
fn regression_m3_1_unterminated_morphogen_produces_list_terminates() {
    for tail in ["produces:  [Quo", "produces: [", "produces: [A,", "produces: [1 2 3"] {
        let src = format!("module M\nbeing B\nmorphogen:\n  signal: S\n  threshold: 0.5\n  {tail}");
        assert!(fails(&src).is_none(), "hang or panic on tail {tail:?}");
        assert!(loom::compile(&src).is_err());
    }
}

fn nest_inputs(depth: usize) -> Vec<(&'static str, String)> {
    let pairs = [
        ("(", ")"),
        ("{", "}"),
        ("[", "]"),
        ("<", ">"),
        ("fn f() -> ", ""),
        ("match x { _ => ", "}"),
        ("-", ""),
        ("(", ""),
    ];
    let mut v = Vec::new();
    for (o, c) in pairs {
        for prefix in ["", "module M\n  fn f() -> Int\n    "] {
            let mut s = String::from(prefix);
            s.push_str(&o.repeat(depth));
            s.push('1');
            s.push_str(&c.repeat(depth));
            v.push((o, s));
        }
    }
    v
}

/// Bug M3-2: a 189k-digit literal overflowed the stack inside the logos lexer in
/// unoptimised builds (abort, not catchable). Found by the debug-profile run at input #333.
/// Over-long identifier/number runs are now a `LexError`; every hostile atom shape must
/// survive on a 2 MiB stack and the over-long ones must be rejected, not accepted.
#[test]
fn regression_m3_2_huge_tokens_do_not_overflow_stack() {
    assert!(loom::lexer::Lexer::tokenize(&"9".repeat(189_000)).is_err());
    assert!(loom::lexer::Lexer::tokenize(&"a".repeat(loom::lexer::MAX_TOKEN_RUN + 1)).is_err());
    assert!(loom::lexer::Lexer::tokenize(&"a".repeat(loom::lexer::MAX_TOKEN_RUN)).is_ok());
    probe_digits();
}

fn probe_digits() {
    let n = 200_000;
    let atoms: Vec<(&str, String)> = vec![
        ("ident", "a".repeat(n)),
        ("digits", "9".repeat(n)),
        ("float", format!("{}.{}", "9".repeat(n), "9".repeat(n))),
        ("string", format!("\"{}\"", "x".repeat(n))),
        ("unterminated-string", format!("\"{}", "x".repeat(n))),
        ("backslashes", format!("\"{}", "\\".repeat(n))),
        ("line-comment", format!("-- {}", "x".repeat(n))),
        ("block-comment", format!("/* {}", "x".repeat(n))),
        ("inline", format!("inline {{ {}", "x".repeat(n))),
        ("inline-braces", format!("inline {{ {}", "{".repeat(n))),
        ("spaces", " ".repeat(n)),
        ("newlines", "\n".repeat(n)),
        ("punct", "-".repeat(n)),
    ];
    for (name, s) in atoms {
    for stage in STAGES {
        eprintln!("[probe] {name} stage={stage:?}");
        let s2 = s.clone();
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || run_stage(stage, &s2))
            .unwrap()
            .join()
            .unwrap();
    }
    }
}

#[test]
#[ignore]
fn probe_nesting() {
    let depth = env_u64("LOOM_PROBE_DEPTH", 10_000) as usize;
    let stack = env_u64("LOOM_PROBE_STACK", 2 * 1024 * 1024) as usize;
    for (o, s) in nest_inputs(depth) {
        for stage in STAGES {
            eprintln!("[probe] open={o:?} len={} stage={stage:?}", s.len());
            let s2 = s.clone();
            std::thread::Builder::new()
                .stack_size(stack)
                .spawn(move || {
                    let _ = std::panic::catch_unwind(|| run_stage(stage, &s2));
                })
                .unwrap()
                .join()
                .unwrap();
        }
    }
}

/// `LOOM_FUZZ_REPLAY=<dir>`: re-run every dumped input in a directory and list those still failing.
#[test]
#[ignore]
fn replay() {
    let Ok(dir) = std::env::var("LOOM_FUZZ_REPLAY") else { return };
    std::panic::set_hook(Box::new(|_| {}));
    let mut still = 0;
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let s = std::fs::read_to_string(e.path()).unwrap();
        if let Some(m) = fails(&s) {
            still += 1;
            eprintln!("[replay] STILL FAILS {} ({m})", e.path().display());
        }
    }
    eprintln!("[replay] still failing: {still}");
}

#[test]
#[ignore]
fn minimize() {
    let Ok(path) = std::env::var("LOOM_FUZZ_MINIMIZE") else { return };
    std::panic::set_hook(Box::new(|_| {}));
    let src = std::fs::read_to_string(&path).unwrap();
    let Some(orig) = fails(&src) else {
        eprintln!("input does not fail");
        return;
    };
    let kind = orig.split(':').next().unwrap().to_string();
    let same = |s: &str| fails(s).map_or(false, |m| m.starts_with(&kind));
    let mut cur = src;
    for granularity in [0usize, 1] {
        let mut changed = true;
        while changed {
            changed = false;
            let parts: Vec<String> = if granularity == 0 {
                cur.split_inclusive('\n').map(String::from).collect()
            } else {
                chunks(&cur).into_iter().map(String::from).collect()
            };
            let mut keep = parts;
            let mut sz = (keep.len() / 2).max(1);
            loop {
                let mut i = 0;
                while i < keep.len() {
                    let end = (i + sz).min(keep.len());
                    let mut t = keep.clone();
                    t.drain(i..end);
                    if same(&t.concat()) {
                        keep = t;
                        changed = true;
                    } else {
                        i += sz;
                    }
                }
                if sz == 1 {
                    break;
                }
                sz /= 2;
            }
            cur = keep.concat();
        }
    }
    eprintln!("[min] {orig}\n[min] ---\n{cur}\n[min] ---");
}
