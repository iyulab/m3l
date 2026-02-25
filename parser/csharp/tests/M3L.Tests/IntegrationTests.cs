using M3L;
using M3L.Models;

namespace M3L.Tests;

public class IntegrationTests
{
    [Fact]
    public void M3LParser_ParseString_BasicModel()
    {
        var parser = new M3LParser();
        var ast = parser.ParseString("## User\n- id: identifier @primary\n- name: string(100)");

        Assert.Single(ast.Models);
        Assert.Equal("User", ast.Models[0].Name);
        Assert.Equal(2, ast.Models[0].Fields.Count);
        Assert.Equal(Resolver.ParserVersion, ast.ParserVersion);
        Assert.Equal(Resolver.AstVersion, ast.AstVersion);
    }

    [Fact]
    public void M3LParser_ParseString_FullScenario()
    {
        var content = @"# Namespace: ecommerce

## Timestampable::interface
- created_at: datetime
- updated_at: datetime

## Status::enum
- Active ""Active status""
- Inactive ""Inactive status""

## Product(상품) : Timestampable
> Product catalog item

- id: identifier @primary @generated
- name: string(200) @searchable
- price: decimal(10,2) @min(0)
- status: string = ""active""
- tags: string[]
- description: text?

### Indexes
- idx_name

### Metadata
- table_name: products
";

        var parser = new M3LParser();
        var ast = parser.ParseString(content);

        // Namespace
        Assert.Equal("ecommerce", ast.Project.Name);

        // Interface
        Assert.Single(ast.Interfaces);
        Assert.Equal("Timestampable", ast.Interfaces[0].Name);

        // Enum
        Assert.Single(ast.Enums);
        Assert.Equal("Status", ast.Enums[0].Name);
        Assert.Equal(2, ast.Enums[0].Values.Count);

        // Model with inheritance
        var product = ast.Models.First(m => m.Name == "Product");
        Assert.Equal("상품", product.Label);
        Assert.Contains("Timestampable", product.Inherits);
        Assert.Equal("Product catalog item", product.Description);

        // Inherited fields should be at the start
        Assert.Equal("created_at", product.Fields[0].Name);
        Assert.Equal("updated_at", product.Fields[1].Name);

        // Own fields
        var idField = product.Fields.First(f => f.Name == "id");
        Assert.Contains(idField.Attributes, a => a.Name == "primary");
        Assert.Contains(idField.Attributes, a => a.Name == "generated");

        var nameField = product.Fields.First(f => f.Name == "name");
        Assert.Equal("string", nameField.Type);

        var priceField = product.Fields.First(f => f.Name == "price");
        Assert.Equal("decimal", priceField.Type);

        var statusField = product.Fields.First(f => f.Name == "status");
        Assert.Equal("\"active\"", statusField.DefaultValue);

        var tagsField = product.Fields.First(f => f.Name == "tags");
        Assert.True(tagsField.Array);

        var descField = product.Fields.First(f => f.Name == "description");
        Assert.True(descField.Nullable);

        // Sections
        Assert.Single(product.Sections.Indexes);
        Assert.Equal("products", product.Sections.Metadata["table_name"]);
    }

    [Fact]
    public void M3LParser_ParseString_ViewWithJoinsAndRefresh()
    {
        var content = @"## Order
- id: identifier @primary
- customer_id: identifier @reference(Customer)
- total: decimal(10,2)

## Customer
- id: identifier @primary
- name: string(100)

## SalesReport::view @materialized

### Source
- from: Order
- join: Customer on Order.customer_id = Customer.id
- where: ""total > 0""

- customer_name: string
- order_total: decimal

### Refresh
- strategy: incremental
- interval: ""1h""
";
        var parser = new M3LParser();
        var ast = parser.ParseString(content);

        var view = ast.Views.First(v => v.Name == "SalesReport");
        Assert.True(view.Materialized);
        Assert.NotNull(view.SourceDef);
        Assert.Equal("Order", view.SourceDef!.From);
        Assert.NotNull(view.SourceDef.Joins);
        Assert.Single(view.SourceDef.Joins);
        Assert.Equal("Customer", view.SourceDef.Joins[0].Model);
        Assert.Equal("total > 0", view.SourceDef.Where);

        Assert.NotNull(view.Refresh);
        Assert.Equal("incremental", view.Refresh!.Strategy);
        Assert.Equal("1h", view.Refresh.Interval);
    }

    [Fact]
    public void M3LParser_ParseString_LookupAndRollup()
    {
        // Parse customer first, then order
        var file1 = Parser.ParseString("## Customer\n- id: identifier @primary\n- name: string", "customer.m3l.md");
        var file2Content = @"## Order
- id: identifier @primary
- customer_id: identifier @reference(Customer)
- amount: decimal(10,2)

### Lookup
- customer_name: string @lookup(customer_id.name)

## Customer2
- id: identifier @primary

### Rollup
- order_count: integer @rollup(Order.customer_id, count)
- total_spent: decimal(10,2) @rollup(Order.customer_id, sum(amount))";
        var file2 = Parser.ParseString(file2Content, "order.m3l.md");
        var ast = Resolver.Resolve([file1, file2]);

        var order = ast.Models.First(m => m.Name == "Order");
        var lookupField = order.Fields.First(f => f.Name == "customer_name");
        Assert.Equal(FieldKind.Lookup, lookupField.Kind);
        Assert.NotNull(lookupField.Lookup);
        Assert.Equal("customer_id.name", lookupField.Lookup.Path);

        var customer2 = ast.Models.First(m => m.Name == "Customer2");
        var rollupField = customer2.Fields.First(f => f.Name == "order_count");
        Assert.Equal(FieldKind.Rollup, rollupField.Kind);
        Assert.Equal("count", rollupField.Rollup!.Aggregate);

        var totalField = customer2.Fields.First(f => f.Name == "total_spent");
        Assert.Equal("sum", totalField.Rollup!.Aggregate);
        Assert.Equal("amount", totalField.Rollup.Field);
    }

    [Fact]
    public void M3LParser_ValidateString_ReturnsValidation()
    {
        var parser = new M3LParser();
        var (ast, validation) = parser.ValidateString("## User\n- id: identifier @primary");

        Assert.Single(ast.Models);
        Assert.Empty(validation.Errors);
    }

    [Fact]
    public void M3LParser_GetVersions_ReturnsVersions()
    {
        Assert.Equal("1.0", M3LParser.GetAstVersion());
        Assert.Equal("0.1.3", M3LParser.GetParserVersion());
    }

    [Fact]
    public void M3LParser_ParseString_ComputedFromRollupKindSection()
    {
        var content = @"## Product
- id: identifier @primary
- price: decimal

### Computed from Rollup
- display_price: string @computed(""'$' || price"")
";
        var parser = new M3LParser();
        var ast = parser.ParseString(content);

        var field = ast.Models[0].Fields.First(f => f.Name == "display_price");
        Assert.Equal(FieldKind.Computed, field.Kind);
    }

    [Fact]
    public void M3LParser_ParseString_IsStandardAttribute()
    {
        var content = "## User\n- id: identifier @primary @generated\n- name: string(100) @searchable\n- email: string @unique @my_custom_attr";
        var parser = new M3LParser();
        var ast = parser.ParseString(content);
        var fields = ast.Models[0].Fields;

        // @primary is standard
        var primary = fields[0].Attributes.First(a => a.Name == "primary");
        Assert.True(primary.IsStandard);

        // @generated is standard
        var generated = fields[0].Attributes.First(a => a.Name == "generated");
        Assert.True(generated.IsStandard);

        // @searchable is standard
        var searchable = fields[1].Attributes.First(a => a.Name == "searchable");
        Assert.True(searchable.IsStandard);

        // @unique is standard
        var unique = fields[2].Attributes.First(a => a.Name == "unique");
        Assert.True(unique.IsStandard);

        // @my_custom_attr is NOT standard — IsStandard should be null
        var custom = fields[2].Attributes.First(a => a.Name == "my_custom_attr");
        Assert.Null(custom.IsStandard);
    }

    [Fact]
    public void M3LParser_ParseString_CustomAttributeParsed()
    {
        var content = "## User\n- password: string(100) `[DataType(DataType.Password)]` `[JsonIgnore]`";
        var parser = new M3LParser();
        var ast = parser.ParseString(content);
        var field = ast.Models[0].Fields[0];

        Assert.NotNull(field.FrameworkAttrs);
        Assert.Equal(2, field.FrameworkAttrs!.Count);

        // DataType(DataType.Password) → parsed
        Assert.Equal("DataType", field.FrameworkAttrs[0].Parsed?.Name);
        Assert.Single(field.FrameworkAttrs[0].Parsed!.Arguments);
        Assert.Equal("DataType.Password", field.FrameworkAttrs[0].Parsed.Arguments[0]);

        // JsonIgnore → parsed (no args)
        Assert.Equal("JsonIgnore", field.FrameworkAttrs[1].Parsed?.Name);
        Assert.Empty(field.FrameworkAttrs[1].Parsed!.Arguments);
    }

    [Fact]
    public void M3LParser_ParseString_CustomAttributeWithMultipleArgs()
    {
        var content = "## Config\n- port: integer `[Range(1, 65535)]` `[Description(\"Port number\")]`";
        var parser = new M3LParser();
        var ast = parser.ParseString(content);
        var field = ast.Models[0].Fields[0];

        Assert.Equal("Range", field.FrameworkAttrs![0].Parsed?.Name);
        Assert.Equal(2, field.FrameworkAttrs[0].Parsed!.Arguments.Count);
        Assert.Equal(1, field.FrameworkAttrs[0].Parsed.Arguments[0]);
        Assert.Equal(65535, field.FrameworkAttrs[0].Parsed.Arguments[1]);

        Assert.Equal("Description", field.FrameworkAttrs![1].Parsed?.Name);
        Assert.Equal("Port number", field.FrameworkAttrs[1].Parsed!.Arguments[0]);
    }

    [Fact]
    public void M3LParser_ParseString_AttributeRegistryDef()
    {
        var content = @"## @pii ::attribute
> Personal information marker
- target: [field]
- type: boolean
- default: false

## @audit_level ::attribute
> Audit compliance level
- target: [field, model]
- type: integer
- range: [1, 5]
- required: false
- default: 1";

        var parser = new M3LParser();
        var ast = parser.ParseString(content);

        Assert.Equal(2, ast.AttributeRegistry.Count);

        var pii = ast.AttributeRegistry[0];
        Assert.Equal("pii", pii.Name);
        Assert.Equal("Personal information marker", pii.Description);
        Assert.Equal(new[] { "field" }, pii.Target);
        Assert.Equal("boolean", pii.Type);
        Assert.Equal(false, pii.DefaultValue);
        Assert.False(pii.Required);

        var audit = ast.AttributeRegistry[1];
        Assert.Equal("audit_level", audit.Name);
        Assert.Equal("Audit compliance level", audit.Description);
        Assert.Contains("field", audit.Target);
        Assert.Contains("model", audit.Target);
        Assert.Equal("integer", audit.Type);
        Assert.Equal(new[] { 1, 5 }, audit.Range);
        Assert.Equal(1, audit.DefaultValue);

        // No models/enums should be created
        Assert.Empty(ast.Models);
        Assert.Empty(ast.Enums);
    }

    [Fact]
    public void M3LParser_ParseString_IsRegisteredAttribute()
    {
        var source = string.Join("\n", new[]
        {
            "## @pii ::attribute",
            "> Personal info marker",
            "- target: [field]",
            "- type: boolean",
            "",
            "## User",
            "- name: string(100) @searchable",
            "- ssn: string(11) @pii @unique",
        });
        var p = new M3LParser();
        var ast = p.ParseString(source, "test.m3l.md");

        Assert.Single(ast.AttributeRegistry);
        Assert.Equal("pii", ast.AttributeRegistry[0].Name);

        var user = ast.Models.First(m => m.Name == "User");
        // @pii should be isRegistered=true
        var piiAttr = user.Fields[1].Attributes.First(a => a.Name == "pii");
        Assert.True(piiAttr.IsRegistered);

        // @unique is standard but not in registry — IsRegistered should be null
        var uniqueAttr = user.Fields[1].Attributes.First(a => a.Name == "unique");
        Assert.Null(uniqueAttr.IsRegistered);

        // @searchable is standard, not in registry — IsRegistered should be null
        var searchAttr = user.Fields[0].Attributes.First(a => a.Name == "searchable");
        Assert.Null(searchAttr.IsRegistered);
    }

    [Fact]
    public void M3LParser_ParseString_IsRegisteredAcrossFiles()
    {
        var registrySource = string.Join("\n", new[]
        {
            "## @audit_level ::attribute",
            "> Audit compliance level",
            "- target: [field, model]",
            "- type: integer",
        });
        var modelSource = string.Join("\n", new[]
        {
            "## Order @audit_level(3)",
            "- amount: decimal(10,2) @audit_level(5)",
        });

        var registryFile = Parser.ParseString(registrySource, "registry.m3l.md");
        var modelFile = Parser.ParseString(modelSource, "model.m3l.md");
        var ast = Resolver.Resolve(new List<ParsedFile> { registryFile, modelFile });

        // Model-level attribute should be tagged
        var order = ast.Models.First(m => m.Name == "Order");
        var modelAttr = order.Attributes.First(a => a.Name == "audit_level");
        Assert.True(modelAttr.IsRegistered);

        // Field-level attribute should be tagged
        var fieldAttr = order.Fields[0].Attributes.First(a => a.Name == "audit_level");
        Assert.True(fieldAttr.IsRegistered);
    }
}
