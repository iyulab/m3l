using M3L;
using M3L.Models;

namespace M3L.Tests;

public class ParserTests
{
    [Fact]
    public void ParseString_BasicModel_ReturnsModel()
    {
        var content = "## User\n\n- id: identifier @primary\n- name: string(100)";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Single(result.Models);
        Assert.Equal("User", result.Models[0].Name);
        Assert.Equal(2, result.Models[0].Fields.Count);
    }

    [Fact]
    public void ParseString_Namespace_SetsNamespace()
    {
        var content = "# Namespace: myapp\n\n## User\n- id: identifier";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Equal("myapp", result.Namespace);
    }

    [Fact]
    public void ParseString_ModelWithInheritance_SetsInherits()
    {
        var content = "## User : BaseModel\n- id: identifier";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Single(result.Models);
        Assert.Contains("BaseModel", result.Models[0].Inherits);
    }

    [Fact]
    public void ParseString_ModelWithLabel_SetsLabel()
    {
        var content = "## Product(상품)\n- id: identifier";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Equal("Product", result.Models[0].Name);
        Assert.Equal("상품", result.Models[0].Label);
    }

    [Fact]
    public void ParseString_FieldTypes_ParsesCorrectly()
    {
        var content = "## Test\n- a: string(100)\n- b: integer\n- c: decimal(10,2)\n- d: boolean\n- e: text?";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Equal(5, result.Models[0].Fields.Count);
        Assert.Equal("string", result.Models[0].Fields[0].Type);
        Assert.Equal("integer", result.Models[0].Fields[1].Type);
        Assert.Equal("decimal", result.Models[0].Fields[2].Type);
        Assert.Equal("boolean", result.Models[0].Fields[3].Type);
        Assert.Equal("text", result.Models[0].Fields[4].Type);
        Assert.True(result.Models[0].Fields[4].Nullable);
    }

    [Fact]
    public void ParseString_FieldAttributes_ParsesCorrectly()
    {
        var content = "## Test\n- email: string @unique @required";
        var result = Parser.ParseString(content, "test.m3l.md");

        var field = result.Models[0].Fields[0];
        Assert.Equal(2, field.Attributes.Count);
        Assert.Equal("unique", field.Attributes[0].Name);
        Assert.Equal("required", field.Attributes[1].Name);
    }

    [Fact]
    public void ParseString_DefaultValue_ParsesCorrectly()
    {
        var content = "## Test\n- status: string = \"active\"";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Equal("\"active\"", result.Models[0].Fields[0].DefaultValue);
    }

    [Fact]
    public void ParseString_Enum_ParsesValues()
    {
        var content = "## Status::enum\n- Active \"Active status\"\n- Inactive \"Inactive status\"";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Single(result.Enums);
        Assert.Equal("Status", result.Enums[0].Name);
        Assert.Equal(2, result.Enums[0].Values.Count);
        Assert.Equal("Active", result.Enums[0].Values[0].Name);
        Assert.Equal("Active status", result.Enums[0].Values[0].Description);
    }

    [Fact]
    public void ParseString_Interface_ParsesCorrectly()
    {
        var content = "## Timestampable::interface\n- created_at: datetime\n- updated_at: datetime";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Single(result.Interfaces);
        Assert.Equal("Timestampable", result.Interfaces[0].Name);
        Assert.Equal(2, result.Interfaces[0].Fields.Count);
    }

    [Fact]
    public void ParseString_View_ParsesCorrectly()
    {
        var content = "## ActiveUsers::view\n\n### Source\n- from: User\n- where: \"status = 'active'\"\n\n- id: identifier\n- name: string";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Single(result.Views);
        Assert.Equal("ActiveUsers", result.Views[0].Name);
        Assert.Equal("view", result.Views[0].NodeType);
        Assert.NotNull(result.Views[0].SourceDef);
        Assert.Equal("User", result.Views[0].SourceDef!.From);
    }

    [Fact]
    public void ParseString_Blockquote_SetsDescription()
    {
        var content = "## User\n> User account information\n- id: identifier";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Equal("User account information", result.Models[0].Description);
    }

    [Fact]
    public void ParseString_Section_Indexes()
    {
        var content = "## User\n- email: string\n\n### Indexes\n- idx_email";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Single(result.Models[0].Sections.Indexes);
    }

    [Fact]
    public void ParseString_Section_Metadata()
    {
        var content = "## User\n- id: identifier\n\n### Metadata\n- table_name: users\n- schema: public";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Equal("users", result.Models[0].Sections.Metadata["table_name"]);
        Assert.Equal("public", result.Models[0].Sections.Metadata["schema"]);
    }

    [Fact]
    public void ParseString_LookupField_ParsesLookupDef()
    {
        var content = "## Order\n- customer_id: identifier @reference(Customer)\n\n### Lookup\n- customer_name: string @lookup(customer_id.name)";
        var result = Parser.ParseString(content, "test.m3l.md");

        var lookupField = result.Models[0].Fields.First(f => f.Name == "customer_name");
        Assert.Equal(FieldKind.Lookup, lookupField.Kind);
        Assert.NotNull(lookupField.Lookup);
        Assert.Equal("customer_id.name", lookupField.Lookup.Path);
    }

    [Fact]
    public void ParseString_RollupField_ParsesRollupDef()
    {
        var content = "## Customer\n- id: identifier @primary\n\n### Rollup\n- order_count: integer @rollup(Order.customer_id, count)";
        var result = Parser.ParseString(content, "test.m3l.md");

        var rollupField = result.Models[0].Fields.First(f => f.Name == "order_count");
        Assert.Equal(FieldKind.Rollup, rollupField.Kind);
        Assert.NotNull(rollupField.Rollup);
        Assert.Equal("Order", rollupField.Rollup.Target);
        Assert.Equal("customer_id", rollupField.Rollup.Fk);
        Assert.Equal("count", rollupField.Rollup.Aggregate);
    }

    [Fact]
    public void ParseString_ComputedField_ParsesComputedDef()
    {
        var content = "## User\n- first_name: string\n- last_name: string\n\n### Computed\n- full_name: string @computed(\"first_name || ' ' || last_name\")";
        var result = Parser.ParseString(content, "test.m3l.md");

        var computedField = result.Models[0].Fields.First(f => f.Name == "full_name");
        Assert.Equal(FieldKind.Computed, computedField.Kind);
        Assert.NotNull(computedField.Computed);
        Assert.Contains("first_name", computedField.Computed.Expression);
    }

    [Fact]
    public void ParseString_FrameworkAttrs_ParsesCustomAttributes()
    {
        var content = "## User\n- password: string `[DataType(DataType.Password)]` `[JsonIgnore]`";
        var result = Parser.ParseString(content, "test.m3l.md");

        var field = result.Models[0].Fields[0];
        Assert.NotNull(field.FrameworkAttrs);
        Assert.Equal(2, field.FrameworkAttrs.Count);
        Assert.Equal("DataType(DataType.Password)", field.FrameworkAttrs[0].Content);
        Assert.Equal("[DataType(DataType.Password)]", field.FrameworkAttrs[0].Raw);
        Assert.Equal("JsonIgnore", field.FrameworkAttrs[1].Content);
    }

    [Fact]
    public void ParseString_ArrayField_ParsesCorrectly()
    {
        var content = "## Test\n- tags: string[]";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.True(result.Models[0].Fields[0].Array);
    }

    [Fact]
    public void ParseString_NullableField_ParsesCorrectly()
    {
        var content = "## Test\n- phone: string?";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.True(result.Models[0].Fields[0].Nullable);
    }

    [Fact]
    public void ParseString_MultipleModels_ParsesAll()
    {
        var content = "## User\n- id: identifier\n\n## Product\n- id: identifier";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Equal(2, result.Models.Count);
        Assert.Equal("User", result.Models[0].Name);
        Assert.Equal("Product", result.Models[1].Name);
    }

    [Fact]
    public void ParseString_InlineEnum_ParsesValues()
    {
        var content = "## Product\n- status: enum\n  - Active: \"Active product\"\n  - Inactive: \"Inactive product\"";
        var result = Parser.ParseString(content, "test.m3l.md");

        var field = result.Models[0].Fields[0];
        Assert.Equal("enum", field.Type);
        Assert.NotNull(field.EnumValues);
        Assert.Equal(2, field.EnumValues.Count);
        Assert.Equal("Active", field.EnumValues[0].Name);
    }

    [Fact]
    public void ParseString_ViewWithJoins_ParsesJoinDefs()
    {
        var content = "## Report::view\n\n### Source\n- from: Order\n- join: Customer on Order.customer_id = Customer.id\n\n- order_id: identifier";
        var result = Parser.ParseString(content, "test.m3l.md");

        var view = result.Views[0];
        Assert.NotNull(view.SourceDef);
        Assert.NotNull(view.SourceDef.Joins);
        Assert.Single(view.SourceDef.Joins);
        Assert.Equal("Customer", view.SourceDef.Joins[0].Model);
    }

    [Fact]
    public void ParseString_MaterializedView_SetsMaterialized()
    {
        var content = "## Stats::view @materialized\n\n### Source\n- from: Order\n\n- total: integer";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.True(result.Views[0].Materialized);
    }

    [Fact]
    public void ParseString_Behaviors_ParsesSection()
    {
        var content = "## User\n- id: identifier\n\n### Behaviors\n- on_create";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Single(result.Models[0].Sections.Behaviors);
    }

    [Fact]
    public void ParseString_SourceLocation_IsSet()
    {
        var content = "## User\n- id: identifier";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Equal("test.m3l.md", result.Models[0].Loc.File);
        Assert.Equal(1, result.Models[0].Loc.Line);
        Assert.Equal("test.m3l.md", result.Models[0].Fields[0].Loc.File);
        Assert.Equal(2, result.Models[0].Fields[0].Loc.Line);
    }

    [Fact]
    public void ParseString_RollupWithWhereClause_Parses()
    {
        var content = "## Customer\n- id: identifier\n\n### Rollup\n- active_orders: integer @rollup(Order.customer_id, count, where: \"status = 'active'\")";
        var result = Parser.ParseString(content, "test.m3l.md");

        var field = result.Models[0].Fields.First(f => f.Name == "active_orders");
        Assert.NotNull(field.Rollup);
        Assert.Equal("status = 'active'", field.Rollup.Where);
    }

    [Fact]
    public void ParseString_RollupWithAggregateField_Parses()
    {
        var content = "## Customer\n- id: identifier\n\n### Rollup\n- total_spent: decimal @rollup(Order.customer_id, sum(total_amount))";
        var result = Parser.ParseString(content, "test.m3l.md");

        var field = result.Models[0].Fields.First(f => f.Name == "total_spent");
        Assert.NotNull(field.Rollup);
        Assert.Equal("sum", field.Rollup.Aggregate);
        Assert.Equal("total_amount", field.Rollup.Field);
    }

    [Fact]
    public void ParseString_UniqueDirective_StoredInIndexesWithUniqueFlag()
    {
        var content = "## User\n- name: string(100)\n- email: string(320)\n- @unique(email, name: \"uq_email\")";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Single(result.Models[0].Sections.Indexes);
        var idx = result.Models[0].Sections.Indexes[0] as Dictionary<string, object?>;
        Assert.NotNull(idx);
        Assert.Equal("directive", idx["type"]);
        Assert.Equal(true, idx["unique"]);
        var args = idx["args"] as string;
        Assert.Contains("email", args!);
    }

    [Fact]
    public void ParseString_IndexDirective_StoredInIndexesWithoutUniqueFlag()
    {
        var content = "## User\n- name: string(100)\n- @index(name)";
        var result = Parser.ParseString(content, "test.m3l.md");

        Assert.Single(result.Models[0].Sections.Indexes);
        var idx = result.Models[0].Sections.Indexes[0] as Dictionary<string, object?>;
        Assert.NotNull(idx);
        Assert.Equal("directive", idx["type"]);
        Assert.Equal(false, idx["unique"]);
    }

    [Fact]
    public void Parse_EnumInheritance_ExtractsBase()
    {
        var input = string.Join("\n", new[]
        {
            "## BasicStatus ::enum",
            "- active: \"Active\"",
            "- inactive: \"Inactive\"",
            "",
            "## UserStatus ::enum : BasicStatus",
            "- suspended: \"Suspended\"",
            "- banned: \"Banned\""
        });
        var result = Parser.ParseString(input, "test.m3l");
        Assert.Equal(2, result.Enums.Count);
        var userStatus = result.Enums.First(e => e.Name == "UserStatus");
        Assert.Equal(new[] { "BasicStatus" }, userStatus.Inherits);
        var basicStatus = result.Enums.First(e => e.Name == "BasicStatus");
        Assert.Empty(basicStatus.Inherits);
    }

    [Fact]
    public void ParseString_BlockquoteFieldDescription_AppliesToField()
    {
        var input = string.Join("\n", new[]
        {
            "## User",
            "- username: string(50) @unique",
            "  > Unique identifier used for login",
            "- email: string(320)"
        });
        var result = Parser.ParseString(input, "test.m3l.md");
        var user = result.Models[0];
        Assert.Equal("Unique identifier used for login", user.Fields[0].Description);
        Assert.Null(user.Fields[1].Description);
    }

    [Fact]
    public void ParseString_InlineComment_SetsFieldDescription()
    {
        var input = string.Join("\n", new[]
        {
            "## User",
            "- email: string(320) @unique  # Primary contact email"
        });
        var result = Parser.ParseString(input, "test.m3l.md");
        var user = result.Models[0];
        Assert.Equal("Primary contact email", user.Fields[0].Description);
    }

    [Fact]
    public void ParseString_ModelLevelBlockquote_PreservedBeforeFields()
    {
        var input = string.Join("\n", new[]
        {
            "## User",
            "> User account information",
            "- name: string(100)"
        });
        var result = Parser.ParseString(input, "test.m3l.md");
        var user = result.Models[0];
        Assert.Equal("User account information", user.Description);
    }

    [Fact]
    public void Parse_RelationSection_SubItems()
    {
        var input = string.Join("\n", new[]
        {
            "## Content",
            "- id: identifier @primary",
            "- author_id: identifier",
            "",
            "### Relations",
            "- >author",
            "  - target: Person",
            "  - from: author_id",
            "  - on_delete: restrict"
        });
        var result = Parser.ParseString(input, "test.m3l");
        var content = result.Models[0];
        Assert.Single(content.Sections.Relations);
        var rel = (Dictionary<string, object?>)content.Sections.Relations[0];
        Assert.Equal(">author", rel["raw"]);
        Assert.Equal("Person", rel["target"]);
        Assert.Equal("author_id", rel["from"]);
        Assert.Equal("restrict", rel["on_delete"]);
    }
}
