/// LPN line-based parser.
///
/// Each non-blank, non-comment line in a `.lp` file is one instruction.
/// The first whitespace-separated token is the opcode.  The rest of the
/// tokens are opcode-specific.
///
/// ## Grammar summary
///
/// ```text
/// # Tier 1
/// FN   name :: sig
/// TYPE name = body
/// ENUM name = body
/// EMIT target Module [FROM file]
/// CHECK kind file
/// TEST  name (args) -> expected
/// VERIFY claim file
/// ADD feature TO module
/// DEL item FROM file
/// RENAME from TO to IN file
///
/// # Tier 2
/// IMPL target USING [M1,M2,M84-M89] EMIT target VERIFY step[+step]
/// REFACTOR file SPLIT AT fn_name
///
/// # Tier 3
/// ALX     key=value …
/// <NAME>  key=value …
/// ```
use crate::lpn::{
    ast::{parse_milestone_list, parse_verify_steps, EmitTarget, ExperimentParams},
    error::LpnError,
    LpnInstruction,
};

// ── Public API ────────────────────────────────────────────────────────────────

/// Stateless line-based parser for LPN source text.
pub struct LpnParser;

impl LpnParser {
    /// Parse a single line.  Returns `None` for blank lines and comments.
    /// Panics if the line contains an unrecognised opcode; use
    /// [`try_parse_line`] for fallible parsing.
    ///
    /// # Panics
    /// Panics on unrecognised opcodes — use [`try_parse_line`] in
    /// production paths.
    pub fn parse_line(line: &str) -> Option<LpnInstruction> {
        Self::try_parse_line(line).expect("invalid LPN instruction")
    }

    /// Parse a single line, returning `None` for blank/comment lines and
    /// `Err(LpnError::Parse)` for unrecognised or malformed instructions.
    pub fn try_parse_line(line: &str) -> Result<Option<LpnInstruction>, LpnError> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return Ok(None);
        }
        parse_instruction(trimmed, 0).map(Some)
    }

    /// Parse an entire `.lp` source string, returning all successfully parsed
    /// instructions.  Blank lines and comments are silently skipped.
    ///
    /// # Errors
    /// Returns the first parse error encountered.  Use
    /// [`parse_str_lenient`] to collect all instructions and skip errors.
    pub fn parse_str(src: &str) -> Vec<LpnInstruction> {
        src.lines()
            .filter_map(|line| Self::try_parse_line(line).ok().flatten())
            .collect()
    }

    /// Parse an entire `.lp` source string, collecting all successfully parsed
    /// instructions and all errors separately.
    pub fn parse_str_lenient(src: &str) -> (Vec<LpnInstruction>, Vec<LpnError>) {
        let mut ok = Vec::new();
        let mut errs = Vec::new();
        for (i, line) in src.lines().enumerate() {
            match Self::try_parse_line(line) {
                Ok(Some(instr)) => ok.push(instr),
                Ok(None) => {}
                Err(e) => errs.push(LpnError::Parse {
                    line: i + 1,
                    message: e.to_string(),
                }),
            }
        }
        (ok, errs)
    }
}

// ── Internal parsing ──────────────────────────────────────────────────────────

fn parse_instruction(line: &str, _line_no: usize) -> Result<LpnInstruction, LpnError> {
    let mut tokens = line.splitn(2, char::is_whitespace);
    let opcode = tokens.next().unwrap_or("").to_ascii_uppercase();
    let rest = tokens.next().unwrap_or("").trim();

    match opcode.as_str() {
        "FN" => parse_fn(rest),
        "TYPE" => parse_type(rest),
        "ENUM" => parse_enum(rest),
        "EMIT" => parse_emit(rest),
        "CHECK" => parse_check(rest),
        "TEST" => parse_test(rest),
        "VERIFY" => parse_verify(rest),
        "ADD" => parse_add(rest),
        "DEL" => parse_del(rest),
        "RENAME" => parse_rename(rest),
        "IMPL" => parse_impl(rest),
        "REFACTOR" => parse_refactor(rest),
        "ALX" => parse_alx(rest),
        other => {
            // Tier 3: any UPPER_CASE name followed by key=value params is a
            // named experiment.  A bare unknown opcode with no `=` in the rest
            // is an error (e.g. "FOOBAR something").
            if other
                .chars()
                .next()
                .map_or(false, |c| c.is_ascii_uppercase())
                && rest.contains('=')
            {
                Ok(parse_experiment(other, rest))
            } else {
                Err(LpnError::Parse {
                    line: 0,
                    message: format!("unknown opcode `{other}`"),
                })
            }
        }
    }
}

fn parse_fn(rest: &str) -> Result<LpnInstruction, LpnError> {
    let (name, sig) = rest
        .split_once("::")
        .map(|(n, s)| (n.trim().to_owned(), s.trim().to_owned()))
        .ok_or_else(|| err("FN requires `name :: TypeSig`"))?;
    Ok(LpnInstruction::Fn { name, sig })
}

fn parse_type(rest: &str) -> Result<LpnInstruction, LpnError> {
    let (name, body) = rest
        .split_once('=')
        .map(|(n, b)| (n.trim().to_owned(), b.trim().to_owned()))
        .ok_or_else(|| err("TYPE requires `name = body`"))?;
    Ok(LpnInstruction::Type { name, body })
}

fn parse_enum(rest: &str) -> Result<LpnInstruction, LpnError> {
    let (name, body) = rest
        .split_once('=')
        .map(|(n, b)| (n.trim().to_owned(), b.trim().to_owned()))
        .ok_or_else(|| err("ENUM requires `name = body`"))?;
    Ok(LpnInstruction::Enum { name, body })
}

fn parse_emit(rest: &str) -> Result<LpnInstruction, LpnError> {
    let parts: Vec<&str> = rest.splitn(4, char::is_whitespace).collect();
    let target_str = parts.first().copied().unwrap_or("");
    let target = EmitTarget::from_str(&target_str.to_lowercase())
        .ok_or_else(|| err(format!("unknown emit target `{target_str}`")))?;
    let module = parts.get(1).copied().unwrap_or("").to_owned();
    if module.is_empty() {
        return Err(err("EMIT requires a module name"));
    }
    let from = if parts
        .get(2)
        .map_or(false, |t| t.eq_ignore_ascii_case("FROM"))
    {
        parts.get(3).map(|s| s.to_string())
    } else {
        None
    };
    Ok(LpnInstruction::Emit {
        target,
        module,
        from,
    })
}

fn parse_check(rest: &str) -> Result<LpnInstruction, LpnError> {
    let mut parts = rest.splitn(2, char::is_whitespace);
    let kind_str = parts.next().unwrap_or("");
    let file = parts.next().unwrap_or("").trim().to_owned();
    let kind = CheckKind::from_str(&kind_str.to_lowercase())
        .ok_or_else(|| err(format!("unknown check kind `{kind_str}`")))?;
    if file.is_empty() {
        return Err(err("CHECK requires a file path"));
    }
    Ok(LpnInstruction::Check { kind, file })
}

fn parse_test(rest: &str) -> Result<LpnInstruction, LpnError> {
    // TEST name (args) -> expected
    let (name_and_args, expected) = rest
        .split_once("->")
        .map(|(l, r)| (l.trim(), r.trim().to_owned()))
        .ok_or_else(|| err("TEST requires `name (args) -> expected`"))?;
    let (name, args) = name_and_args
        .split_once('(')
        .map(|(n, a)| {
            let args = a.trim_end_matches(')').trim().to_owned();
            (n.trim().to_owned(), args)
        })
        .ok_or_else(|| err("TEST requires `name (args) -> expected`"))?;
    Ok(LpnInstruction::Test {
        name,
        args,
        expected,
    })
}

fn parse_verify(rest: &str) -> Result<LpnInstruction, LpnError> {
    let mut parts = rest.splitn(2, char::is_whitespace);
    let claim = parts.next().unwrap_or("").trim().to_owned();
    let file = parts.next().unwrap_or("").trim().to_owned();
    if claim.is_empty() || file.is_empty() {
        return Err(err("VERIFY requires `claim file`"));
    }
    Ok(LpnInstruction::Verify { claim, file })
}

fn parse_add(rest: &str) -> Result<LpnInstruction, LpnError> {
    // ADD feature TO module
    let parts: Vec<&str> = rest.splitn(3, char::is_whitespace).collect();
    let feature = parts.first().copied().unwrap_or("").to_owned();
    let module = parts.get(2).copied().unwrap_or("").to_owned();
    Ok(LpnInstruction::Add { feature, module })
}

fn parse_del(rest: &str) -> Result<LpnInstruction, LpnError> {
    // DEL item FROM file
    let parts: Vec<&str> = rest.splitn(3, char::is_whitespace).collect();
    let item = parts.first().copied().unwrap_or("").to_owned();
    let from = parts.get(2).copied().unwrap_or("").to_owned();
    Ok(LpnInstruction::Del { item, from })
}

fn parse_rename(rest: &str) -> Result<LpnInstruction, LpnError> {
    // RENAME from TO to IN file
    let parts: Vec<&str> = rest.split_whitespace().collect();
    // parts: [from, TO, to, IN, file]
    let from = parts.first().copied().unwrap_or("").to_owned();
    let to = parts.get(2).copied().unwrap_or("").to_owned();
    let in_file = parts.get(4).copied().unwrap_or("").to_owned();
    Ok(LpnInstruction::Rename { from, to, in_file })
}

fn parse_impl(rest: &str) -> Result<LpnInstruction, LpnError> {
    // IMPL target USING [M1,M2,M84-M89] EMIT target VERIFY step[+step]
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let target = tokens.first().copied().unwrap_or("").to_owned();

    let milestones = tokens
        .iter()
        .find(|t| t.starts_with('['))
        .map(|s| parse_milestone_list(s))
        .unwrap_or_default();

    let emit = tokens
        .windows(2)
        .find(|w| w[0].eq_ignore_ascii_case("EMIT"))
        .and_then(|w| EmitTarget::from_str(&w[1].to_lowercase()))
        .unwrap_or(EmitTarget::Rust);

    let verify = tokens
        .windows(2)
        .find(|w| w[0].eq_ignore_ascii_case("VERIFY"))
        .map(|w| parse_verify_steps(w[1]))
        .unwrap_or_default();

    Ok(LpnInstruction::Impl {
        target,
        milestones,
        emit,
        verify,
    })
}

fn parse_refactor(rest: &str) -> Result<LpnInstruction, LpnError> {
    // REFACTOR file SPLIT AT fn_name
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let file = tokens.first().copied().unwrap_or("").to_owned();
    let split_at = tokens.last().copied().unwrap_or("").to_owned();
    Ok(LpnInstruction::Refactor { file, split_at })
}

fn parse_alx(rest: &str) -> Result<LpnInstruction, LpnError> {
    let mut params = ExperimentParams::default();
    for kv in rest.split_whitespace() {
        if let Some((k, v)) = parse_kv(kv) {
            match k {
                "n" => {
                    params.n = v.parse().ok();
                }
                "domain" => params.domain = Some(v.to_owned()),
                "coverage" => params.min_coverage = v.trim_start_matches(">=").parse().ok(),
                "emit" => {
                    params.emit = EmitTarget::from_str(v).unwrap_or(EmitTarget::Rust);
                }
                "verify" => params.verify = parse_verify_steps(v),
                "evidence" => params.evidence = v == "store",
                _ => {}
            }
        }
    }
    Ok(LpnInstruction::Alx(params))
}

fn parse_experiment(name: &str, rest: &str) -> LpnInstruction {
    let params: Vec<(String, String)> = rest
        .split_whitespace()
        .filter_map(|kv| parse_kv(kv).map(|(k, v)| (k.to_owned(), v.to_owned())))
        .collect();
    LpnInstruction::Experiment {
        name: name.to_owned(),
        params,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_kv(s: &str) -> Option<(&str, &str)> {
    // Try `>=` before `=` so "coverage>=0.95" splits as ("coverage","0.95")
    // not ("coverage>","0.95").
    s.split_once(">=").or_else(|| s.split_once('='))
}

fn err(msg: impl Into<String>) -> LpnError {
    LpnError::Parse {
        line: 0,
        message: msg.into(),
    }
}

// Bring CheckKind into scope for parse_check
use crate::lpn::ast::CheckKind;
