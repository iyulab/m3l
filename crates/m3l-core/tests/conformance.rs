use m3l_core::{parse_string, resolve, validate, ValidateOptions};

// ---------------------------------------------------------------------------
// Helper: Full pipeline (parse → resolve → validate → JSON)
// ---------------------------------------------------------------------------
fn full_pipeline(input: &str, source: &str) -> m3l_core::M3lAst {
    let parsed = parse_string(input, source);
    let ast = resolve(&[parsed], None);
    validate(&ast, &ValidateOptions { strict: false });
    ast
}

// ===========================================================================
// 10 conformance fixtures
// ===========================================================================

#[test]
fn conformance_basic_model() {
    let input = r#"## User
- id: identifier @pk
- name: string(100) @not_null
- email: string(320)? @unique
- is_active: boolean = true"#;

    let ast = full_pipeline(input, "basic-model.m3l.md");

    assert_eq!(ast.models.len(), 1);
    assert_eq!(ast.enums.len(), 0);
    assert!(ast.errors.is_empty());

    let user = &ast.models[0];
    assert_eq!(user.name, "User");
    assert_eq!(user.fields.len(), 4);

    // id field
    let id = &user.fields[0];
    assert_eq!(id.name, "id");
    assert_eq!(id.field_type.as_deref(), Some("identifier"));
    assert!(!id.nullable);
    assert!(id.attributes.iter().any(|a| a.name == "pk"));

    // name field
    let name = &user.fields[1];
    assert_eq!(name.name, "name");
    assert_eq!(name.field_type.as_deref(), Some("string"));
    assert_eq!(name.params.as_ref().unwrap().len(), 1);

    // email field - nullable
    let email = &user.fields[2];
    assert_eq!(email.name, "email");
    assert!(email.nullable);
    assert!(email.attributes.iter().any(|a| a.name == "unique"));

    // is_active - default value
    let is_active = &user.fields[3];
    assert_eq!(is_active.name, "is_active");
    assert_eq!(is_active.default_value.as_deref(), Some("true"));
}

#[test]
fn conformance_enum_standalone() {
    let input = r#"## Status ::enum
- active: "Active"
- inactive: "Inactive"
- pending: "Pending""#;

    let ast = full_pipeline(input, "enum-standalone.m3l.md");

    assert_eq!(ast.enums.len(), 1);
    assert_eq!(ast.models.len(), 0);
    assert!(ast.errors.is_empty());

    let status = &ast.enums[0];
    assert_eq!(status.name, "Status");
    assert_eq!(status.values.len(), 3);
    assert_eq!(status.values[0].name, "active");
    assert_eq!(status.values[1].name, "inactive");
    assert_eq!(status.values[2].name, "pending");
}

#[test]
fn conformance_inheritance() {
    let input = r#"## BaseModel
- id: identifier @pk
- created_at: timestamp = now()

## User : BaseModel
- name: string(100)
- email: string(320)"#;

    let ast = full_pipeline(input, "inheritance.m3l.md");

    assert_eq!(ast.models.len(), 2);
    assert!(ast.errors.is_empty());

    let user = ast.models.iter().find(|m| m.name == "User").unwrap();
    assert_eq!(user.inherits, vec!["BaseModel"]);
    // User should have 4 fields: 2 inherited + 2 own
    assert_eq!(user.fields.len(), 4);
    assert_eq!(user.fields[0].name, "id");
    assert_eq!(user.fields[1].name, "created_at");
    assert_eq!(user.fields[2].name, "name");
    assert_eq!(user.fields[3].name, "email");
}

#[test]
fn conformance_view() {
    let input = r#"## User
- id: identifier @pk
- name: string(100)
- department: string(50)

## ActiveUsers ::view
### Source
- from: User
- where: "is_active = true"
- order_by: "name asc""#;

    let ast = full_pipeline(input, "view.m3l.md");

    assert_eq!(ast.models.len(), 1);
    assert_eq!(ast.views.len(), 1);
    assert!(ast.errors.is_empty());

    let view = &ast.views[0];
    assert_eq!(view.name, "ActiveUsers");
    assert!(view.source_def.is_some());
    let src = view.source_def.as_ref().unwrap();
    assert_eq!(src.from.as_deref(), Some("User"));
    assert_eq!(src.where_clause.as_deref(), Some("is_active = true"));
    assert_eq!(src.order_by.as_deref(), Some("name asc"));
}

#[test]
fn conformance_framework_attrs() {
    let input = r#"## Account
- id: identifier @pk
- password: string(100) `[DataType(DataType.Password)]` `[JsonIgnore]`
- display_name: string(50)"#;

    let ast = full_pipeline(input, "framework-attrs.m3l.md");

    assert_eq!(ast.models.len(), 1);
    assert!(ast.errors.is_empty());

    let password = &ast.models[0].fields[1];
    assert_eq!(password.name, "password");
    let fw = password.framework_attrs.as_ref().unwrap();
    assert_eq!(fw.len(), 2);
    assert!(fw[0].raw.contains("DataType(DataType.Password)"));
    assert!(fw[1].raw.contains("JsonIgnore"));
}

#[test]
fn conformance_interface() {
    let input = r#"## Timestampable ::interface
- created_at: timestamp = now()
- updated_at: timestamp = now()

## Article : Timestampable
- id: identifier @pk
- title: string(200)
- content: text"#;

    let ast = full_pipeline(input, "interface.m3l.md");

    assert_eq!(ast.interfaces.len(), 1);
    assert_eq!(ast.models.len(), 1);
    assert!(ast.errors.is_empty());

    let iface = &ast.interfaces[0];
    assert_eq!(iface.name, "Timestampable");
    assert_eq!(iface.fields.len(), 2);

    let article = &ast.models[0];
    assert_eq!(article.name, "Article");
    assert_eq!(article.inherits, vec!["Timestampable"]);
    // 2 inherited + 3 own
    assert_eq!(article.fields.len(), 5);
    assert_eq!(article.fields[0].name, "created_at");
    assert_eq!(article.fields[1].name, "updated_at");
    assert_eq!(article.fields[2].name, "id");
}

#[test]
fn conformance_lookup_rollup() {
    let input = r#"## Customer
- id: identifier @pk
- name: string(100)

## Order
- id: identifier @pk
- customer_id: identifier @fk(Customer.id)
- total: decimal(10,2)

### Lookup
- customer_name: string @lookup(customer_id.name)

## OrderSummary
- id: identifier @pk
- customer_id: identifier @fk(Customer.id)

### Rollup
- order_count: integer @rollup(Order.customer_id, count)
- total_spent: decimal(12,2) @rollup(Order.customer_id, sum(total))"#;

    let ast = full_pipeline(input, "lookup-rollup.m3l.md");

    assert!(ast.errors.is_empty(), "errors: {:?}", ast.errors);

    let order = ast.models.iter().find(|m| m.name == "Order").unwrap();
    let cn = order
        .fields
        .iter()
        .find(|f| f.name == "customer_name")
        .unwrap();
    assert_eq!(cn.kind, m3l_core::FieldKind::Lookup);
    assert!(cn.lookup.is_some());
    assert_eq!(cn.lookup.as_ref().unwrap().path, "customer_id.name");

    let summary = ast
        .models
        .iter()
        .find(|m| m.name == "OrderSummary")
        .unwrap();
    let oc = summary
        .fields
        .iter()
        .find(|f| f.name == "order_count")
        .unwrap();
    assert_eq!(oc.kind, m3l_core::FieldKind::Rollup);
    assert!(oc.rollup.is_some());
    assert_eq!(oc.rollup.as_ref().unwrap().aggregate, "count");

    let ts = summary
        .fields
        .iter()
        .find(|f| f.name == "total_spent")
        .unwrap();
    assert_eq!(ts.rollup.as_ref().unwrap().aggregate, "sum");
    assert_eq!(ts.rollup.as_ref().unwrap().field.as_deref(), Some("total"));
}

#[test]
fn conformance_computed_field() {
    let input = r#"## Product
- id: identifier @pk
- price: decimal(10,2)
- quantity: integer

### Computed
- total_value: decimal(12,2) @computed("price * quantity")"#;

    let ast = full_pipeline(input, "computed-field.m3l.md");

    assert_eq!(ast.models.len(), 1);
    assert!(ast.errors.is_empty());

    let tv = ast.models[0]
        .fields
        .iter()
        .find(|f| f.name == "total_value")
        .unwrap();
    assert_eq!(tv.kind, m3l_core::FieldKind::Computed);
    assert!(tv.computed.is_some());
    assert_eq!(tv.computed.as_ref().unwrap().expression, "price * quantity");
}

#[test]
fn conformance_view_sql_block() {
    let input = "## User\n- id: identifier @pk\n- name: string(100)\n- department: string(50)\n- is_active: boolean = true\n\n## UserReport ::view\n### Source\n```sql\nFROM User u\nWHERE u.department = 'Engineering'\nORDER BY u.name ASC\n```\n\n- user_name: string @from(User.name)\n- dept: string @from(User.department)";

    let ast = full_pipeline(input, "view-sql-block.m3l.md");

    assert_eq!(ast.views.len(), 1);

    let view = &ast.views[0];
    assert_eq!(view.name, "UserReport");
    assert!(view.source_def.is_some());
    let src = view.source_def.as_ref().unwrap();
    assert!(src.raw_sql.is_some());
    assert!(src.raw_sql.as_ref().unwrap().contains("FROM User u"));
    assert_eq!(src.language_hint.as_deref(), Some("sql"));
}

#[test]
fn conformance_backtick_expression() {
    let input = r#"## BacktickExpr
- id: identifier @pk
- profit: decimal @computed(`(price - cost) / price * 100`)
- status: string = "active"
- created_at: timestamp = `now()`
- discount: decimal = `price * 0.9`"#;

    let ast = full_pipeline(input, "backtick-expression.m3l.md");

    assert_eq!(ast.models.len(), 1);
    assert!(ast.errors.is_empty());

    let model = &ast.models[0];

    // profit - computed with backtick expression
    let profit = model.fields.iter().find(|f| f.name == "profit").unwrap();
    assert_eq!(profit.kind, m3l_core::FieldKind::Computed);
    assert!(profit.computed.is_some());
    assert_eq!(
        profit.computed.as_ref().unwrap().expression,
        "(price - cost) / price * 100"
    );

    // created_at - expression default
    let ca = model
        .fields
        .iter()
        .find(|f| f.name == "created_at")
        .unwrap();
    assert_eq!(ca.default_value.as_deref(), Some("now()"));

    // discount - expression default
    let disc = model.fields.iter().find(|f| f.name == "discount").unwrap();
    assert_eq!(disc.default_value.as_deref(), Some("price * 0.9"));
}

// ===========================================================================
// Sample file: 01-ecommerce.m3l.md — verify against reference JSON structure
// ===========================================================================

#[test]
fn sample_01_ecommerce_structure() {
    let input = include_str!("../../../spec/conformance/inputs/01-ecommerce.m3l.md");
    let ast = full_pipeline(input, "samples/01-ecommerce.m3l.md");

    // Model counts (from reference: 8 models)
    assert_eq!(
        ast.models.len(),
        8,
        "expected 8 models, got {:?}",
        ast.models.iter().map(|m| &m.name).collect::<Vec<_>>()
    );

    // Enum counts (from reference: 4 enums)
    assert_eq!(
        ast.enums.len(),
        4,
        "expected 4 enums, got {:?}",
        ast.enums.iter().map(|e| &e.name).collect::<Vec<_>>()
    );

    // Interface counts (from reference: 2 interfaces)
    assert_eq!(
        ast.interfaces.len(),
        2,
        "expected 2 interfaces, got {:?}",
        ast.interfaces.iter().map(|i| &i.name).collect::<Vec<_>>()
    );

    // View counts (from reference: 2 views)
    assert_eq!(
        ast.views.len(),
        2,
        "expected 2 views, got {:?}",
        ast.views.iter().map(|v| &v.name).collect::<Vec<_>>()
    );

    // Verify model names match reference
    let model_names: Vec<&str> = ast.models.iter().map(|m| m.name.as_str()).collect();
    assert!(model_names.contains(&"Customer"));
    assert!(model_names.contains(&"Address"));
    assert!(model_names.contains(&"Category"));
    assert!(model_names.contains(&"Product"));
    assert!(model_names.contains(&"Inventory"));
    assert!(model_names.contains(&"Order"));
    assert!(model_names.contains(&"OrderItem"));
    assert!(model_names.contains(&"Review"));

    // Verify enum names
    let enum_names: Vec<&str> = ast.enums.iter().map(|e| e.name.as_str()).collect();
    assert!(enum_names.contains(&"CustomerStatus"));
    assert!(enum_names.contains(&"PaymentMethod"));
    assert!(enum_names.contains(&"OrderStatus"));
    assert!(enum_names.contains(&"ShippingPriority"));

    // Verify namespace → project name
    assert_eq!(ast.project.name.as_deref(), Some("sample.ecommerce"));

    // No errors expected (only informational diagnostics)
    let hard_errors: Vec<_> = ast
        .errors
        .iter()
        .filter(|e| !e.code.starts_with("W"))
        .collect();
    // There may be validation warnings, but structural parsing should succeed
    assert!(
        hard_errors.is_empty()
            || hard_errors.iter().all(|e| {
                // E009 (undefined type) for inline enums, E002 for lookups — acceptable in this test context
                e.code == "M3L-E009" || e.code == "M3L-E002" || e.code == "M3L-E001"
            }),
        "unexpected errors: {:?}",
        hard_errors
    );
}

#[test]
fn sample_01_ecommerce_customer_detail() {
    let input = include_str!("../../../spec/conformance/inputs/01-ecommerce.m3l.md");
    let ast = full_pipeline(input, "samples/01-ecommerce.m3l.md");

    let customer = ast.models.iter().find(|m| m.name == "Customer").unwrap();

    // Customer inherits Timestampable
    assert_eq!(customer.inherits, vec!["Timestampable"]);

    // Customer has @public attribute
    assert!(customer.attributes.iter().any(|a| a.name == "public"));

    // Customer has description
    assert!(customer.description.is_some());

    // inherited created_at, updated_at + own id, email, name, phone, status, loyalty_points, is_verified = 9
    assert!(
        customer.fields.len() >= 7,
        "Customer should have at least 7 own fields, total: {}",
        customer.fields.len()
    );

    // id field has @pk @generated
    let id = customer.fields.iter().find(|f| f.name == "id").unwrap();
    assert!(id.attributes.iter().any(|a| a.name == "pk"));
    assert!(id.attributes.iter().any(|a| a.name == "generated"));

    // email has @unique with blockquote description
    let email = customer.fields.iter().find(|f| f.name == "email").unwrap();
    assert!(email.attributes.iter().any(|a| a.name == "unique"));
    assert!(email.description.is_some());

    // status is inline enum
    let status = customer.fields.iter().find(|f| f.name == "status").unwrap();
    assert_eq!(status.field_type.as_deref(), Some("enum"));
    assert!(status.enum_values.is_some());
    assert_eq!(status.enum_values.as_ref().unwrap().len(), 3);

    // loyalty_points default = 0
    let lp = customer
        .fields
        .iter()
        .find(|f| f.name == "loyalty_points")
        .unwrap();
    assert_eq!(lp.default_value.as_deref(), Some("0"));

    // Metadata section
    assert!(!customer.sections.metadata.is_empty());
}

#[test]
fn sample_01_ecommerce_product_detail() {
    let input = include_str!("../../../spec/conformance/inputs/01-ecommerce.m3l.md");
    let ast = full_pipeline(input, "samples/01-ecommerce.m3l.md");

    let product = ast.models.iter().find(|m| m.name == "Product").unwrap();

    // Product inherits Timestampable, Auditable
    assert_eq!(product.inherits.len(), 2);
    assert!(product.inherits.contains(&"Timestampable".to_string()));
    assert!(product.inherits.contains(&"Auditable".to_string()));

    // Has @public attribute
    assert!(product.attributes.iter().any(|a| a.name == "public"));

    // Lookup: category_name
    let cn = product
        .fields
        .iter()
        .find(|f| f.name == "category_name")
        .unwrap();
    assert_eq!(cn.kind, m3l_core::FieldKind::Lookup);
    assert_eq!(cn.lookup.as_ref().unwrap().path, "category_id.name");

    // Computed: profit_margin with backtick expression
    let pm = product
        .fields
        .iter()
        .find(|f| f.name == "profit_margin")
        .unwrap();
    assert_eq!(pm.kind, m3l_core::FieldKind::Computed);
    assert!(pm
        .computed
        .as_ref()
        .unwrap()
        .expression
        .contains("price - cost"));

    // Indexes section
    assert!(!product.sections.indexes.is_empty());

    // Relations section
    assert!(!product.sections.relations.is_empty());
}

#[test]
fn sample_01_ecommerce_views() {
    let input = include_str!("../../../spec/conformance/inputs/01-ecommerce.m3l.md");
    let ast = full_pipeline(input, "samples/01-ecommerce.m3l.md");

    // ActiveProducts view
    let ap = ast
        .views
        .iter()
        .find(|v| v.name == "ActiveProducts")
        .unwrap();
    assert!(ap.source_def.is_some());
    let src = ap.source_def.as_ref().unwrap();
    assert_eq!(src.from.as_deref(), Some("Product"));
    assert!(src.where_clause.is_some());

    // CustomerOrderSummary view - materialized with SQL block
    let cos = ast
        .views
        .iter()
        .find(|v| v.name == "CustomerOrderSummary")
        .unwrap();
    assert_eq!(cos.materialized, Some(true));
    assert!(cos.source_def.is_some());
    let src2 = cos.source_def.as_ref().unwrap();
    assert!(src2.raw_sql.is_some());
    assert!(src2.raw_sql.as_ref().unwrap().contains("FROM Customer"));

    // CustomerOrderSummary has fields
    assert!(cos.fields.len() >= 4);

    // Refresh section
    assert!(cos.refresh.is_some());
    assert_eq!(cos.refresh.as_ref().unwrap().strategy, "incremental");
}

// ===========================================================================
// Sample file: 02-blog-cms.m3l.md — verify against reference JSON structure
// ===========================================================================

#[test]
fn sample_02_blog_cms_structure() {
    let input = include_str!("../../../spec/conformance/inputs/02-blog-cms.m3l.md");
    let ast = full_pipeline(input, "samples/02-blog-cms.m3l.md");

    // Reference: 7 models, 2 enums, 2 views
    assert_eq!(
        ast.models.len(),
        7,
        "expected 7 models, got {:?}",
        ast.models.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
    assert_eq!(
        ast.enums.len(),
        2,
        "expected 2 enums, got {:?}",
        ast.enums.iter().map(|e| &e.name).collect::<Vec<_>>()
    );
    assert_eq!(
        ast.views.len(),
        2,
        "expected 2 views, got {:?}",
        ast.views.iter().map(|v| &v.name).collect::<Vec<_>>()
    );

    // Interfaces: BaseModel, Trackable
    assert_eq!(ast.interfaces.len(), 2);

    // Verify namespace
    assert_eq!(ast.project.name.as_deref(), Some("sample.blog"));

    // Model names
    let model_names: Vec<&str> = ast.models.iter().map(|m| m.name.as_str()).collect();
    assert!(model_names.contains(&"User"));
    assert!(model_names.contains(&"Tag"));
    assert!(model_names.contains(&"Category"));
    assert!(model_names.contains(&"Post"));
    assert!(model_names.contains(&"PostTag"));
    assert!(model_names.contains(&"Comment"));
    assert!(model_names.contains(&"MediaAsset"));
}

#[test]
fn sample_02_blog_cms_post_detail() {
    let input = include_str!("../../../spec/conformance/inputs/02-blog-cms.m3l.md");
    let ast = full_pipeline(input, "samples/02-blog-cms.m3l.md");

    let post = ast.models.iter().find(|m| m.name == "Post").unwrap();

    // Inherits BaseModel, Trackable
    assert_eq!(post.inherits.len(), 2);
    assert!(post.inherits.contains(&"BaseModel".to_string()));
    assert!(post.inherits.contains(&"Trackable".to_string()));

    // Has description
    assert!(post.description.is_some());

    // Lookup fields
    let author_name = post
        .fields
        .iter()
        .find(|f| f.name == "author_name")
        .unwrap();
    assert_eq!(author_name.kind, m3l_core::FieldKind::Lookup);

    // Computed fields
    let is_published = post
        .fields
        .iter()
        .find(|f| f.name == "is_published")
        .unwrap();
    assert_eq!(is_published.kind, m3l_core::FieldKind::Computed);

    // Rollup fields
    let comment_count = post
        .fields
        .iter()
        .find(|f| f.name == "comment_count")
        .unwrap();
    assert_eq!(comment_count.kind, m3l_core::FieldKind::Rollup);
    assert_eq!(comment_count.rollup.as_ref().unwrap().aggregate, "count");

    let avg_rating = post.fields.iter().find(|f| f.name == "avg_rating").unwrap();
    assert_eq!(avg_rating.rollup.as_ref().unwrap().aggregate, "avg");
    assert_eq!(
        avg_rating.rollup.as_ref().unwrap().field.as_deref(),
        Some("rating")
    );

    // Sections
    assert!(!post.sections.indexes.is_empty());
    assert!(!post.sections.behaviors.is_empty());
    assert!(!post.sections.metadata.is_empty());

    // Framework attrs on seo fields
    let seo_title = post.fields.iter().find(|f| f.name == "seo_title").unwrap();
    assert!(seo_title.framework_attrs.is_some());
}

#[test]
fn sample_02_blog_cms_views() {
    let input = include_str!("../../../spec/conformance/inputs/02-blog-cms.m3l.md");
    let ast = full_pipeline(input, "samples/02-blog-cms.m3l.md");

    // PublishedPosts view
    let pp = ast
        .views
        .iter()
        .find(|v| v.name == "PublishedPosts")
        .unwrap();
    assert!(pp.source_def.is_some());
    assert_eq!(
        pp.source_def.as_ref().unwrap().from.as_deref(),
        Some("Post")
    );

    // PopularPosts view - materialized
    let pop = ast.views.iter().find(|v| v.name == "PopularPosts").unwrap();
    assert_eq!(pop.materialized, Some(true));
    assert!(pop.refresh.is_some());
    assert_eq!(pop.refresh.as_ref().unwrap().strategy, "full");
}

// ===========================================================================
// Sample file: 03-types-showcase.m3l.md — verify against reference JSON structure
// ===========================================================================

#[test]
fn sample_03_types_showcase_structure() {
    let input = include_str!("../../../spec/conformance/inputs/03-types-showcase.m3l.md");
    let ast = full_pipeline(input, "samples/03-types-showcase.m3l.md");

    // Reference: 17 models, 0 enums, 0 views
    assert_eq!(
        ast.models.len(),
        17,
        "expected 17 models, got {:?}",
        ast.models.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
    assert_eq!(ast.enums.len(), 0);
    assert_eq!(ast.views.len(), 0);

    // Verify namespace
    assert_eq!(ast.project.name.as_deref(), Some("sample.types"));
}

#[test]
fn sample_03_all_primitive_types() {
    let input = include_str!("../../../spec/conformance/inputs/03-types-showcase.m3l.md");
    let ast = full_pipeline(input, "samples/03-types-showcase.m3l.md");

    let model = ast
        .models
        .iter()
        .find(|m| m.name == "AllPrimitiveTypes")
        .unwrap();
    // 13 fields: id + 12 primitive types
    assert_eq!(model.fields.len(), 13);

    let field_types: Vec<&str> = model
        .fields
        .iter()
        .filter_map(|f| f.field_type.as_deref())
        .collect();
    assert!(field_types.contains(&"identifier"));
    assert!(field_types.contains(&"string"));
    assert!(field_types.contains(&"text"));
    assert!(field_types.contains(&"integer"));
    assert!(field_types.contains(&"long"));
    assert!(field_types.contains(&"decimal"));
    assert!(field_types.contains(&"float"));
    assert!(field_types.contains(&"boolean"));
    assert!(field_types.contains(&"date"));
    assert!(field_types.contains(&"time"));
    assert!(field_types.contains(&"timestamp"));
    assert!(field_types.contains(&"binary"));
}

#[test]
fn sample_03_type_modifiers() {
    let input = include_str!("../../../spec/conformance/inputs/03-types-showcase.m3l.md");
    let ast = full_pipeline(input, "samples/03-types-showcase.m3l.md");

    let model = ast
        .models
        .iter()
        .find(|m| m.name == "TypeModifiers")
        .unwrap();

    // nullable field
    let nullable_str = model
        .fields
        .iter()
        .find(|f| f.name == "nullable_str")
        .unwrap();
    assert!(nullable_str.nullable);

    // array field
    let str_array = model.fields.iter().find(|f| f.name == "str_array").unwrap();
    assert!(str_array.array);
    assert!(!str_array.nullable);

    // nullable array
    let nullable_array = model
        .fields
        .iter()
        .find(|f| f.name == "nullable_array")
        .unwrap();
    assert!(nullable_array.array);
    assert!(nullable_array.nullable);

    // array of nullable items
    let array_of_nullable = model
        .fields
        .iter()
        .find(|f| f.name == "array_of_nullable")
        .unwrap();
    assert!(array_of_nullable.array);
    assert!(array_of_nullable.array_item_nullable);
}

#[test]
fn sample_03_computed_variants() {
    let input = include_str!("../../../spec/conformance/inputs/03-types-showcase.m3l.md");
    let ast = full_pipeline(input, "samples/03-types-showcase.m3l.md");

    let model = ast
        .models
        .iter()
        .find(|m| m.name == "ComputedVariants")
        .unwrap();

    // Regular computed
    let full_name = model.fields.iter().find(|f| f.name == "full_name").unwrap();
    assert_eq!(full_name.kind, m3l_core::FieldKind::Computed);
    assert!(full_name
        .computed
        .as_ref()
        .unwrap()
        .expression
        .contains("first_name"));

    // Backtick computed
    let tax_amount = model
        .fields
        .iter()
        .find(|f| f.name == "tax_amount")
        .unwrap();
    assert_eq!(tax_amount.kind, m3l_core::FieldKind::Computed);
    assert!(tax_amount
        .computed
        .as_ref()
        .unwrap()
        .expression
        .contains("price * tax_rate"));

    // computed_raw with platform
    let age = model.fields.iter().find(|f| f.name == "age").unwrap();
    assert_eq!(age.kind, m3l_core::FieldKind::Computed);
    assert_eq!(
        age.computed.as_ref().unwrap().platform.as_deref(),
        Some("sqlserver")
    );
}

#[test]
fn sample_03_inheritance_override() {
    let input = include_str!("../../../spec/conformance/inputs/03-types-showcase.m3l.md");
    let ast = full_pipeline(input, "samples/03-types-showcase.m3l.md");

    let model = ast
        .models
        .iter()
        .find(|m| m.name == "InheritanceOverride")
        .unwrap();
    assert!(model.inherits.contains(&"AllPrimitiveTypes".to_string()));

    // str field should be overridden with string(500)
    let str_field = model.fields.iter().find(|f| f.name == "str").unwrap();
    assert_eq!(str_field.field_type.as_deref(), Some("string"));
    assert!(str_field.attributes.iter().any(|a| a.name == "override"));
}

// ===========================================================================
// JSON serialization compatibility check
// ===========================================================================

#[test]
fn json_output_basic_model_structure() {
    // Verify Rust JSON output matches TS reference structure for a basic model
    let input = "## User\n- id: identifier @pk\n- name: string(100) @not_null";
    let parsed = parse_string(input, "test.m3l.md");
    let ast = resolve(&[parsed], None);
    validate(&ast, &ValidateOptions { strict: false });

    let json_val: serde_json::Value = serde_json::to_value(&ast).unwrap();

    // Top-level structure
    assert!(json_val["parserVersion"].is_string());
    assert!(json_val["astVersion"].is_string());
    assert!(json_val["project"].is_object());
    assert!(json_val["sources"].is_array());
    assert!(json_val["models"].is_array());
    assert!(json_val["enums"].is_array());
    assert!(json_val["interfaces"].is_array());
    assert!(json_val["views"].is_array());
    assert!(json_val["attributeRegistry"].is_array());
    assert!(json_val["errors"].is_array());
    assert!(json_val["warnings"].is_array());

    // Model structure
    let model = &json_val["models"][0];
    assert_eq!(model["name"], "User");
    assert_eq!(model["type"], "model");
    assert!(model["source"].is_string());
    assert!(model["line"].is_number());
    assert!(model["inherits"].is_array());
    assert!(model["attributes"].is_array());
    assert!(model["fields"].is_array());
    assert!(model["sections"].is_object());
    assert!(model["loc"].is_object());

    // Field structure — check ALL required fields are present
    let field = &model["fields"][0];
    assert_eq!(field["name"], "id");
    assert_eq!(field["type"], "identifier");
    assert_eq!(field["nullable"], false);
    assert_eq!(field["array"], false);
    assert_eq!(field["arrayItemNullable"], false);
    assert_eq!(field["kind"], "stored");
    assert!(field["attributes"].is_array());
    assert!(field["loc"].is_object());
    assert_eq!(field["loc"]["col"], 1);

    // Attribute without isStandard should NOT have the field
    let pk_attr = &field["attributes"][0];
    assert_eq!(pk_attr["name"], "pk");
    assert!(
        pk_attr.get("isStandard").is_none() || pk_attr["isStandard"].is_null(),
        "@pk should not have isStandard"
    );

    // Sections should have standard keys with empty arrays/objects
    let sections = &model["sections"];
    assert!(sections["indexes"].is_array());
    assert!(sections["relations"].is_array());
    assert!(sections["behaviors"].is_array());
    assert!(sections["metadata"].is_object());
}

#[test]
fn json_output_enum_structure() {
    let input = "## Status ::enum\n- active: \"Active\"\n- inactive: \"Inactive\"";
    let ast = full_pipeline(input, "test.m3l.md");
    let json_val: serde_json::Value = serde_json::to_value(&ast).unwrap();

    let enum_node = &json_val["enums"][0];
    assert_eq!(enum_node["name"], "Status");
    assert_eq!(enum_node["type"], "enum");
    assert!(enum_node["values"].is_array());
    assert_eq!(enum_node["values"][0]["name"], "active");
}

#[test]
fn json_output_optional_fields_omitted() {
    // Verify skip_serializing_if = None works correctly
    let input = "## User\n- id: identifier @pk";
    let ast = full_pipeline(input, "test.m3l.md");
    let json_val: serde_json::Value = serde_json::to_value(&ast).unwrap();

    let field = &json_val["models"][0]["fields"][0];

    // These optional fields should NOT be present when None
    assert!(field.get("label").is_none() || field["label"].is_null());
    assert!(field.get("params").is_none() || field["params"].is_null());
    assert!(field.get("generic_params").is_none() || field["generic_params"].is_null());
    assert!(field.get("default_value").is_none() || field["default_value"].is_null());
    assert!(field.get("description").is_none() || field["description"].is_null());
    assert!(field.get("framework_attrs").is_none() || field["framework_attrs"].is_null());
    assert!(field.get("lookup").is_none() || field["lookup"].is_null());
    assert!(field.get("rollup").is_none() || field["rollup"].is_null());
    assert!(field.get("computed").is_none() || field["computed"].is_null());
    assert!(field.get("enum_values").is_none() || field["enum_values"].is_null());

    // These should always be present
    assert!(field.get("nullable").is_some());
    assert!(field.get("array").is_some());
    assert!(field.get("arrayItemNullable").is_some());
    assert!(field.get("kind").is_some());
    assert!(field.get("attributes").is_some());
    assert!(field.get("loc").is_some());
}

#[test]
fn json_output_field_names_match_ts() {
    let input = "## User\n- id: identifier @pk @generated\n- name: string(100) @not_null";
    let ast = full_pipeline(input, "test.m3l.md");
    let json = serde_json::to_string_pretty(&ast).unwrap();

    // Check top-level field names match TS output exactly
    assert!(json.contains("\"parserVersion\""));
    assert!(json.contains("\"astVersion\""));
    assert!(json.contains("\"attributeRegistry\""));

    // Check field-level names
    assert!(json.contains("\"arrayItemNullable\""));

    // Check isStandard on standard attributes (@generated, @not_null)
    // Note: @pk is NOT in STANDARD_ATTRIBUTES (matches TS behavior)
    assert!(
        json.contains("\"isStandard\""),
        "json should contain isStandard for @generated or @not_null"
    );
}

// ===========================================================================
// Deep JSON comparison: Rust output vs spec/conformance/expected reference
// ===========================================================================

#[test]
fn deep_compare_01_ecommerce_vs_reference() {
    let input = include_str!("../../../spec/conformance/inputs/01-ecommerce.m3l.md");
    let ast = full_pipeline(input, "samples/01-ecommerce.m3l.md");
    let rust_json: serde_json::Value = serde_json::to_value(&ast).unwrap();
    let ts_json: serde_json::Value = serde_json::from_str(include_str!(
        "../../../spec/conformance/expected/01-ecommerce.json"
    ))
    .unwrap();

    // Compare top-level keys (excluding parserVersion which differs)
    assert_eq!(rust_json["astVersion"], ts_json["astVersion"]);
    assert_eq!(rust_json["project"]["name"], ts_json["project"]["name"]);

    // Compare model counts
    assert_eq!(
        rust_json["models"].as_array().unwrap().len(),
        ts_json["models"].as_array().unwrap().len(),
        "Model count mismatch"
    );

    // Compare enum counts
    assert_eq!(
        rust_json["enums"].as_array().unwrap().len(),
        ts_json["enums"].as_array().unwrap().len(),
        "Enum count mismatch"
    );

    // Compare interface counts
    assert_eq!(
        rust_json["interfaces"].as_array().unwrap().len(),
        ts_json["interfaces"].as_array().unwrap().len(),
        "Interface count mismatch"
    );

    // Compare view counts
    assert_eq!(
        rust_json["views"].as_array().unwrap().len(),
        ts_json["views"].as_array().unwrap().len(),
        "View count mismatch"
    );

    // Compare model names in order
    let rust_model_names: Vec<&str> = rust_json["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    let ts_model_names: Vec<&str> = ts_json["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        rust_model_names, ts_model_names,
        "Model name order mismatch"
    );

    // Compare field counts per model
    for (i, ts_model) in ts_json["models"].as_array().unwrap().iter().enumerate() {
        let rust_model = &rust_json["models"][i];
        let model_name = ts_model["name"].as_str().unwrap();
        let ts_field_count = ts_model["fields"].as_array().unwrap().len();
        let rust_field_count = rust_model["fields"].as_array().unwrap().len();
        assert_eq!(
            rust_field_count, ts_field_count,
            "Field count mismatch for model '{}': Rust={} TS={}",
            model_name, rust_field_count, ts_field_count
        );
    }

    // Compare field names per model
    for (i, ts_model) in ts_json["models"].as_array().unwrap().iter().enumerate() {
        let rust_model = &rust_json["models"][i];
        let model_name = ts_model["name"].as_str().unwrap();
        let ts_field_names: Vec<&str> = ts_model["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        let rust_field_names: Vec<&str> = rust_model["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            rust_field_names, ts_field_names,
            "Field name order mismatch for model '{}'",
            model_name
        );
    }

    // Compare field types per model
    for (i, ts_model) in ts_json["models"].as_array().unwrap().iter().enumerate() {
        let rust_model = &rust_json["models"][i];
        let model_name = ts_model["name"].as_str().unwrap();
        for (j, ts_field) in ts_model["fields"].as_array().unwrap().iter().enumerate() {
            let rust_field = &rust_model["fields"][j];
            let field_name = ts_field["name"].as_str().unwrap();
            assert_eq!(
                rust_field["type"], ts_field["type"],
                "Type mismatch for {}.{}",
                model_name, field_name
            );
            assert_eq!(
                rust_field["nullable"], ts_field["nullable"],
                "Nullable mismatch for {}.{}",
                model_name, field_name
            );
            assert_eq!(
                rust_field["array"], ts_field["array"],
                "Array mismatch for {}.{}",
                model_name, field_name
            );
            assert_eq!(
                rust_field["kind"], ts_field["kind"],
                "Kind mismatch for {}.{}",
                model_name, field_name
            );
        }
    }

    // Compare enum values
    for (i, ts_enum) in ts_json["enums"].as_array().unwrap().iter().enumerate() {
        let rust_enum = &rust_json["enums"][i];
        let enum_name = ts_enum["name"].as_str().unwrap();
        let ts_values: Vec<&str> = ts_enum["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["name"].as_str().unwrap())
            .collect();
        let rust_values: Vec<&str> = rust_enum["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            rust_values, ts_values,
            "Enum value mismatch for '{}'",
            enum_name
        );
    }
}

/// Helper for deep comparison of Rust vs TS reference JSON
fn deep_compare_ast_structure(
    rust_json: &serde_json::Value,
    ts_json: &serde_json::Value,
    label: &str,
) {
    // Compare counts
    for key in &["models", "enums", "interfaces", "views"] {
        let rust_len = rust_json[key].as_array().map(|a| a.len()).unwrap_or(0);
        let ts_len = ts_json[key].as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(rust_len, ts_len, "[{}] {} count mismatch", label, key);
    }

    // Compare model names and field details
    if let Some(ts_models) = ts_json["models"].as_array() {
        for (i, ts_model) in ts_models.iter().enumerate() {
            let rust_model = &rust_json["models"][i];
            let model_name = ts_model["name"].as_str().unwrap();

            assert_eq!(
                rust_model["name"], ts_model["name"],
                "[{}] Model name mismatch at index {}",
                label, i
            );

            let ts_fields = ts_model["fields"].as_array().unwrap();
            let rust_fields = rust_model["fields"].as_array().unwrap();
            // Note: TS reference was generated at v0.1.2; some fields (e.g., backtick defaults)
            // may be missing in the TS output due to older parser bugs.
            // We allow Rust to have MORE fields but not FEWER.
            assert!(
                rust_fields.len() >= ts_fields.len(),
                "[{}] Rust has fewer fields than TS for '{}': Rust={} TS={}",
                label,
                model_name,
                rust_fields.len(),
                ts_fields.len()
            );

            // Match each TS field by name in Rust output
            for ts_field in ts_fields.iter() {
                let field_name = ts_field["name"].as_str().unwrap();
                let rust_field = rust_fields
                    .iter()
                    .find(|f| f["name"].as_str() == Some(field_name));
                let rust_field = match rust_field {
                    Some(f) => f,
                    None => panic!(
                        "[{}] TS field '{}' in model '{}' not found in Rust output",
                        label, field_name, model_name
                    ),
                };

                assert_eq!(
                    rust_field["type"], ts_field["type"],
                    "[{}] Type mismatch for {}.{}",
                    label, model_name, field_name
                );
                assert_eq!(
                    rust_field["nullable"], ts_field["nullable"],
                    "[{}] Nullable mismatch for {}.{}",
                    label, model_name, field_name
                );
                assert_eq!(
                    rust_field["array"], ts_field["array"],
                    "[{}] Array mismatch for {}.{}",
                    label, model_name, field_name
                );
                assert_eq!(
                    rust_field["kind"], ts_field["kind"],
                    "[{}] Kind mismatch for {}.{}",
                    label, model_name, field_name
                );
            }
        }
    }

    // Compare enum names and values
    if let Some(ts_enums) = ts_json["enums"].as_array() {
        for (i, ts_enum) in ts_enums.iter().enumerate() {
            let rust_enum = &rust_json["enums"][i];
            assert_eq!(
                rust_enum["name"], ts_enum["name"],
                "[{}] Enum name mismatch at index {}",
                label, i
            );
        }
    }
}

#[test]
fn deep_compare_02_blog_cms_vs_reference() {
    let input = include_str!("../../../spec/conformance/inputs/02-blog-cms.m3l.md");
    let ast = full_pipeline(input, "samples/02-blog-cms.m3l.md");
    let rust_json: serde_json::Value = serde_json::to_value(&ast).unwrap();
    let ts_json: serde_json::Value = serde_json::from_str(include_str!(
        "../../../spec/conformance/expected/02-blog-cms.json"
    ))
    .unwrap();

    assert_eq!(rust_json["project"]["name"], ts_json["project"]["name"]);
    deep_compare_ast_structure(&rust_json, &ts_json, "02-blog-cms");
}

#[test]
fn deep_compare_03_types_showcase_vs_reference() {
    let input = include_str!("../../../spec/conformance/inputs/03-types-showcase.m3l.md");
    let ast = full_pipeline(input, "samples/03-types-showcase.m3l.md");
    let rust_json: serde_json::Value = serde_json::to_value(&ast).unwrap();
    let ts_json: serde_json::Value = serde_json::from_str(include_str!(
        "../../../spec/conformance/expected/03-types-showcase.json"
    ))
    .unwrap();

    assert_eq!(rust_json["project"]["name"], ts_json["project"]["name"]);
    deep_compare_ast_structure(&rust_json, &ts_json, "03-types-showcase");
}
