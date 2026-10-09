//! The portable expression subset (§10.7) — what `@computed`, a rollup's `where:` and a view's `where`
//! are written in, and what lies outside it.
//!
//! This is not a parser: an expression is still stored as text and its meaning is the implementation's
//! to translate. What is checked here is the line §10.7 draws — a construct a portable expression cannot
//! contain (a function not in the closed list, a bracketed or quoted identifier, `||`, a cast, a JSON
//! operator, `+` joining text) — so that a model which will not travel says so (`M3L-W011`) instead of
//! being guessed at by the next implementation.

/// The closed list of portable functions (§10.7.4), lower case. Names are matched without regard to case.
pub const PORTABLE_FUNCTIONS: &[&str] = &[
    "coalesce",
    "nullif",
    "concat",
    "lower",
    "upper",
    "trim",
    "length",
    "substring",
    "replace",
    "abs",
    "round",
    "floor",
    "ceiling",
    "today",
    "now",
    "date_add",
    "date_diff",
    "year",
    "month",
    "day",
    "generate_uuid",
];

/// Keywords of the subset (§10.7.2). `IN (` is a keyword followed by a list, not a function call.
const KEYWORDS: &[&str] = &[
    "and", "or", "not", "is", "null", "in", "between", "like", "case", "when", "then", "else",
    "end", "true", "false",
];

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Text,
    Number,
    Name(String),
    Plus,
    OpenParen,
    Other,
}

/// The constructs in `expression` that are outside the portable subset, each named once, in the order
/// they appear. Empty when the expression is portable as far as its tokens show.
///
/// `is_text` answers whether a name refers to a text-valued field — it lets `+` between two text fields
/// be named as the concatenation it is. A name it does not recognise is assumed not to be text.
pub fn non_portable_constructs(expression: &str, is_text: &dyn Fn(&str) -> bool) -> Vec<String> {
    let (tokens, mut found) = tokenize(expression);

    for (k, (at, token)) in tokens.iter().enumerate() {
        match token {
            Token::Name(name) if matches!(tokens.get(k + 1), Some((_, Token::OpenParen))) => {
                let lower = name.to_ascii_lowercase();
                if !PORTABLE_FUNCTIONS.contains(&lower.as_str())
                    && !KEYWORDS.contains(&lower.as_str())
                {
                    found.push((*at, format!("function {name}()")));
                }
            }
            Token::Plus => {
                let text_operand = |t: Option<&(usize, Token)>| match t {
                    Some((_, Token::Text)) => true,
                    Some((_, Token::Name(n))) => is_text(n),
                    _ => false,
                };
                if text_operand(k.checked_sub(1).and_then(|p| tokens.get(p)))
                    || text_operand(tokens.get(k + 1))
                {
                    found.push((*at, "+ joining text (write concat(…))".to_string()));
                }
            }
            _ => {}
        }
    }

    found.sort_by_key(|(at, _)| *at);
    let mut named: Vec<String> = Vec::new();
    for (_, what) in found {
        if !named.contains(&what) {
            named.push(what);
        }
    }
    named
}

/// Tokens with where each starts, and constructs (with where each starts) found outside the subset.
type Tokenized = (Vec<(usize, Token)>, Vec<(usize, String)>);

/// The expression's tokens with where each starts, and the constructs the characters alone already
/// place outside the subset.
fn tokenize(expression: &str) -> Tokenized {
    let chars: Vec<char> = expression.chars().collect();
    let mut tokens: Vec<(usize, Token)> = Vec::new();
    let mut found: Vec<(usize, String)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let token = if c == '\'' {
            // A string literal; a doubled quote is an escaped quote.
            i += 1;
            while i < chars.len() {
                if chars[i] == '\'' {
                    if chars.get(i + 1) == Some(&'\'') {
                        i += 2;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            i += 1;
            Token::Text
        } else if c.is_ascii_digit() {
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            Token::Number
        } else if c.is_alphabetic() || c == '_' || c == '$' {
            while i < chars.len()
                && (chars[i].is_alphanumeric()
                    || chars[i] == '_'
                    || chars[i] == '.'
                    || chars[i] == '$')
            {
                i += 1;
            }
            Token::Name(chars[start..i].iter().collect())
        } else if c == '[' || c == '"' || c == '`' {
            let close = if c == '[' { ']' } else { c };
            i += 1;
            while i < chars.len() && chars[i] != close {
                i += 1;
            }
            i = (i + 1).min(chars.len());
            let quoted: String = chars[start..i].iter().collect();
            let kind = if c == '[' { "bracketed" } else { "quoted" };
            found.push((start, format!("{kind} identifier {quoted}")));
            Token::Other
        } else {
            let ahead: String = chars[i..(i + 3).min(chars.len())].iter().collect();
            let (width, construct) = if ahead.starts_with("->>") {
                (3, Some("JSON operator ->>".to_string()))
            } else if ahead.starts_with("->") {
                (2, Some("JSON operator ->".to_string()))
            } else if ahead.starts_with("||") {
                (2, Some("|| (write concat(…))".to_string()))
            } else if ahead.starts_with("::") {
                (2, Some(":: cast".to_string()))
            } else if ["<>", "!=", "<=", ">="]
                .iter()
                .any(|op| ahead.starts_with(op))
            {
                (2, None)
            } else if "+-*/%=<>(),".contains(c) {
                (1, None)
            } else {
                (1, Some(format!("'{c}'")))
            };
            i += width;
            if let Some(construct) = construct {
                found.push((start, construct));
            }
            match c {
                '+' if width == 1 => Token::Plus,
                '(' => Token::OpenParen,
                _ => Token::Other,
            }
        };
        tokens.push((start, token));
    }
    (tokens, found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constructs(expression: &str) -> Vec<String> {
        non_portable_constructs(expression, &|name| {
            name == "first_name" || name == "last_name"
        })
    }

    #[test]
    fn the_subset_is_silent() {
        for e in [
            "quantity * unit_price",
            "concat(first_name, ' ', last_name)",
            "CASE WHEN total > 1000 THEN 'Gold' ELSE 'Bronze' END",
            "status IN ('paid', 'shipped') AND NOT is_deleted",
            "created_at > date_add(now(), -30, 'day')",
            "total_spent / NULLIF(order_count, 0)",
            "Order.status <> 'cancelled' AND $parent.region = Order.region",
            "name LIKE 'a%' OR name IS NULL",
            "'it''s' = label",
            "price - 1.5 >= 0",
        ] {
            assert!(constructs(e).is_empty(), "{e}: {:?}", constructs(e));
        }
    }

    #[test]
    fn platform_constructs_are_named_in_order() {
        assert_eq!(
            constructs("DATEDIFF(DAY, created_at, GETDATE())"),
            ["function DATEDIFF()", "function GETDATE()"]
        );
        assert_eq!(
            constructs("ISNULL([Name], 'x')"),
            ["function ISNULL()", "bracketed identifier [Name]"]
        );
        assert_eq!(constructs("\"Name\" = 'x'"), ["quoted identifier \"Name\""]);
        assert_eq!(constructs("a || b"), ["|| (write concat(…))"]);
        assert_eq!(constructs("total::int"), [":: cast"]);
        assert_eq!(constructs("metadata->>'category'"), ["JSON operator ->>"]);
        assert_eq!(constructs("a ; b"), ["';'"]);
    }

    #[test]
    fn plus_joining_text_is_named_and_plus_on_numbers_is_not() {
        assert_eq!(
            constructs("first_name + ' ' + last_name"),
            ["+ joining text (write concat(…))"]
        );
        assert_eq!(
            constructs("first_name + last_name"),
            ["+ joining text (write concat(…))"]
        );
        assert!(constructs("price + tax").is_empty());
    }

    #[test]
    fn keywords_before_a_parenthesis_are_not_functions() {
        assert!(constructs("x IN (1, 2)").is_empty());
        assert!(constructs("NOT (a = b)").is_empty());
    }
}
