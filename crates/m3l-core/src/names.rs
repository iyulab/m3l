//! Finding the declaration a written name refers to (§2.1.3). One rule set for every place a name is
//! read — an inheritance parent, a `@reference`, a lookup hop, a view's `from` — so that two of them
//! never resolve the same text to different declarations.

use std::collections::HashMap;

/// The aliases files give their imports (`@import "inventory/models.m3l" as inventory`, §10.6.2) — for each
/// importing file, alias → the file the import names. An alias is the importing file's own: `inventory.Product`
/// written there means the `Product` declared in that file, whatever its namespace.
#[derive(Debug, Default)]
pub(crate) struct Aliases {
    by_file: HashMap<String, HashMap<String, String>>,
}

impl Aliases {
    pub(crate) fn add(&mut self, importer: &str, alias: &str, target: &str) {
        self.by_file
            .entry(importer.to_string())
            .or_default()
            .insert(alias.to_string(), target.to_string());
    }

    /// The aliases an AST's imports give — each alias whose import names a file read (the resolver's resolution).
    /// An alias the resolver refused (`M3L-E034`, given twice) keeps the first import's file, as the resolver does.
    pub(crate) fn of_imports(imports: &[crate::types::ImportNode]) -> Self {
        let mut aliases = Self::default();
        let mut seen = std::collections::HashSet::new();
        for import in imports {
            let Some(alias) = &import.alias else { continue };
            if !seen.insert((import.file.as_str(), alias.as_str())) {
                continue;
            }
            if let Some(target) = &import.target {
                aliases.add(&import.file, alias, target);
            }
        }
        aliases
    }

    /// The file `alias` names in `importer`, if `importer` declares that alias.
    pub(crate) fn target(&self, importer: &str, alias: &str) -> Option<&str> {
        self.by_file
            .get(importer)
            .and_then(|aliases| aliases.get(alias))
            .map(String::as_str)
    }
}

/// Where a name is written — the referring declaration's namespace and file, and the aliases that file declares.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Referrer<'r> {
    pub ns: Option<&'r str>,
    pub source: &'r str,
    pub aliases: &'r Aliases,
}

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
/// is `short_name(written)`. A name qualified by an alias the referring file declares (`inventory.Product`)
/// names the declaration in the file that import names (Rule 3); any other qualified name
/// (`domain.inventory.Product`) names its namespace. A short name is the only candidate, else the only
/// one in the referring namespace, else the only one in the referring file (Rules 2 and 3); more than
/// one left is ambiguous (Rule 4).
pub(crate) fn pick<'a, T>(
    candidates: &'a [T],
    written: &str,
    from: Referrer<'_>,
    namespace: impl Fn(&T) -> Option<&str>,
    source: impl Fn(&T) -> &str,
) -> Lookup<'a, T> {
    let (from_ns, from_source) = (from.ns, from.source);
    if let Some((qualifier, _)) = written.rsplit_once('.') {
        // An alias is the referring file's own binding, so it is read before a namespace of the same text.
        let found = match from.aliases.target(from_source, qualifier) {
            Some(file) => candidates.iter().find(|c| source(c) == file),
            None => candidates.iter().find(|c| namespace(c) == Some(qualifier)),
        };
        return found.map_or(Lookup::Missing, Lookup::Found);
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

    fn find_with<'a>(
        all: &'a [D],
        written: &str,
        ns: Option<&str>,
        file: &str,
        aliases: &Aliases,
    ) -> Option<&'a str> {
        let from = Referrer {
            ns,
            source: file,
            aliases,
        };
        match pick(all, written, from, |d| d.1, |d| d.2) {
            Lookup::Found(d) => Some(d.0),
            _ => None,
        }
    }

    fn find<'a>(all: &'a [D], written: &str, ns: Option<&str>, file: &str) -> Option<&'a str> {
        find_with(all, written, ns, file, &Aliases::default())
    }

    #[test]
    fn a_short_name_narrows_by_namespace_then_file() {
        let all = [D("a", Some("x"), "1.md"), D("b", Some("y"), "2.md")];
        assert_eq!(find(&all, "P", Some("y"), "9.md"), Some("b"));
        assert_eq!(find(&all, "P", Some("z"), "1.md"), Some("a"));
        let none = Aliases::default();
        let from = Referrer {
            ns: Some("z"),
            source: "9.md",
            aliases: &none,
        };
        assert!(matches!(
            pick(&all, "P", from, |d| d.1, |d| d.2),
            Lookup::Ambiguous(ns) if ns == ["x", "y"]
        ));
    }

    #[test]
    fn an_alias_names_the_imported_file_before_a_namespace_of_that_text() {
        // `x` is both an alias in 9.md (for 2.md) and a namespace (of 1.md's declaration).
        let all = [D("a", Some("x"), "1.md"), D("b", Some("y"), "2.md")];
        let mut aliases = Aliases::default();
        aliases.add("9.md", "x", "2.md");
        assert_eq!(find_with(&all, "x.P", None, "9.md", &aliases), Some("b"));
        // The alias is 9.md's own — another file reads `x` as the namespace.
        assert_eq!(find_with(&all, "x.P", None, "8.md", &aliases), Some("a"));
        // An alias for a file that declares no `P` names nothing.
        aliases.add("9.md", "q", "3.md");
        assert_eq!(find_with(&all, "q.P", None, "9.md", &aliases), None);
    }

    #[test]
    fn a_qualified_name_names_its_namespace() {
        let all = [D("a", Some("x.y"), "1.md"), D("b", Some("z"), "2.md")];
        assert_eq!(find(&all, "x.y.P", None, "9.md"), Some("a"));
        assert_eq!(find(&all, "q.P", None, "9.md"), None);
        assert_eq!(short_name("x.y.P"), "P");
    }
}
