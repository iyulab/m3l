using M3L;
using M3L.Models;

namespace M3L.Tests;

public class ResolverTests
{
    [Fact]
    public void Resolve_SingleFile_ReturnsAst()
    {
        var content = "## User\n- id: identifier @primary\n- name: string";
        var parsed = Parser.ParseString(content, "test.m3l.md");
        var ast = Resolver.Resolve([parsed]);

        Assert.Single(ast.Models);
        Assert.Equal("User", ast.Models[0].Name);
        Assert.Equal(Resolver.ParserVersion, ast.ParserVersion);
        Assert.Equal(Resolver.AstVersion, ast.AstVersion);
    }

    [Fact]
    public void Resolve_MultipleFiles_MergesModels()
    {
        var file1 = Parser.ParseString("## User\n- id: identifier", "user.m3l.md");
        var file2 = Parser.ParseString("## Product\n- id: identifier", "product.m3l.md");
        var ast = Resolver.Resolve([file1, file2]);

        Assert.Equal(2, ast.Models.Count);
        Assert.Equal(2, ast.Sources.Count);
    }

    [Fact]
    public void Resolve_DuplicateModelNames_ReportsError()
    {
        var file1 = Parser.ParseString("## User\n- id: identifier", "a.m3l.md");
        var file2 = Parser.ParseString("## User\n- id: identifier", "b.m3l.md");
        var ast = Resolver.Resolve([file1, file2]);

        Assert.Contains(ast.Errors, e => e.Code == "M3L-E005" && e.Message.Contains("User"));
    }

    [Fact]
    public void Resolve_Inheritance_InheritsFields()
    {
        var content = "## Timestampable::interface\n- created_at: datetime\n- updated_at: datetime\n\n## User : Timestampable\n- id: identifier\n- name: string";
        var parsed = Parser.ParseString(content, "test.m3l.md");
        var ast = Resolver.Resolve([parsed]);

        var user = ast.Models.First(m => m.Name == "User");
        // Should have inherited fields + own fields
        Assert.Equal(4, user.Fields.Count);
        Assert.Equal("created_at", user.Fields[0].Name);
        Assert.Equal("updated_at", user.Fields[1].Name);
        Assert.Equal("id", user.Fields[2].Name);
        Assert.Equal("name", user.Fields[3].Name);
    }

    [Fact]
    public void Resolve_UnresolvedInheritance_ReportsError()
    {
        var content = "## User : NonExistent\n- id: identifier";
        var parsed = Parser.ParseString(content, "test.m3l.md");
        var ast = Resolver.Resolve([parsed]);

        Assert.Contains(ast.Errors, e => e.Code == "M3L-E007" && e.Message.Contains("NonExistent"));
    }

    [Fact]
    public void Resolve_TransitiveInheritance_InheritsAllFields()
    {
        var content = "## A::interface\n- a_field: string\n\n## B : A\n- b_field: string\n\n## C : B\n- c_field: string";
        var parsed = Parser.ParseString(content, "test.m3l.md");
        var ast = Resolver.Resolve([parsed]);

        // B inherits from A (interface), so B should get a_field
        var b = ast.Models.First(m => m.Name == "B");
        Assert.Equal(2, b.Fields.Count); // a_field + b_field

        // C inherits from B, which inherits from A
        var c = ast.Models.First(m => m.Name == "C");
        Assert.Equal(3, c.Fields.Count); // a_field + b_field + c_field
    }

    [Fact]
    public void Resolve_MultipleInheritance_MergesFields()
    {
        var content = "## HasTimestamp::interface\n- created_at: datetime\n\n## HasAudit::interface\n- audit_log: text\n\n## User : HasTimestamp, HasAudit\n- id: identifier";
        var parsed = Parser.ParseString(content, "test.m3l.md");
        var ast = Resolver.Resolve([parsed]);

        var user = ast.Models.First(m => m.Name == "User");
        Assert.Equal(3, user.Fields.Count);
    }

    [Fact]
    public void Resolve_Namespace_SetsProjectName()
    {
        var content = "# Namespace: myapp\n\n## User\n- id: identifier";
        var parsed = Parser.ParseString(content, "test.m3l.md");
        var ast = Resolver.Resolve([parsed]);

        Assert.Equal("myapp", ast.Project.Name);
    }

    [Fact]
    public void Resolve_ExplicitProject_UsesProvidedProject()
    {
        var content = "## User\n- id: identifier";
        var parsed = Parser.ParseString(content, "test.m3l.md");
        var project = new ProjectInfo { Name = "TestProject", Version = "1.0.0" };
        var ast = Resolver.Resolve([parsed], project);

        Assert.Equal("TestProject", ast.Project.Name);
        Assert.Equal("1.0.0", ast.Project.Version);
    }

    [Fact]
    public void Resolve_DuplicateFieldsInModel_ReportsError()
    {
        var content = "## User\n- id: identifier\n- id: string";
        var parsed = Parser.ParseString(content, "test.m3l.md");
        var ast = Resolver.Resolve([parsed]);

        Assert.Contains(ast.Errors, e => e.Code == "M3L-E005" && e.Message.Contains("Duplicate field"));
    }

    [Fact]
    public void Resolve_EnumsAndViews_AreMerged()
    {
        var content = "## Status::enum\n- Active\n- Inactive\n\n## UserView::view\n\n### Source\n- from: User\n\n- id: identifier";
        var parsed = Parser.ParseString(content, "test.m3l.md");
        var ast = Resolver.Resolve([parsed]);

        Assert.Single(ast.Enums);
        Assert.Single(ast.Views);
    }

    [Fact]
    public void Resolve_CrossFileInheritance_Works()
    {
        var file1 = Parser.ParseString("## BaseModel::interface\n- id: identifier @primary\n- created_at: datetime", "base.m3l.md");
        var file2 = Parser.ParseString("## User : BaseModel\n- name: string\n- email: string", "user.m3l.md");
        var ast = Resolver.Resolve([file1, file2]);

        var user = ast.Models.First(m => m.Name == "User");
        Assert.Equal(4, user.Fields.Count);
        Assert.Equal("id", user.Fields[0].Name);
    }
}
