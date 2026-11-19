//! A small JSON writer: ordered keys, explicit comma placement, one escaper.
//!
//! The projections used to build JSON by appending to a String, which is how a
//! double comma and a stray quote both shipped once. Here the commas are the
//! writer's job and cannot be forgotten, keys keep the order the caller gave,
//! and the only escaper in the crate is `escape`.

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    /// Numbers are carried as their printed form: the caller already decided
    /// the precision, and re-formatting it here would silently change it.
    Num(String),
    Bool(bool),
    Null,
    /// `[a, b]` - for short lists of scalars, which read better on one line.
    Inline(Vec<Value>),
    /// One item per line - for objects and for lists that carry structure.
    Block(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

impl Value {
    pub fn obj(fields: Vec<(&str, Value)>) -> Value {
        Value::Obj(
            fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    pub fn strs<S: AsRef<str>>(items: &[S]) -> Value {
        Value::Inline(
            items
                .iter()
                .map(|s| Value::Str(s.as_ref().to_string()))
                .collect(),
        )
    }

    pub fn num<T: std::fmt::Display>(v: T) -> Value {
        Value::Num(v.to_string())
    }

    /// Render with two-space indentation, the shape every projection uses.
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, indent: usize) {
        match self {
            Value::Str(s) => {
                out.push('"');
                out.push_str(&escape(s));
                out.push('"');
            }
            Value::Num(n) => out.push_str(n),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Null => out.push_str("null"),
            Value::Inline(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    v.write(out, indent);
                }
                out.push(']');
            }
            Value::Block(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    out.push('\n');
                    push_indent(out, indent + 2);
                    v.write(out, indent + 2);
                    if i + 1 < items.len() {
                        out.push(',');
                    }
                }
                out.push('\n');
                push_indent(out, indent);
                out.push(']');
            }
            Value::Obj(fields) => {
                if fields.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    out.push('\n');
                    push_indent(out, indent + 2);
                    out.push('"');
                    out.push_str(&escape(k));
                    out.push_str("\": ");
                    v.write(out, indent + 2);
                    if i + 1 < fields.len() {
                        out.push(',');
                    }
                }
                out.push('\n');
                push_indent(out, indent);
                out.push('}');
            }
        }
    }
}

fn push_indent(out: &mut String, n: usize) {
    for _ in 0..n {
        out.push(' ');
    }
}

/// The single escaper. `projection::json_escape` and `signals::esc` both route
/// here, so a fix lands everywhere at once.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Parse JSON text into the same `Value` tree the writer emits, so a stored
/// document can be read back, modified as data, and rendered again without a
/// second representation. Arrays written on one line come back as `Inline`;
/// multi-line ones as `Block`, which keeps `render(parse(render(v))) ==
/// render(v)` for every shape this crate writes.
pub fn parse(src: &str) -> Result<Value, String> {
    let cs: Vec<char> = src.chars().collect();
    let mut p = 0usize;
    let v = parse_value(&cs, &mut p)?;
    skip_ws(&cs, &mut p);
    if p < cs.len() {
        return Err(format!("trailing input at char {p}"));
    }
    Ok(v)
}

fn skip_ws(cs: &[char], p: &mut usize) {
    while *p < cs.len() && matches!(cs[*p], ' ' | '\t' | '\n' | '\r') {
        *p += 1;
    }
}

fn peek(cs: &[char], p: usize) -> Result<char, String> {
    cs.get(p)
        .copied()
        .ok_or_else(|| "unexpected end of input".to_string())
}

fn parse_value(cs: &[char], p: &mut usize) -> Result<Value, String> {
    skip_ws(cs, p);
    match peek(cs, *p)? {
        '{' => parse_obj(cs, p),
        '[' => parse_arr(cs, p),
        '"' => parse_string(cs, p).map(Value::Str),
        't' => literal(cs, p, "true", Value::Bool(true)),
        'f' => literal(cs, p, "false", Value::Bool(false)),
        'n' => literal(cs, p, "null", Value::Null),
        c if c == '-' || c.is_ascii_digit() => parse_num(cs, p),
        c => Err(format!("unexpected character {c:?} at char {p}")),
    }
}

fn literal(cs: &[char], p: &mut usize, word: &str, v: Value) -> Result<Value, String> {
    for c in word.chars() {
        if peek(cs, *p)? != c {
            return Err(format!("expected {word:?} at char {p}"));
        }
        *p += 1;
    }
    Ok(v)
}

fn parse_num(cs: &[char], p: &mut usize) -> Result<Value, String> {
    let start = *p;
    if peek(cs, *p)? == '-' {
        *p += 1;
    }
    while *p < cs.len()
        && (cs[*p].is_ascii_digit() || matches!(cs[*p], '.' | 'e' | 'E' | '+' | '-'))
    {
        *p += 1;
    }
    let raw: String = cs[start..*p].iter().collect();
    raw.parse::<f64>()
        .map_err(|_| format!("invalid number {raw:?} at char {start}"))?;
    Ok(Value::Num(raw))
}

fn parse_obj(cs: &[char], p: &mut usize) -> Result<Value, String> {
    *p += 1;
    let mut fields = Vec::new();
    skip_ws(cs, p);
    if peek(cs, *p)? == '}' {
        *p += 1;
        return Ok(Value::Obj(fields));
    }
    loop {
        skip_ws(cs, p);
        let k = parse_string(cs, p)?;
        skip_ws(cs, p);
        if peek(cs, *p)? != ':' {
            return Err(format!("expected ':' at char {p}"));
        }
        *p += 1;
        let v = parse_value(cs, p)?;
        fields.push((k, v));
        skip_ws(cs, p);
        match peek(cs, *p)? {
            ',' => *p += 1,
            '}' => {
                *p += 1;
                return Ok(Value::Obj(fields));
            }
            c => return Err(format!("expected ',' or '}}' at char {p}, got {c:?}")),
        }
    }
}

fn parse_arr(cs: &[char], p: &mut usize) -> Result<Value, String> {
    let start = *p;
    *p += 1;
    let mut items = Vec::new();
    skip_ws(cs, p);
    if peek(cs, *p)? == ']' {
        *p += 1;
        return Ok(Value::Inline(items));
    }
    loop {
        items.push(parse_value(cs, p)?);
        skip_ws(cs, p);
        match peek(cs, *p)? {
            ',' => *p += 1,
            ']' => {
                *p += 1;
                // Layout is read back from the source: a bracket pair with no
                // interior newline was written on one line.
                let interior: String = cs[start..*p].iter().collect();
                if interior.contains('\n') {
                    return Ok(Value::Block(items));
                }
                return Ok(Value::Inline(items));
            }
            c => return Err(format!("expected ',' or ']' at char {p}, got {c:?}")),
        }
    }
}

fn parse_string(cs: &[char], p: &mut usize) -> Result<String, String> {
    if peek(cs, *p)? != '"' {
        return Err(format!("expected a string at char {p}"));
    }
    *p += 1;
    let mut s = String::new();
    loop {
        let c = peek(cs, *p)?;
        *p += 1;
        match c {
            '"' => return Ok(s),
            '\\' => {
                let e = peek(cs, *p)?;
                *p += 1;
                match e {
                    '"' => s.push('"'),
                    '\\' => s.push('\\'),
                    '/' => s.push('/'),
                    'b' => s.push('\u{0008}'),
                    'f' => s.push('\u{000C}'),
                    'n' => s.push('\n'),
                    'r' => s.push('\r'),
                    't' => s.push('\t'),
                    'u' => s.push(unicode_escape(cs, p)?),
                    c => return Err(format!("unknown escape \\{c} at char {}", *p - 1)),
                }
            }
            c if (c as u32) < 0x20 => {
                return Err(format!("unescaped control character at char {}", *p - 1))
            }
            c => s.push(c),
        }
    }
}

/// `\uXXXX`, combining a surrogate pair when the first code unit is high.
fn unicode_escape(cs: &[char], p: &mut usize) -> Result<char, String> {
    let hi = hex4(cs, p)?;
    let unit = if (0xD800..0xDC00).contains(&hi) {
        if peek(cs, *p)? != '\\' || peek(cs, *p + 1)? != 'u' {
            return Err(format!("lone high surrogate at char {p}"));
        }
        *p += 2;
        let lo = hex4(cs, p)?;
        0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
    } else {
        hi
    };
    char::from_u32(unit).ok_or_else(|| format!("invalid \\u escape at char {p}"))
}

fn hex4(cs: &[char], p: &mut usize) -> Result<u32, String> {
    let mut v = 0u32;
    for _ in 0..4 {
        let c = peek(cs, *p)?;
        let d = c
            .to_digit(16)
            .ok_or_else(|| format!("invalid hex digit {c:?} at char {p}"))?;
        v = v * 16 + d;
        *p += 1;
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_keep_key_order_and_indent() {
        let v = Value::obj(vec![("b", Value::num(1)), ("a", Value::Str("x".into()))]);
        assert_eq!(v.render(), "{\n  \"b\": 1,\n  \"a\": \"x\"\n}");
    }

    #[test]
    fn empty_containers_are_compact() {
        assert_eq!(Value::Block(vec![]).render(), "[]");
        assert_eq!(Value::Obj(vec![]).render(), "{}");
    }

    #[test]
    fn nested_objects_indent_recursively() {
        let v = Value::obj(vec![("a", Value::obj(vec![("b", Value::Bool(true))]))]);
        assert_eq!(v.render(), "{\n  \"a\": {\n    \"b\": true\n  }\n}");
    }

    #[test]
    fn block_lists_put_one_item_per_line() {
        let v = Value::Block(vec![Value::Str("x".into()), Value::Str("y".into())]);
        assert_eq!(v.render(), "[\n  \"x\",\n  \"y\"\n]");
    }

    #[test]
    fn escaping_is_the_writer_s_job() {
        assert_eq!(
            Value::Str("a\"b\\c\n".into()).render(),
            "\"a\\\"b\\\\c\\n\""
        );
    }

    #[test]
    fn parsed_documents_render_identically() {
        let v = Value::obj(vec![
            ("inline", Value::strs(&["a", "b"])),
            (
                "block",
                Value::Block(vec![Value::num(1), Value::Bool(false)]),
            ),
            ("nested", Value::obj(vec![("x", Value::Null)])),
            ("num", Value::Num("39.5".into())),
        ]);
        let doc = v.render();
        assert_eq!(parse(&doc).unwrap().render(), doc);
    }

    #[test]
    fn parse_rejects_broken_documents() {
        assert!(parse("1 x").is_err()); // trailing input
        assert!(parse("{\"a\":}").is_err()); // missing value
        assert!(parse("[1,]").is_err()); // trailing comma
        assert!(parse("{\"a\" 1}").is_err()); // missing colon
    }

    #[test]
    fn strings_survive_escape_round_trip() {
        let s = "quote\" back\\slash\n\tcé";
        assert_eq!(
            parse(&Value::Str(s.into()).render()).unwrap(),
            Value::Str(s.into())
        );
    }

    #[test]
    fn surrogate_pairs_decode() {
        let v = parse("\"\\uD83D\\uDE00\"").unwrap();
        assert_eq!(v, Value::Str("\u{1F600}".into()));
    }

    #[test]
    fn numbers_keep_their_written_form() {
        let v = parse("[0.50, 3, -2e3]").unwrap();
        assert_eq!(
            v.render(),
            Value::Inline(vec![
                Value::Num("0.50".into()),
                Value::Num("3".into()),
                Value::Num("-2e3".into()),
            ])
            .render()
        );
    }
}
