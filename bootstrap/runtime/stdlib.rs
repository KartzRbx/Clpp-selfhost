//! Standard modules shared by the interpreter (`clpp-boot`) and the VM
//! (`clpp-vm`): `@clpp.math`, `@clpp.text`, `@clpp.table`, `@clpp.fs`, and the
//! sized numeric types (`i8` … `u64`, `f16`, `f32`, `f64`).
//!
//! Each runtime converts its values to `SVal`, calls `call`, and converts the
//! result back. Objects and enums travel as `Other`, so their identity is kept.

use std::cell::Cell;

#[derive(Clone, Debug)]
pub enum SVal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<SVal>),
    Map(Vec<(SVal, SVal)>),
    /// A runtime value this module does not look into: (id, display text).
    Other(usize, String),
}

impl SVal {
    pub fn type_name(&self) -> &'static str {
        match self {
            SVal::Null => "null",
            SVal::Bool(_) => "bool",
            SVal::Int(_) => "number",
            SVal::Float(_) => "float",
            SVal::Str(_) => "string",
            SVal::List(_) => "array",
            SVal::Map(_) => "dictionary",
            SVal::Other(..) => "value",
        }
    }

    pub fn text(&self) -> String {
        match self {
            SVal::Null => "null".into(),
            SVal::Bool(b) => b.to_string(),
            SVal::Int(i) => i.to_string(),
            SVal::Float(f) => {
                if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e16 {
                    format!("{:.1}", f)
                } else {
                    format!("{}", f)
                }
            }
            SVal::Str(s) => s.clone(),
            SVal::List(items) => format!("{{{}}}", items.iter().map(SVal::text).collect::<Vec<_>>().join(", ")),
            SVal::Map(entries) => format!(
                "{{{}}}",
                entries.iter().map(|(k, v)| format!("[{}] = {}", k.text(), v.text())).collect::<Vec<_>>().join(", ")
            ),
            SVal::Other(_, t) => t.clone(),
        }
    }
}

pub fn same(a: &SVal, b: &SVal) -> bool {
    match (a, b) {
        (SVal::Null, SVal::Null) => true,
        (SVal::Bool(x), SVal::Bool(y)) => x == y,
        (SVal::Int(x), SVal::Int(y)) => x == y,
        (SVal::Float(x), SVal::Float(y)) => x == y,
        (SVal::Int(x), SVal::Float(y)) | (SVal::Float(y), SVal::Int(x)) => *x as f64 == *y,
        (SVal::Str(x), SVal::Str(y)) => x == y,
        (SVal::Other(x, _), SVal::Other(y, _)) => x == y,
        _ => false,
    }
}

// ==================================================================== catalog

/// (module, member, signature, description). Members with no parentheses in
/// the signature are constants.
pub const CATALOG: &[(&str, &str, &str, &str)] = &[
    ("math", "pi", "float Math.pi", "π ≈ 3.14159."),
    ("math", "tau", "float Math.tau", "2π ≈ 6.28318."),
    ("math", "e", "float Math.e", "Euler's number ≈ 2.71828."),
    ("math", "huge", "float Math.huge", "Positive infinity: larger than any number."),
    ("math", "nan", "float Math.nan", "Not a number."),
    ("math", "epsilon", "float Math.epsilon", "Smallest difference between 1.0 and the next float."),
    ("math", "maxInt", "number Math.maxInt", "Largest number: 9223372036854775807."),
    ("math", "minInt", "number Math.minInt", "Smallest number: -9223372036854775808."),
    ("math", "abs", "number Math.abs(number x)", "Absolute value (keeps number or float)."),
    ("math", "sign", "number Math.sign(number x)", "-1, 0 or 1."),
    ("math", "floor", "number Math.floor(float x)", "Largest integer ≤ x."),
    ("math", "ceil", "number Math.ceil(float x)", "Smallest integer ≥ x."),
    ("math", "round", "number Math.round(float x)", "Nearest integer; halves round away from zero."),
    ("math", "trunc", "number Math.trunc(float x)", "Integer part, toward zero."),
    ("math", "fract", "float Math.fract(float x)", "x minus its integer part."),
    ("math", "sqrt", "float Math.sqrt(float x)", "Square root."),
    ("math", "cbrt", "float Math.cbrt(float x)", "Cube root."),
    ("math", "pow", "float Math.pow(float base, float exponent)", "base raised to exponent; exact for number base and non-negative number exponent."),
    ("math", "exp", "float Math.exp(float x)", "e raised to x."),
    ("math", "log", "float Math.log(float x)", "Natural logarithm."),
    ("math", "log2", "float Math.log2(float x)", "Base-2 logarithm."),
    ("math", "log10", "float Math.log10(float x)", "Base-10 logarithm."),
    ("math", "sin", "float Math.sin(float radians)", "Sine."),
    ("math", "cos", "float Math.cos(float radians)", "Cosine."),
    ("math", "tan", "float Math.tan(float radians)", "Tangent."),
    ("math", "asin", "float Math.asin(float x)", "Arc sine, in radians."),
    ("math", "acos", "float Math.acos(float x)", "Arc cosine, in radians."),
    ("math", "atan", "float Math.atan(float x)", "Arc tangent, in radians."),
    ("math", "atan2", "float Math.atan2(float y, float x)", "Angle of the point (x, y), in radians."),
    ("math", "sinh", "float Math.sinh(float x)", "Hyperbolic sine."),
    ("math", "cosh", "float Math.cosh(float x)", "Hyperbolic cosine."),
    ("math", "tanh", "float Math.tanh(float x)", "Hyperbolic tangent."),
    ("math", "hypot", "float Math.hypot(float x, float y)", "sqrt(x² + y²) without overflow."),
    ("math", "min", "number Math.min(number a, number b)", "Smaller of two numbers."),
    ("math", "max", "number Math.max(number a, number b)", "Larger of two numbers."),
    ("math", "clamp", "number Math.clamp(number x, number low, number high)", "x limited to [low, high]."),
    ("math", "lerp", "float Math.lerp(float a, float b, float t)", "a + (b - a) * t."),
    ("math", "deg", "float Math.deg(float radians)", "Radians to degrees."),
    ("math", "rad", "float Math.rad(float degrees)", "Degrees to radians."),
    ("math", "isNan", "bool Math.isNan(float x)", "True for NaN."),
    ("math", "isInf", "bool Math.isInf(float x)", "True for ±infinity."),
    ("math", "random", "float Math.random()", "Random float in [0, 1)."),
    ("math", "randomInt", "number Math.randomInt(number low, number high)", "Random integer in [low, high], both included."),
    ("math", "seed", "void Math.seed(number value)", "Makes random numbers repeatable."),
    ("text", "trim", "string Text.trim(string s)", "Removes spaces at both ends."),
    ("text", "trimStart", "string Text.trimStart(string s)", "Removes spaces at the start."),
    ("text", "trimEnd", "string Text.trimEnd(string s)", "Removes spaces at the end."),
    ("text", "lower", "string Text.lower(string s)", "Lowercase."),
    ("text", "upper", "string Text.upper(string s)", "Uppercase."),
    ("text", "contains", "bool Text.contains(string s, string part)", "True when part occurs in s."),
    ("text", "startsWith", "bool Text.startsWith(string s, string prefix)", "True when s begins with prefix."),
    ("text", "endsWith", "bool Text.endsWith(string s, string suffix)", "True when s ends with suffix."),
    ("text", "indexOf", "number Text.indexOf(string s, string part)", "Character index of the first match, or -1."),
    ("text", "lastIndexOf", "number Text.lastIndexOf(string s, string part)", "Character index of the last match, or -1."),
    ("text", "count", "number Text.count(string s, string part)", "Number of non-overlapping matches."),
    ("text", "replace", "string Text.replace(string s, string from, string to)", "Replaces every match."),
    ("text", "split", "array<string> Text.split(string s, string separator)", "Splits on a separator; \"\" splits into characters."),
    ("text", "join", "string Text.join(array<string> items, string separator)", "Joins values with a separator."),
    ("text", "repeat", "string Text.repeat(string s, number times)", "s repeated."),
    ("text", "reverse", "string Text.reverse(string s)", "Characters in reverse order."),
    ("text", "chars", "array<string> Text.chars(string s)", "One string per character."),
    ("text", "code", "number Text.code(string character)", "Unicode code point of the first character."),
    ("text", "fromCode", "string Text.fromCode(number code)", "Character for a Unicode code point."),
    ("text", "padLeft", "string Text.padLeft(string s, number width, string fill)", "Pads at the start up to width characters."),
    ("text", "padRight", "string Text.padRight(string s, number width, string fill)", "Pads at the end up to width characters."),
    ("text", "toNumber", "number Text.toNumber(string s)", "Parses \"42\", \"-3\", \"0xFF\" or \"2.5\"; errors on anything else."),
    ("text", "isNumber", "bool Text.isNumber(string s)", "True when Text.toNumber would succeed."),
    ("text", "format", "string Text.format(string template, any... values)", "Replaces each {} with the next value: Text.format(\"{} has {} hp\", name, hp)."),
    ("table", "keys", "array<K> Table.keys(dictionary<K, V> d)", "Keys in insertion order."),
    ("table", "values", "array<V> Table.values(dictionary<K, V> d)", "Values in insertion order."),
    ("table", "has", "bool Table.has(dictionary<K, V> d, K key)", "True when the key exists."),
    ("table", "get", "V Table.get(dictionary<K, V> d, K key, V fallback)", "Value for key, or fallback when missing."),
    ("table", "merge", "dictionary<K, V> Table.merge(dictionary<K, V> a, dictionary<K, V> b)", "New dictionary; b wins on equal keys."),
    ("table", "copy", "T Table.copy(T collection)", "Shallow copy of an array or dictionary."),
    ("table", "reverse", "array<T> Table.reverse(array<T> items)", "New array in reverse order."),
    ("table", "sorted", "array<T> Table.sorted(array<T> items)", "New sorted array (numbers or strings)."),
    ("table", "contains", "bool Table.contains(array<T> items, T value)", "True when value is an element."),
    ("table", "indexOf", "number Table.indexOf(array<T> items, T value)", "Index of value, or -1."),
    ("table", "count", "number Table.count(array<T> items, T value)", "How many elements equal value."),
    ("table", "slice", "array<T> Table.slice(array<T> items, number start, number end)", "Elements from start up to end (excluded); bounds are clamped."),
    ("table", "concat", "array<T> Table.concat(array<T> a, array<T> b)", "New array with a then b."),
    ("table", "unique", "array<T> Table.unique(array<T> items)", "New array without repeated values."),
    ("table", "range", "array<number> Table.range(number start, number end)", "start, start + 1, … end - 1."),
    ("table", "fill", "array<T> Table.fill(number count, T value)", "count copies of value."),
    ("table", "sum", "number Table.sum(array<number> items)", "Sum of the elements."),
    ("table", "min", "number Table.min(array<number> items)", "Smallest element."),
    ("table", "max", "number Table.max(array<number> items)", "Largest element."),
    ("table", "isEmpty", "bool Table.isEmpty(T collection)", "True when the array or dictionary has no elements."),
    ("fs", "read", "string Fs.read(string path)", "Reads a whole file as text. Paths are relative to the working directory; '..' is not allowed."),
    ("fs", "write", "void Fs.write(string path, string text)", "Writes text to a file, replacing it. '..' is not allowed."),
    ("fs", "exists", "bool Fs.exists(string path)", "True when the file or folder exists."),
];

/// Module key for `@clpp.<key>`, and its default alias.
pub fn module_alias(link: &str) -> Option<(&'static str, &'static str)> {
    match link {
        "@clpp.math" => Some(("math", "Math")),
        "@clpp.text" => Some(("text", "Text")),
        "@clpp.table" => Some(("table", "Table")),
        "@clpp.fs" => Some(("fs", "Fs")),
        _ => None,
    }
}

pub fn members(module: &str) -> impl Iterator<Item = &'static (&'static str, &'static str, &'static str, &'static str)> + '_ {
    CATALOG.iter().filter(move |(m, ..)| *m == module)
}

pub fn is_constant(signature: &str) -> bool {
    !signature.contains('(')
}

/// Parameter count, or None for variadic members.
pub fn arity(signature: &str) -> Option<usize> {
    let inside = signature.split_once('(')?.1.trim_end_matches(')');
    if inside.contains("...") {
        return None;
    }
    if inside.trim().is_empty() {
        return Some(0);
    }
    // Commas inside type arguments (dictionary<K, V>) do not separate parameters.
    let mut depth = 0;
    let mut count = 1;
    for c in inside.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => count += 1,
            _ => {}
        }
    }
    Some(count)
}

// ==================================================================== calls

fn float(v: &SVal, what: &str) -> Result<f64, String> {
    match v {
        SVal::Int(i) => Ok(*i as f64),
        SVal::Float(f) => Ok(*f),
        other => Err(format!("{} needs a number, got {}", what, other.type_name())),
    }
}

fn int(v: &SVal, what: &str) -> Result<i64, String> {
    match v {
        SVal::Int(i) => Ok(*i),
        other => Err(format!("{} needs a whole number, got {}", what, other.type_name())),
    }
}

fn string<'a>(v: &'a SVal, what: &str) -> Result<&'a str, String> {
    match v {
        SVal::Str(s) => Ok(s),
        other => Err(format!("{} needs a string, got {}", what, other.type_name())),
    }
}

fn list<'a>(v: &'a SVal, what: &str) -> Result<&'a Vec<SVal>, String> {
    match v {
        SVal::List(l) => Ok(l),
        other => Err(format!("{} needs an array, got {}", what, other.type_name())),
    }
}

fn map<'a>(v: &'a SVal, what: &str) -> Result<&'a Vec<(SVal, SVal)>, String> {
    match v {
        SVal::Map(m) => Ok(m),
        other => Err(format!("{} needs a dictionary, got {}", what, other.type_name())),
    }
}

fn to_int(x: f64, what: &str) -> Result<SVal, String> {
    if !x.is_finite() || x >= 9.2e18 || x <= -9.2e18 {
        return Err(format!("{}: {} does not fit in a number", what, x));
    }
    Ok(SVal::Int(x as i64))
}

fn char_index(haystack: &str, byte: usize) -> i64 {
    haystack[..byte].chars().count() as i64
}

thread_local! {
    static RNG: Cell<u64> = Cell::new(
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0x9E37_79B9) | 1
    );
}

fn next_random() -> u64 {
    RNG.with(|r| {
        // xorshift64*
        let mut x = r.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        r.set(x);
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    })
}

fn compare(a: &SVal, b: &SVal) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (SVal::Int(x), SVal::Int(y)) => Some(x.cmp(y)),
        (SVal::Str(x), SVal::Str(y)) => Some(x.cmp(y)),
        _ => float(a, "").ok()?.partial_cmp(&float(b, "").ok()?),
    }
}

fn checked_path(path: &str, what: &str) -> Result<(), String> {
    if std::path::Path::new(path).components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(format!("{}: '..' is not allowed in paths ('{}')", what, path));
    }
    Ok(())
}

/// Calls `module.member` (or reads a constant when `member` is one).
pub fn call(module: &str, member: &str, args: &[SVal]) -> Result<SVal, String> {
    let Some((_, _, signature, _)) = CATALOG.iter().find(|(m, n, ..)| *m == module && *n == member) else {
        return Err(format!("{} has no member '{}'", module, member));
    };
    let name = format!("{}.{}", module, member);
    if !is_constant(signature) {
        if let Some(n) = arity(signature) {
            if args.len() != n {
                return Err(format!("{} expects {} argument(s), got {}", signature, n, args.len()));
            }
        }
    }
    let a = |i: usize| &args[i];
    let f1 = |op: fn(f64) -> f64| -> Result<SVal, String> { Ok(SVal::Float(op(float(a(0), &name)?))) };
    Ok(match (module, member) {
        ("math", "pi") => SVal::Float(std::f64::consts::PI),
        ("math", "tau") => SVal::Float(std::f64::consts::TAU),
        ("math", "e") => SVal::Float(std::f64::consts::E),
        ("math", "huge") => SVal::Float(f64::INFINITY),
        ("math", "nan") => SVal::Float(f64::NAN),
        ("math", "epsilon") => SVal::Float(f64::EPSILON),
        ("math", "maxInt") => SVal::Int(i64::MAX),
        ("math", "minInt") => SVal::Int(i64::MIN),
        ("math", "abs") => match a(0) {
            SVal::Int(i) => SVal::Int(i.checked_abs().ok_or("Math.abs: result does not fit in a number")?),
            v => SVal::Float(float(v, &name)?.abs()),
        },
        ("math", "sign") => match a(0) {
            SVal::Int(i) => SVal::Int(i.signum()),
            v => {
                let x = float(v, &name)?;
                SVal::Int(if x > 0.0 { 1 } else if x < 0.0 { -1 } else { 0 })
            }
        },
        ("math", "floor") => to_int(float(a(0), &name)?.floor(), &name)?,
        ("math", "ceil") => to_int(float(a(0), &name)?.ceil(), &name)?,
        ("math", "round") => to_int(float(a(0), &name)?.round(), &name)?,
        ("math", "trunc") => to_int(float(a(0), &name)?.trunc(), &name)?,
        ("math", "fract") => {
            let x = float(a(0), &name)?;
            SVal::Float(x - x.trunc())
        }
        ("math", "sqrt") => {
            let x = float(a(0), &name)?;
            if x < 0.0 {
                return Err(format!("Math.sqrt of a negative number ({})", x));
            }
            SVal::Float(x.sqrt())
        }
        ("math", "cbrt") => f1(f64::cbrt)?,
        ("math", "pow") => match (a(0), a(1)) {
            (SVal::Int(b), SVal::Int(e)) if *e >= 0 && *e <= u32::MAX as i64 => {
                SVal::Int(b.checked_pow(*e as u32).ok_or("Math.pow: result does not fit in a number; use floats")?)
            }
            (b, e) => SVal::Float(float(b, &name)?.powf(float(e, &name)?)),
        },
        ("math", "exp") => f1(f64::exp)?,
        ("math", "log") => f1(f64::ln)?,
        ("math", "log2") => f1(f64::log2)?,
        ("math", "log10") => f1(f64::log10)?,
        ("math", "sin") => f1(f64::sin)?,
        ("math", "cos") => f1(f64::cos)?,
        ("math", "tan") => f1(f64::tan)?,
        ("math", "asin") => f1(f64::asin)?,
        ("math", "acos") => f1(f64::acos)?,
        ("math", "atan") => f1(f64::atan)?,
        ("math", "sinh") => f1(f64::sinh)?,
        ("math", "cosh") => f1(f64::cosh)?,
        ("math", "tanh") => f1(f64::tanh)?,
        ("math", "deg") => f1(f64::to_degrees)?,
        ("math", "rad") => f1(f64::to_radians)?,
        ("math", "atan2") => SVal::Float(float(a(0), &name)?.atan2(float(a(1), &name)?)),
        ("math", "hypot") => SVal::Float(float(a(0), &name)?.hypot(float(a(1), &name)?)),
        ("math", "min") | ("math", "max") => {
            let ord = compare(a(0), a(1)).ok_or(format!("{} needs two numbers", name))?;
            let first = (member == "min") == (ord != std::cmp::Ordering::Greater);
            if first { a(0).clone() } else { a(1).clone() }
        }
        ("math", "clamp") => {
            let (x, lo, hi) = (a(0), a(1), a(2));
            if compare(lo, hi) == Some(std::cmp::Ordering::Greater) {
                return Err("Math.clamp: low is greater than high".into());
            }
            if compare(x, lo) == Some(std::cmp::Ordering::Less) {
                lo.clone()
            } else if compare(x, hi) == Some(std::cmp::Ordering::Greater) {
                hi.clone()
            } else {
                float(x, &name).map(|_| x.clone())?
            }
        }
        ("math", "lerp") => {
            let (x, y, t) = (float(a(0), &name)?, float(a(1), &name)?, float(a(2), &name)?);
            SVal::Float(x + (y - x) * t)
        }
        ("math", "isNan") => SVal::Bool(float(a(0), &name)?.is_nan()),
        ("math", "isInf") => SVal::Bool(float(a(0), &name)?.is_infinite()),
        ("math", "random") => SVal::Float((next_random() >> 11) as f64 / (1u64 << 53) as f64),
        ("math", "randomInt") => {
            let (lo, hi) = (int(a(0), &name)?, int(a(1), &name)?);
            if lo > hi {
                return Err("Math.randomInt: low is greater than high".into());
            }
            let span = (hi as i128 - lo as i128 + 1) as u128;
            SVal::Int((lo as i128 + (next_random() as u128 % span) as i128) as i64)
        }
        ("math", "seed") => {
            let s = int(a(0), &name)? as u64;
            RNG.with(|r| r.set(s.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1));
            SVal::Null
        }

        ("text", "trim") => SVal::Str(string(a(0), &name)?.trim().to_string()),
        ("text", "trimStart") => SVal::Str(string(a(0), &name)?.trim_start().to_string()),
        ("text", "trimEnd") => SVal::Str(string(a(0), &name)?.trim_end().to_string()),
        ("text", "lower") => SVal::Str(string(a(0), &name)?.to_lowercase()),
        ("text", "upper") => SVal::Str(string(a(0), &name)?.to_uppercase()),
        ("text", "contains") => SVal::Bool(string(a(0), &name)?.contains(string(a(1), &name)?)),
        ("text", "startsWith") => SVal::Bool(string(a(0), &name)?.starts_with(string(a(1), &name)?)),
        ("text", "endsWith") => SVal::Bool(string(a(0), &name)?.ends_with(string(a(1), &name)?)),
        ("text", "indexOf") => {
            let s = string(a(0), &name)?;
            SVal::Int(s.find(string(a(1), &name)?).map(|b| char_index(s, b)).unwrap_or(-1))
        }
        ("text", "lastIndexOf") => {
            let s = string(a(0), &name)?;
            SVal::Int(s.rfind(string(a(1), &name)?).map(|b| char_index(s, b)).unwrap_or(-1))
        }
        ("text", "count") => {
            let part = string(a(1), &name)?;
            if part.is_empty() {
                return Err("Text.count: the part to count is empty".into());
            }
            SVal::Int(string(a(0), &name)?.matches(part).count() as i64)
        }
        ("text", "replace") => {
            let from = string(a(1), &name)?;
            if from.is_empty() {
                return Err("Text.replace: the text to replace is empty".into());
            }
            SVal::Str(string(a(0), &name)?.replace(from, string(a(2), &name)?))
        }
        ("text", "split") => {
            let (s, sep) = (string(a(0), &name)?, string(a(1), &name)?);
            if sep.is_empty() {
                SVal::List(s.chars().map(|c| SVal::Str(c.to_string())).collect())
            } else {
                SVal::List(s.split(sep).map(|p| SVal::Str(p.to_string())).collect())
            }
        }
        ("text", "join") => {
            let sep = string(a(1), &name)?;
            SVal::Str(list(a(0), &name)?.iter().map(SVal::text).collect::<Vec<_>>().join(sep))
        }
        ("text", "repeat") => {
            let n = int(a(1), &name)?;
            if n < 0 {
                return Err("Text.repeat: times must not be negative".into());
            }
            SVal::Str(string(a(0), &name)?.repeat(n as usize))
        }
        ("text", "reverse") => SVal::Str(string(a(0), &name)?.chars().rev().collect()),
        ("text", "chars") => SVal::List(string(a(0), &name)?.chars().map(|c| SVal::Str(c.to_string())).collect()),
        ("text", "code") => match string(a(0), &name)?.chars().next() {
            Some(c) => SVal::Int(c as i64),
            None => return Err("Text.code of an empty string".into()),
        },
        ("text", "fromCode") => {
            let n = int(a(0), &name)?;
            match u32::try_from(n).ok().and_then(char::from_u32) {
                Some(c) => SVal::Str(c.to_string()),
                None => return Err(format!("Text.fromCode: {} is not a Unicode character", n)),
            }
        }
        ("text", "padLeft") | ("text", "padRight") => {
            let (s, width, fill) = (string(a(0), &name)?, int(a(1), &name)?, string(a(2), &name)?);
            let fill_char = fill.chars().next().ok_or(format!("{}: fill must not be empty", name))?;
            let missing = (width.max(0) as usize).saturating_sub(s.chars().count());
            let pad: String = std::iter::repeat(fill_char).take(missing).collect();
            SVal::Str(if member == "padLeft" { pad + s } else { s.to_string() + &pad })
        }
        ("text", "toNumber") | ("text", "isNumber") => {
            let s = string(a(0), &name)?.trim();
            let clean: String = s.chars().filter(|c| *c != '_').collect();
            let (negative, digits) = match clean.strip_prefix('-') {
                Some(rest) => (true, rest.to_string()),
                None => (false, clean.clone()),
            };
            let parsed = if let Some(h) = digits.strip_prefix("0x").or(digits.strip_prefix("0X")) {
                i64::from_str_radix(h, 16).ok().map(SVal::Int)
            } else if let Some(b) = digits.strip_prefix("0b").or(digits.strip_prefix("0B")) {
                i64::from_str_radix(b, 2).ok().map(SVal::Int)
            } else if let Ok(i) = digits.parse::<i64>() {
                Some(SVal::Int(i))
            } else if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-')) {
                digits.parse::<f64>().ok().map(SVal::Float)
            } else {
                None
            };
            let parsed = parsed.map(|v| match (negative, v) {
                (true, SVal::Int(i)) => SVal::Int(-i),
                (true, SVal::Float(f)) => SVal::Float(-f),
                (_, v) => v,
            });
            if member == "isNumber" {
                SVal::Bool(parsed.is_some())
            } else {
                parsed.ok_or(format!("Text.toNumber: '{}' is not a number", s))?
            }
        }
        ("text", "format") => {
            if args.is_empty() {
                return Err("Text.format needs a template".into());
            }
            let template = string(a(0), &name)?;
            let mut out = String::new();
            let mut next = 1;
            let mut rest = template;
            while let Some(i) = rest.find("{}") {
                out.push_str(&rest[..i]);
                let value = args.get(next).ok_or(format!("Text.format: the template has more {{}} than values ({})", args.len() - 1))?;
                out.push_str(&value.text());
                next += 1;
                rest = &rest[i + 2..];
            }
            out.push_str(rest);
            if next < args.len() {
                return Err(format!("Text.format: {} value(s) but only {} {{}} in the template", args.len() - 1, next - 1));
            }
            SVal::Str(out)
        }

        ("table", "keys") => SVal::List(map(a(0), &name)?.iter().map(|(k, _)| k.clone()).collect()),
        ("table", "values") => SVal::List(map(a(0), &name)?.iter().map(|(_, v)| v.clone()).collect()),
        ("table", "has") => SVal::Bool(map(a(0), &name)?.iter().any(|(k, _)| same(k, a(1)))),
        ("table", "get") => map(a(0), &name)?.iter().find(|(k, _)| same(k, a(1))).map(|(_, v)| v.clone()).unwrap_or_else(|| a(2).clone()),
        ("table", "merge") => {
            let mut out = map(a(0), &name)?.clone();
            for (k, v) in map(a(1), &name)? {
                match out.iter_mut().find(|(e, _)| same(e, k)) {
                    Some(e) => e.1 = v.clone(),
                    None => out.push((k.clone(), v.clone())),
                }
            }
            SVal::Map(out)
        }
        ("table", "copy") => match a(0) {
            SVal::List(_) | SVal::Map(_) => a(0).clone(),
            other => return Err(format!("Table.copy needs an array or dictionary, got {}", other.type_name())),
        },
        ("table", "reverse") => SVal::List(list(a(0), &name)?.iter().rev().cloned().collect()),
        ("table", "sorted") => {
            let mut items = list(a(0), &name)?.clone();
            let mut bad = false;
            items.sort_by(|x, y| compare(x, y).unwrap_or_else(|| {
                bad = true;
                std::cmp::Ordering::Equal
            }));
            if bad {
                return Err("Table.sorted needs an array of numbers or of strings".into());
            }
            SVal::List(items)
        }
        ("table", "contains") => SVal::Bool(list(a(0), &name)?.iter().any(|v| same(v, a(1)))),
        ("table", "indexOf") => SVal::Int(list(a(0), &name)?.iter().position(|v| same(v, a(1))).map(|i| i as i64).unwrap_or(-1)),
        ("table", "count") => SVal::Int(list(a(0), &name)?.iter().filter(|v| same(v, a(1))).count() as i64),
        ("table", "slice") => {
            let items = list(a(0), &name)?;
            let n = items.len() as i64;
            let start = int(a(1), &name)?.clamp(0, n) as usize;
            let end = int(a(2), &name)?.clamp(0, n) as usize;
            SVal::List(if start < end { items[start..end].to_vec() } else { Vec::new() })
        }
        ("table", "concat") => {
            let mut out = list(a(0), &name)?.clone();
            out.extend(list(a(1), &name)?.iter().cloned());
            SVal::List(out)
        }
        ("table", "unique") => {
            let mut out: Vec<SVal> = Vec::new();
            for v in list(a(0), &name)? {
                if !out.iter().any(|o| same(o, v)) {
                    out.push(v.clone());
                }
            }
            SVal::List(out)
        }
        ("table", "range") => {
            let (start, end) = (int(a(0), &name)?, int(a(1), &name)?);
            if end.saturating_sub(start) > 10_000_000 {
                return Err("Table.range: more than 10 million elements".into());
            }
            SVal::List((start..end).map(SVal::Int).collect())
        }
        ("table", "fill") => {
            let n = int(a(0), &name)?;
            if !(0..=10_000_000).contains(&n) {
                return Err("Table.fill: count must be between 0 and 10 million".into());
            }
            SVal::List(vec![a(1).clone(); n as usize])
        }
        ("table", "sum") => {
            let items = list(a(0), &name)?;
            if items.iter().all(|v| matches!(v, SVal::Int(_))) {
                let mut total: i64 = 0;
                for v in items {
                    total = total.wrapping_add(int(v, &name)?);
                }
                SVal::Int(total)
            } else {
                let mut total = 0.0;
                for v in items {
                    total += float(v, &name)?;
                }
                SVal::Float(total)
            }
        }
        ("table", "min") | ("table", "max") => {
            let items = list(a(0), &name)?;
            let mut best = items.first().ok_or(format!("{} of an empty array", name))?.clone();
            for v in &items[1..] {
                let ord = compare(v, &best).ok_or(format!("{} needs numbers or strings", name))?;
                if (member == "min" && ord == std::cmp::Ordering::Less) || (member == "max" && ord == std::cmp::Ordering::Greater) {
                    best = v.clone();
                }
            }
            best
        }
        ("table", "isEmpty") => match a(0) {
            SVal::List(l) => SVal::Bool(l.is_empty()),
            SVal::Map(m) => SVal::Bool(m.is_empty()),
            other => return Err(format!("Table.isEmpty needs an array or dictionary, got {}", other.type_name())),
        },

        ("fs", "read") => {
            let path = string(a(0), &name)?;
            checked_path(path, "Fs.read")?;
            SVal::Str(std::fs::read_to_string(path).map_err(|e| format!("Fs.read('{}'): {}", path, e))?)
        }
        ("fs", "write") => {
            let path = string(a(0), &name)?;
            checked_path(path, "Fs.write")?;
            std::fs::write(path, string(a(1), &name)?).map_err(|e| format!("Fs.write('{}'): {}", path, e))?;
            SVal::Null
        }
        ("fs", "exists") => {
            let path = string(a(0), &name)?;
            checked_path(path, "Fs.exists")?;
            SVal::Bool(std::path::Path::new(path).exists())
        }
        _ => return Err(format!("{} is not implemented", name)),
    })
}

// ==================================================================== numeric types

pub const NUMERIC_TYPES: &[&str] = &["number", "float", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f16", "f32", "f64"];

pub fn is_numeric_type(name: &str) -> bool {
    NUMERIC_TYPES.contains(&name)
}

fn int_range(name: &str) -> Option<(i128, i128)> {
    Some(match name {
        "i8" => (i8::MIN as i128, i8::MAX as i128),
        "i16" => (i16::MIN as i128, i16::MAX as i128),
        "i32" => (i32::MIN as i128, i32::MAX as i128),
        "i64" | "number" => (i64::MIN as i128, i64::MAX as i128),
        "u8" => (0, u8::MAX as i128),
        "u16" => (0, u16::MAX as i128),
        "u32" => (0, u32::MAX as i128),
        // u64 values above i64::MAX cannot be represented by `number` yet.
        "u64" => (0, i64::MAX as i128),
        _ => return None,
    })
}

/// Nearest IEEE 754 half-precision value (ties to even); None on overflow.
pub fn round_f16(x: f64) -> Option<f64> {
    if x.is_nan() || x.is_infinite() || x == 0.0 {
        return Some(x);
    }
    let a = x.abs();
    let mut e = a.log2().floor() as i32;
    if 2f64.powi(e) > a {
        e -= 1;
    }
    if 2f64.powi(e + 1) <= a {
        e += 1;
    }
    let e = e.max(-14); // subnormals share the smallest exponent
    let quantum = 2f64.powi(e - 10);
    let q = a / quantum;
    let floor = q.floor();
    let diff = q - floor;
    let rounded = if diff > 0.5 || (diff == 0.5 && floor % 2.0 != 0.0) { floor + 1.0 } else { floor };
    let r = rounded * quantum;
    if r > 65504.0 {
        return None;
    }
    Some(if x < 0.0 { -r } else { r })
}

/// Converts a number to a sized type.
///
/// `exact` is true for literals and `(Type value)`: a value out of range is
/// an error. Computed values wrap around (two's complement) like the CPU.
pub fn coerce(type_name: &str, value: &SVal, exact: bool) -> Result<SVal, String> {
    if let Some((lo, hi)) = int_range(type_name) {
        let i = match value {
            SVal::Int(i) => *i as i128,
            SVal::Float(f) => {
                return Err(format!(
                    "{} needs a whole number, got {} (convert with Math.floor, Math.round or Math.trunc)",
                    type_name,
                    SVal::Float(*f).text()
                ))
            }
            other => return Err(format!("{} needs a number, got {}", type_name, other.type_name())),
        };
        if i >= lo && i <= hi {
            return Ok(SVal::Int(i as i64));
        }
        if exact {
            return Err(format!("{} does not fit in {} (range {} .. {})", i, type_name, lo, hi));
        }
        let bits = match type_name {
            "i8" | "u8" => 8,
            "i16" | "u16" => 16,
            "i32" | "u32" => 32,
            _ => 64,
        };
        let mask: i128 = (1i128 << bits) - 1;
        let mut wrapped = i & mask;
        if lo < 0 && wrapped > hi {
            wrapped -= 1i128 << bits;
        }
        return Ok(SVal::Int(wrapped as i64));
    }
    let x = match value {
        SVal::Int(i) => *i as f64,
        SVal::Float(f) => *f,
        other => return Err(format!("{} needs a number, got {}", type_name, other.type_name())),
    };
    match type_name {
        "float" | "f64" => Ok(SVal::Float(x)),
        "f32" => {
            let r = x as f32;
            if r.is_infinite() && x.is_finite() {
                if exact {
                    return Err(format!("{} does not fit in f32 (largest is about 3.4e38)", SVal::Float(x).text()));
                }
            }
            Ok(SVal::Float(r as f64))
        }
        "f16" => match round_f16(x) {
            Some(r) => Ok(SVal::Float(r)),
            None if exact => Err(format!("{} does not fit in f16 (largest is 65504)", SVal::Float(x).text())),
            None => Ok(SVal::Float(if x < 0.0 { f64::NEG_INFINITY } else { f64::INFINITY })),
        },
        _ => Ok(value.clone()),
    }
}
