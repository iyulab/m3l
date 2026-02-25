using M3L;
using M3L.Models;

namespace M3L.Tests;

public class LexerTests
{
    [Fact]
    public void Lex_BlankLines_ProducesBlankTokens()
    {
        var tokens = Lexer.Lex("\n\n", "test.m3l.md");
        Assert.All(tokens, t => Assert.Equal(TokenType.Blank, t.Type));
    }

    [Fact]
    public void Lex_Namespace_ProducesNamespaceToken()
    {
        var tokens = Lexer.Lex("# Namespace: myapp", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.Namespace, tokens[0].Type);
        Assert.Equal("myapp", tokens[0].Data["name"]);
    }

    [Fact]
    public void Lex_ModelDefinition_ProducesModelToken()
    {
        var tokens = Lexer.Lex("## User", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.Model, tokens[0].Type);
        Assert.Equal("User", tokens[0].Data["name"]);
    }

    [Fact]
    public void Lex_ModelWithLabel_ParsesNameAndLabel()
    {
        var tokens = Lexer.Lex("## Product(상품)", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal("Product", tokens[0].Data["name"]);
        Assert.Equal("상품", tokens[0].Data["label"]);
    }

    [Fact]
    public void Lex_ModelWithInheritance_ParsesParents()
    {
        var tokens = Lexer.Lex("## User : BaseModel, Auditable", "test.m3l.md");
        Assert.Single(tokens);
        var inherits = tokens[0].Data["inherits"] as List<string>;
        Assert.NotNull(inherits);
        Assert.Equal(2, inherits.Count);
        Assert.Contains("BaseModel", inherits);
        Assert.Contains("Auditable", inherits);
    }

    [Fact]
    public void Lex_EnumDefinition_ProducesEnumToken()
    {
        var tokens = Lexer.Lex("## Status::enum", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.Enum, tokens[0].Type);
        Assert.Equal("Status", tokens[0].Data["name"]);
    }

    [Fact]
    public void Lex_InterfaceDefinition_ProducesInterfaceToken()
    {
        var tokens = Lexer.Lex("## Timestampable::interface", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.Interface, tokens[0].Type);
        Assert.Equal("Timestampable", tokens[0].Data["name"]);
    }

    [Fact]
    public void Lex_ViewDefinition_ProducesViewToken()
    {
        var tokens = Lexer.Lex("## ActiveUsers::view", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.View, tokens[0].Type);
        Assert.Equal("ActiveUsers", tokens[0].Data["name"]);
    }

    [Fact]
    public void Lex_FieldLine_ParsesNameAndType()
    {
        var tokens = Lexer.Lex("- name: string(100)", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.Field, tokens[0].Type);
        Assert.Equal("name", tokens[0].Data["name"]);
        Assert.Equal("string", tokens[0].Data["type_name"]);
    }

    [Fact]
    public void Lex_FieldWithAttributes_ParsesAttributes()
    {
        var tokens = Lexer.Lex("- email: string @unique @required", "test.m3l.md");
        var attrs = tokens[0].Data["attributes"] as List<Dictionary<string, object?>>;
        Assert.NotNull(attrs);
        Assert.Equal(2, attrs.Count);
        Assert.Equal("unique", attrs[0]["name"]);
        Assert.Equal("required", attrs[1]["name"]);
    }

    [Fact]
    public void Lex_FieldNullable_ParsesNullableFlag()
    {
        var tokens = Lexer.Lex("- phone: string?", "test.m3l.md");
        Assert.True(tokens[0].Data["nullable"] is true);
    }

    [Fact]
    public void Lex_FieldArray_ParsesArrayFlag()
    {
        var tokens = Lexer.Lex("- tags: string[]", "test.m3l.md");
        Assert.True(tokens[0].Data["array"] is true);
    }

    [Fact]
    public void Lex_FieldDefaultValue_ParsesDefault()
    {
        var tokens = Lexer.Lex("- status: string = \"active\"", "test.m3l.md");
        Assert.Equal("\"active\"", tokens[0].Data["default_value"]);
    }

    [Fact]
    public void Lex_FieldDescription_ParsesDescription()
    {
        var tokens = Lexer.Lex("- name: string \"User's full name\"", "test.m3l.md");
        Assert.Equal("User's full name", tokens[0].Data["description"]);
    }

    [Fact]
    public void Lex_FrameworkAttribute_ParsesBracketedAttr()
    {
        var tokens = Lexer.Lex("- password: string `[DataType(DataType.Password)]`", "test.m3l.md");
        var fwAttrs = tokens[0].Data["framework_attrs"] as List<string>;
        Assert.NotNull(fwAttrs);
        Assert.Single(fwAttrs);
        Assert.Equal("[DataType(DataType.Password)]", fwAttrs[0]);
    }

    [Fact]
    public void Lex_InlineComment_ParsesComment()
    {
        var tokens = Lexer.Lex("- name: string # the user's name", "test.m3l.md");
        Assert.Equal("the user's name", tokens[0].Data["comment"]);
    }

    [Fact]
    public void Lex_HorizontalRule_ProducesHrToken()
    {
        var tokens = Lexer.Lex("---", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.HorizontalRule, tokens[0].Type);
    }

    [Fact]
    public void Lex_Blockquote_ProducesBlockquoteToken()
    {
        var tokens = Lexer.Lex("> Description text", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.Blockquote, tokens[0].Type);
        Assert.Equal("Description text", tokens[0].Data["text"]);
    }

    [Fact]
    public void Lex_NestedItem_ProducesNestedToken()
    {
        var tokens = Lexer.Lex("  - type: unique", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.NestedItem, tokens[0].Type);
        Assert.Equal("type", tokens[0].Data["key"]);
        Assert.Equal("unique", tokens[0].Data["value"]);
    }

    [Fact]
    public void Lex_Section_ProducesSectionToken()
    {
        var tokens = Lexer.Lex("### Indexes", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.Section, tokens[0].Type);
        Assert.Equal("Indexes", tokens[0].Data["name"]);
    }

    [Fact]
    public void Lex_KindSection_ProducesKindSectionToken()
    {
        var tokens = Lexer.Lex("# Lookup", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.Section, tokens[0].Type);
        Assert.True(tokens[0].Data["kind_section"] is true);
    }

    [Fact]
    public void Lex_Import_ProducesTextTokenWithImport()
    {
        var tokens = Lexer.Lex("@import './other.m3l.md'", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.Text, tokens[0].Type);
        Assert.True(tokens[0].Data["is_import"] is true);
        Assert.Equal("./other.m3l.md", tokens[0].Data["import_path"]);
    }

    [Fact]
    public void Lex_DirectiveLine_ParsesAsField()
    {
        var tokens = Lexer.Lex("- @index(email, name)", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.Field, tokens[0].Type);
        Assert.True(tokens[0].Data["is_directive"] is true);
    }

    [Fact]
    public void Lex_ReferenceAttribute_ParsesArgs()
    {
        var tokens = Lexer.Lex("- user_id: identifier @reference(User)!", "test.m3l.md");
        var attrs = tokens[0].Data["attributes"] as List<Dictionary<string, object?>>;
        Assert.NotNull(attrs);
        Assert.Single(attrs);
        Assert.Equal("reference", attrs[0]["name"]);
    }

    [Fact]
    public void Lex_EnumValueWithDescription_Parses()
    {
        var tokens = Lexer.Lex("- Active \"Currently active\"", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal("Active", tokens[0].Data["name"]);
        Assert.Equal("Currently active", tokens[0].Data["description"]);
    }

    [Fact]
    public void Lex_MaterializedView_ParsesFlag()
    {
        var tokens = Lexer.Lex("## SalesReport::view @materialized", "test.m3l.md");
        Assert.Single(tokens);
        Assert.Equal(TokenType.View, tokens[0].Type);
        Assert.True(tokens[0].Data["materialized"] is true);
    }

    [Fact]
    public void Lex_MultilineContent_PreservesLineNumbers()
    {
        var content = "# Namespace: test\n\n## User\n\n- id: identifier";
        var tokens = Lexer.Lex(content, "test.m3l.md");
        var fieldToken = tokens.First(t => t.Type == TokenType.Field);
        Assert.Equal(5, fieldToken.Line);
    }
}
