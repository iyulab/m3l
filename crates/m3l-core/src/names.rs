//! Finding the declaration a written name refers to (§2.1.3). One rule set for every place a name is
//! read — an inheritance parent, a `@reference`, a lookup hop, a view's `from` — so that two of them
//! never resolve the same text to different declarations.

/// What a written name refers to among the candidates sharing its short name.
#[derive(Debug)]
pub(crate) enum Lookup<'a, T> {
    Found(&'a T),
    /// More than one candidate fits a short name, and neither the referring namespace nor the
    /// referring file picks one (Rule 4) — their namespaces, sorted.
    Ambiguous(Vec<String>),
    Missing,
}

/// The short name a reference writes — the last part of a qualified name.
pub(crate) fn short_name(written: &str) -> &str {
    written.rsplit_once('.').map_or(written, |(_, short)| short)
}

/// Picks the declaration `written` refers to from `candidates` — every declaration whose short name
/// is `short_name(written)`. A qualified name (`domain.inventory.Product`) names its namespace. A
/// short name is the only candidate, else the only one in the referring namespace, else the only one
/// in the referring file (Rules 2 and 3); more than one left is ambiguous (Rule 4).
pub(crate) fn pick<'a, T>(
    candidates: &'a [T],
    written: &str,
    from_ns: Option<&str>,
    from_source: &str,
    namespace: impl Fn(&T) -> Option<&str>,
    source: impl Fn(&T) -> &str,
) -> Lookup<'a, T> {
    if let Some((ns, _)) = written.rsplit_once('.') {
        return match candidates.iter().find(|c| namespace(c) == Some(ns)) {
            Some(c) => Lookup::Found(c),
            None => Lookup::Missing,
        };
    }
    match candidates {
        [] => return Lookup::Missing,
        [only] => return Lookup::Found(only),
        _ => {}
    }
    let in_namespace: Vec<&T> = candidates
        .iter()
        .filter(|c| namespace(c) == from_ns)
        .collect();
    if let [only] = in_namespace.as_slice() {
        return Lookup::Found(only);
    }
    let in_file: Vec<&T> = candidates
        .iter()
        .filter(|c| source(c) == from_source)
        .collect();
    if let [only] = in_file.as_slice() {
        return Lookup::Found(only);
    }
    let mut namespaces: Vec<String> = candidates
        .iter()
        .map(|c| namespace(c).unwrap_or("(none)").to_string())
        .collect();
    namespaces.sort();
    namespaces.dedup();
    Lookup::Ambiguous(namespaces)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct D(&'static str, Option<&'static str>, &'static str);

    fn find<'a>(all: &'a [D], written: &str, ns: Option<&str>, file: &str) -> Option<&'a str> {
        match pick(all, written, ns, file, |d| d.1, |d| d.2) {
            Lookup::Found(d) => Some(d.0),
            _ => None,
        }
    }

    #[test]
    fn a_short_name_narrows_by_namespace_then_file() {
        let all = [D("a", Some("x"), "1.md"), D("b", Some("y"), "2.md")];
        assert_eq!(find(&all, "P", Some("y"), "9.md"), Some("b"));
        assert_eq!(find(&all, "P", Some("z"), "1.md"), Some("a"));
        assert!(matches!(
            pick(&all, "P", Some("z"), "9.md", |d| d.1, |d| d.2),
            Lookup::Ambiguous(ns) if ns == ["x", "y"]
        ));
    }

    #[test]
    fn a_qualified_name_names_its_namespace() {
        let all = [D("a", Some("x.y"), "1.md"), D("b", Some("z"), "2.md")];
        assert_eq!(find(&all, "x.y.P", None, "9.md"), Some("a"));
        assert_eq!(find(&all, "q.P", None, "9.md"), None);
        assert_eq!(short_name("x.y.P"), "P");
    }
}
