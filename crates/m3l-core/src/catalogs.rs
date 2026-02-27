use std::collections::HashSet;
use std::sync::LazyLock;

/// Official M3L type catalog (from specification §10.4).
/// Types not in this set are treated as model/enum/interface references.
pub static TYPE_CATALOG: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    // Primitive types (§10.4.1)
    s.insert("string");
    s.insert("text");
    s.insert("integer");
    s.insert("long");
    s.insert("decimal");
    s.insert("float");
    s.insert("boolean");
    s.insert("date");
    s.insert("time");
    s.insert("timestamp");
    s.insert("identifier");
    s.insert("binary");
    // Semantic types / shorthands (§10.4.2)
    s.insert("email");
    s.insert("phone");
    s.insert("url");
    s.insert("money");
    s.insert("percentage");
    // Structural types (§10.4.3)
    s.insert("object");
    s.insert("json");
    s.insert("enum");
    s.insert("map");
    // Deprecated (§10.4.5) — still accepted
    s.insert("datetime");
    s
});

/// Standard M3L attribute catalog.
/// These are the officially defined attributes in the M3L specification.
pub static STANDARD_ATTRIBUTES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    // Field constraints
    s.insert("primary");
    s.insert("unique");
    s.insert("required");
    s.insert("index");
    s.insert("generated");
    s.insert("immutable");
    // References / relations
    s.insert("reference");
    s.insert("fk");
    s.insert("relation");
    s.insert("on_update");
    s.insert("on_delete");
    // Search / display
    s.insert("searchable");
    s.insert("description");
    s.insert("visibility");
    // Validation
    s.insert("min");
    s.insert("max");
    s.insert("validate");
    s.insert("not_null");
    // Derived fields
    s.insert("computed");
    s.insert("computed_raw");
    s.insert("lookup");
    s.insert("rollup");
    s.insert("from");
    s.insert("persisted");
    // Model-level
    s.insert("public");
    s.insert("private");
    s.insert("materialized");
    s.insert("meta");
    s.insert("behavior");
    s.insert("override");
    s.insert("default_attribute");
    s
});

/// Section names that change the current field kind.
pub static KIND_SECTIONS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("Lookup");
    s.insert("Rollup");
    s.insert("Computed");
    s.insert("Computed from Rollup");
    s
});

/// Parser and AST version constants.
pub const PARSER_VERSION: &str = "0.4.0";
pub const AST_VERSION: &str = "1.0";
