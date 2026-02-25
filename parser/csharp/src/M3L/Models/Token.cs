namespace M3L.Models;

public enum TokenType
{
    Namespace,
    Model,
    Enum,
    Interface,
    View,
    AttributeDef,
    Section,
    Field,
    NestedItem,
    Blockquote,
    HorizontalRule,
    Blank,
    Text
}

public class Token
{
    public TokenType Type { get; set; }
    public string Raw { get; set; } = "";
    public int Line { get; set; }
    public int Indent { get; set; }
    public Dictionary<string, object?> Data { get; set; } = new();
}
