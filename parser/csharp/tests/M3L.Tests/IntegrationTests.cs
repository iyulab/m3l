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

# Lookup
- customer_name: string @lookup(customer_id.name)

## Customer2
- id: identifier @primary

# Rollup
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
        Assert.Equal("0.1.0", M3LParser.GetParserVersion());
    }

    [Fact]
    public void M3LParser_ParseString_ComputedFromRollupKindSection()
    {
        var content = @"## Product
- id: identifier @primary
- price: decimal

# Computed from Rollup
- display_price: string @computed(""'$' || price"")
";
        var parser = new M3LParser();
        var ast = parser.ParseString(content);

        var field = ast.Models[0].Fields.First(f => f.Name == "display_price");
        Assert.Equal(FieldKind.Computed, field.Kind);
    }
}
