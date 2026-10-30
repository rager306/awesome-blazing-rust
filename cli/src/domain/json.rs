//! A small JSON writer: ordered keys, explicit comma placement, one escaper.
//!
//! The projections used to build JSON by appending to a String, which is how a
//! double comma and a stray quote both shipped once. Here the commas are the
//! writer's job and cannot be forgotten, keys keep the order the caller gave,
//! and the only escaper in the crate is `escape`.

#[derive(Debug, Clone)]
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
}
