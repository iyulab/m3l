//! Built-in lint rules.

pub mod model_size;
pub mod naming_convention;
pub mod relation_complexity;
pub mod similar_fields;

pub use model_size::ModelSizeRule;
pub use naming_convention::NamingConventionRule;
pub use relation_complexity::RelationComplexityRule;
pub use similar_fields::SimilarFieldsRule;
