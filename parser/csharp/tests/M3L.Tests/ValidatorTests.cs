using M3L;
using M3L.Models;

namespace M3L.Tests;

public class ValidatorTests
{
    private M3LAst ParseAndResolve(string content)
    {
        var parsed = Parser.ParseString(content, "test.m3l.md");
        return Resolver.Resolve([parsed]);
    }

    [Fact]
    public void Validate_NoErrors_ReturnsClean()
    {
        var ast = ParseAndResolve("## User\n- id: identifier @primary\n- name: string");
        var result = Validator.Validate(ast);

        Assert.Empty(result.Errors);
    }

    [Fact]
    public void Validate_ViewFromMissingModel_ReportsE004()
    {
        var content = "## MyView::view\n\n### Source\n- from: NonExistent\n\n- id: identifier";
        var ast = ParseAndResolve(content);
        var result = Validator.Validate(ast);

        Assert.Contains(result.Errors, e => e.Code == "M3L-E004");
    }

    [Fact]
    public void Validate_ViewFromExistingModel_NoError()
    {
        var content = "## User\n- id: identifier\n- name: string\n\n## UserView::view\n\n### Source\n- from: User\n\n- id: identifier";
        var ast = ParseAndResolve(content);
        var result = Validator.Validate(ast);

        Assert.DoesNotContain(result.Errors, e => e.Code == "M3L-E004");
    }

    [Fact]
    public void Validate_StrictMode_LongFieldLine_ReportsW001()
    {
        // Build a field with many attributes to exceed 80 chars
        var content = "## User\n- very_long_field_name_here: string @unique @required @searchable @index \"A long description for this field\"";
        var ast = ParseAndResolve(content);
        var result = Validator.Validate(ast, new ValidateOptions { Strict = true });

        Assert.Contains(result.Warnings, w => w.Code == "M3L-W001");
    }

    [Fact]
    public void Validate_StrictMode_ShortFieldLine_NoWarning()
    {
        var content = "## User\n- id: identifier";
        var ast = ParseAndResolve(content);
        var result = Validator.Validate(ast, new ValidateOptions { Strict = true });

        Assert.DoesNotContain(result.Warnings, w => w.Code == "M3L-W001");
    }

    [Fact]
    public void Validate_DuplicateFieldNames_ReportsE006()
    {
        var content = "## User\n- id: identifier\n- id: string";
        var ast = ParseAndResolve(content);
        var result = Validator.Validate(ast);

        Assert.Contains(result.Errors, e => e.Code == "M3L-E006");
    }

    [Fact]
    public void Validate_PropagatesResolverErrors()
    {
        var content = "## User : NonExistent\n- id: identifier";
        var ast = ParseAndResolve(content);
        var result = Validator.Validate(ast);

        // Resolver error M3L-E007 should be propagated
        Assert.Contains(result.Errors, e => e.Code == "M3L-E007");
    }
}
