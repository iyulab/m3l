using System.Text.RegularExpressions;
using M3L.Models;

namespace M3L;

/// <summary>
/// Tokenizes M3L markdown content into a sequence of tokens.
/// </summary>
public static partial class Lexer
{
    private static readonly Regex ReH1 = new(@"^# (.+)$");
    private static readonly Regex ReH2 = new(@"^## (.+)$");
    private static readonly Regex ReH3 = new(@"^### (.+)$");
    private static readonly Regex ReHr = new(@"^-{3,}$");
    private static readonly Regex ReBlockquote = new(@"^\s*> (.+)$");
    private static readonly Regex ReListItem = new(@"^(\s*)- (.+)$");
    private static readonly Regex ReBlank = new(@"^\s*$");

    private static readonly Regex ReTypeIndicator = new(@"^(@?[\w][\w.]*(?:\([^)]*\))?)\s*::(\w+)(.*)$");
    private static readonly Regex ReModelDef = new(@"^([\w][\w.]*(?:\([^)]*\))?)\s*(?::\s*(.+?))?(\s+@.+)?$");
    private static readonly Regex ReFieldName = new(@"^([\w]+)(?:\(([^)]*)\))?\s*(?::\s*(.+))?$");
    private static readonly Regex ReTypePart = new(@"^([\w]+)(?:<([^>]+)>)?(?:\(([^)]*)\))?(\?)?(\[\])?(\?)?");
    private static readonly Regex ReFrameworkAttr = new(@"`\[([^\]]+)\]`");
    private static readonly Regex ReInlineComment = new(@"\s+#\s+(.+)$");
    private static readonly Regex ReImport = new(@"^@import\s+[""'](.+?)[""']\s*$");

    private static readonly HashSet<string> KindSections = new(StringComparer.Ordinal)
    {
        "Lookup", "Rollup", "Computed", "Computed from Rollup"
    };

    public static List<Token> Lex(string content, string file)
    {
        var lines = content.Split('\n');
        var tokens = new List<Token>();

        for (int i = 0; i < lines.Length; i++)
        {
            var raw = lines[i].TrimEnd('\r');
            var lineNum = i + 1;

            if (ReBlank.IsMatch(raw))
            {
                tokens.Add(new Token { Type = TokenType.Blank, Raw = raw, Line = lineNum });
                continue;
            }

            if (ReHr.IsMatch(raw.Trim()))
            {
                tokens.Add(new Token { Type = TokenType.HorizontalRule, Raw = raw, Line = lineNum });
                continue;
            }

            var h3 = ReH3.Match(raw);
            if (h3.Success)
            {
                var h3Name = h3.Groups[1].Value.Trim();
                var h3Data = new Dictionary<string, object?> { ["name"] = h3Name };
                if (KindSections.Contains(h3Name))
                    h3Data["kind_section"] = true;
                tokens.Add(new Token
                {
                    Type = TokenType.Section, Raw = raw, Line = lineNum,
                    Data = h3Data
                });
                continue;
            }

            var h2 = ReH2.Match(raw);
            if (h2.Success)
            {
                tokens.Add(TokenizeH2(h2.Groups[1].Value.Trim(), raw, lineNum));
                continue;
            }

            var h1 = ReH1.Match(raw);
            if (h1.Success)
            {
                var h1Content = h1.Groups[1].Value.Trim();
                tokens.Add(new Token
                {
                    Type = TokenType.Namespace, Raw = raw, Line = lineNum,
                    Data = ParseNamespace(h1Content)
                });
                continue;
            }

            var bq = ReBlockquote.Match(raw);
            if (bq.Success)
            {
                tokens.Add(new Token
                {
                    Type = TokenType.Blockquote, Raw = raw, Line = lineNum,
                    Data = new() { ["text"] = bq.Groups[1].Value.Trim() }
                });
                continue;
            }

            var list = ReListItem.Match(raw);
            if (list.Success)
            {
                var indent = list.Groups[1].Value.Length;
                var itemContent = list.Groups[2].Value;

                if (indent >= 2)
                {
                    tokens.Add(new Token
                    {
                        Type = TokenType.NestedItem, Raw = raw, Line = lineNum, Indent = indent,
                        Data = ParseNestedItem(itemContent)
                    });
                }
                else
                {
                    tokens.Add(new Token
                    {
                        Type = TokenType.Field, Raw = raw, Line = lineNum,
                        Data = ParseFieldLine(itemContent)
                    });
                }
                continue;
            }

            var imp = ReImport.Match(raw.Trim());
            if (imp.Success)
            {
                tokens.Add(new Token
                {
                    Type = TokenType.Text, Raw = raw, Line = lineNum,
                    Data = new() { ["text"] = raw.Trim(), ["is_import"] = true, ["import_path"] = imp.Groups[1].Value }
                });
                continue;
            }

            tokens.Add(new Token
            {
                Type = TokenType.Text, Raw = raw, Line = lineNum,
                Data = new() { ["text"] = raw.Trim() }
            });
        }

        return tokens;
    }

    private static Token TokenizeH2(string content, string raw, int line)
    {
        var ti = ReTypeIndicator.Match(content);
        if (ti.Success)
        {
            var namepart = ti.Groups[1].Value;
            var typeIndicator = ti.Groups[2].Value;
            var rest = ti.Groups[3].Value.Trim();

            var (name, label) = ParseNameLabel(namepart);
            var data = new Dictionary<string, object?> { ["name"] = name, ["label"] = label };

            // Parse inheritance: ::enum : Base1, Base2
            var inheritMatch = Regex.Match(rest, @"^:\s*(.+?)(?:\s+@|\s*""|\s*$)");
            if (inheritMatch.Success)
                data["inherits"] = inheritMatch.Groups[1].Value.Split(',').Select(s => s.Trim()).Where(s => s.Length > 0).ToList();
            else
                data["inherits"] = new List<string>();

            if (typeIndicator == "view")
                data["materialized"] = rest.Contains("@materialized");

            var descMatch = Regex.Match(rest, @"""([^""]+)""");
            if (descMatch.Success)
                data["description"] = descMatch.Groups[1].Value;

            var tokenType = typeIndicator switch
            {
                "enum" => TokenType.Enum,
                "interface" => TokenType.Interface,
                "view" => TokenType.View,
                "attribute" => TokenType.AttributeDef,
                _ => TokenType.Model,
            };

            return new Token { Type = tokenType, Raw = raw, Line = line, Data = data };
        }

        var md = ReModelDef.Match(content);
        if (md.Success)
        {
            var namepart = md.Groups[1].Value;
            var inheritsStr = md.Groups[2].Value.Trim();
            var attrsStr = md.Groups[3].Value.Trim();

            var (name, label) = ParseNameLabel(namepart);
            var inherits = string.IsNullOrEmpty(inheritsStr)
                ? new List<string>()
                : inheritsStr.Split(',').Select(s => s.Trim()).Where(s => s.Length > 0).ToList();

            var data = new Dictionary<string, object?>
            {
                ["name"] = name,
                ["label"] = label,
                ["inherits"] = inherits
            };

            if (!string.IsNullOrEmpty(attrsStr))
            {
                var attrs = new List<Dictionary<string, object?>>();
                foreach (Match m in Regex.Matches(attrsStr, @"@([\w]+)(?:\(([^)]*)\))?"))
                {
                    var attr = new Dictionary<string, object?> { ["name"] = m.Groups[1].Value };
                    if (m.Groups[2].Success && m.Groups[2].Value.Length > 0)
                        attr["args"] = new List<string> { m.Groups[2].Value };
                    attrs.Add(attr);
                }
                data["attributes"] = attrs;
            }

            return new Token { Type = TokenType.Model, Raw = raw, Line = line, Data = data };
        }

        return new Token
        {
            Type = TokenType.Model, Raw = raw, Line = line,
            Data = new() { ["name"] = content, ["inherits"] = new List<string>() }
        };
    }

    private static (string name, string? label) ParseNameLabel(string s)
    {
        var m = Regex.Match(s, @"^([\w][\w.]*)\(([^)]*)\)$");
        if (m.Success) return (m.Groups[1].Value, m.Groups[2].Value);
        return (s, null);
    }

    private static Dictionary<string, object?> ParseNamespace(string content)
    {
        var ns = Regex.Match(content, @"^Namespace:\s*(.+)$");
        if (ns.Success)
            return new() { ["name"] = ns.Groups[1].Value.Trim(), ["is_namespace"] = true };
        return new() { ["name"] = content, ["is_namespace"] = false };
    }

    private static Dictionary<string, object?> ParseFieldLine(string content)
    {
        var data = new Dictionary<string, object?>();

        // Directive-only line
        if (content.StartsWith('@'))
        {
            data["is_directive"] = true;
            data["raw_content"] = content;
            data["attributes"] = ParseAttributesBalanced(content);
            return data;
        }

        // Strip inline comment
        var commentMatch = ReInlineComment.Match(content);
        if (commentMatch.Success)
        {
            data["comment"] = commentMatch.Groups[1].Value;
            content = ReInlineComment.Replace(content, "");
        }

        // Framework attributes
        var fwMatches = ReFrameworkAttr.Matches(content);
        if (fwMatches.Count > 0)
        {
            var fwAttrs = fwMatches.Select(m => $"[{m.Groups[1].Value}]").ToList();
            data["framework_attrs"] = fwAttrs;
            content = ReFrameworkAttr.Replace(content, "").Trim();
        }

        // Enum value: NAME "desc"
        var enumVal = Regex.Match(content, @"^([\w]+)(?:\(([^)]*)\))?\s+""((?:[^""\\]|\\.)*)""$");
        if (enumVal.Success)
        {
            data["name"] = enumVal.Groups[1].Value;
            if (enumVal.Groups[2].Success && enumVal.Groups[2].Value.Length > 0)
                data["label"] = enumVal.Groups[2].Value;
            data["description"] = enumVal.Groups[3].Value;
            return data;
        }

        // Parse name(label): rest
        var field = ReFieldName.Match(content);
        if (!field.Success)
        {
            data["name"] = content;
            return data;
        }

        data["name"] = field.Groups[1].Value;
        if (field.Groups[2].Success && field.Groups[2].Value.Length > 0)
            data["label"] = field.Groups[2].Value;

        var rest = field.Groups[3].Value.Trim();
        if (string.IsNullOrEmpty(rest)) return data;

        data["raw_value"] = rest;
        ParseTypeAndAttrs(rest, data);
        return data;
    }

    private static void ParseTypeAndAttrs(string rest, Dictionary<string, object?> data)
    {
        int pos = 0;
        int len = rest.Length;
        void SkipWs() { while (pos < len && rest[pos] == ' ') pos++; }

        // Quoted string as entire rest
        if (rest[0] == '"')
        {
            var closeIdx = FindClosingQuote(rest, 0);
            if (closeIdx >= 0 && closeIdx == len - 1)
            {
                data["description"] = rest[1..closeIdx];
                return;
            }
        }

        // Type
        var typeMatch = ReTypePart.Match(rest);
        if (typeMatch.Success)
        {
            data["type_name"] = typeMatch.Groups[1].Value;
            if (typeMatch.Groups[2].Success && typeMatch.Groups[2].Value.Length > 0)
                data["type_generic_params"] = typeMatch.Groups[2].Value.Split(',').Select(s => s.Trim()).ToList();
            if (typeMatch.Groups[3].Success && typeMatch.Groups[3].Value.Length > 0)
                data["type_params"] = typeMatch.Groups[3].Value.Split(',').Select(s => s.Trim()).ToList();
            var isArray = typeMatch.Groups[5].Success && typeMatch.Groups[5].Value == "[]";
            data["array"] = isArray;
            if (isArray)
            {
                // Group 4: ? before [] = element nullable; Group 6: ? after [] = container nullable
                data["nullable"] = typeMatch.Groups[6].Success && typeMatch.Groups[6].Value == "?";
                data["arrayItemNullable"] = typeMatch.Groups[4].Success && typeMatch.Groups[4].Value == "?";
            }
            else
            {
                data["nullable"] = (typeMatch.Groups[4].Success && typeMatch.Groups[4].Value == "?")
                                 || (typeMatch.Groups[6].Success && typeMatch.Groups[6].Value == "?");
                data["arrayItemNullable"] = false;
            }
            pos = typeMatch.Length;
            SkipWs();
        }

        // Default value
        if (pos < len && rest[pos] == '=')
        {
            pos++;
            SkipWs();
            if (pos < len && rest[pos] == '"')
            {
                var closeIdx = FindClosingQuote(rest, pos);
                if (closeIdx >= 0)
                {
                    data["default_value"] = rest[pos..(closeIdx + 1)];
                    pos = closeIdx + 1;
                    SkipWs();
                }
            }
            else
            {
                int start = pos;
                while (pos < len && rest[pos] != ' ' && rest[pos] != '@' && rest[pos] != '"')
                {
                    if (rest[pos] == '(')
                    {
                        var closeP = FindBalancedParen(rest, pos);
                        pos = closeP >= 0 ? closeP + 1 : pos + 1;
                    }
                    else pos++;
                }
                data["default_value"] = rest[start..pos];
                SkipWs();
            }
        }

        // Attributes (with optional cascade symbols: !, !!, ?)
        var attrs = new List<Dictionary<string, object?>>();
        while (pos < len && (rest[pos] == '@' || rest[pos] == '!' || rest[pos] == '?'))
        {
            // Cascade symbols: !, !!, ? — attach to the previous attribute
            if (rest[pos] == '!' || rest[pos] == '?')
            {
                var symbol = rest[pos].ToString();
                pos++;
                if (symbol == "!" && pos < len && rest[pos] == '!')
                {
                    symbol = "!!";
                    pos++;
                }
                // Attach cascade to last attribute
                if (attrs.Count > 0)
                {
                    attrs[^1]["cascade"] = symbol;
                }
                SkipWs();
                continue;
            }

            pos++;
            int nameStart = pos;
            while (pos < len && char.IsLetterOrDigit(rest[pos]) || (pos < len && rest[pos] == '_')) pos++;
            var attrName = rest[nameStart..pos];
            string? args = null;
            if (pos < len && rest[pos] == '(')
            {
                var closeP = FindBalancedParen(rest, pos);
                if (closeP >= 0)
                {
                    args = rest[(pos + 1)..closeP];
                    pos = closeP + 1;
                }
            }
            var attr = new Dictionary<string, object?> { ["name"] = attrName };
            if (args != null) attr["args"] = args;
            attrs.Add(attr);
            SkipWs();
        }
        if (attrs.Count > 0) data["attributes"] = attrs;

        // Trailing description
        SkipWs();
        if (pos < len && rest[pos] == '"')
        {
            var closeIdx = FindClosingQuote(rest, pos);
            if (closeIdx >= 0)
                data["description"] = rest[(pos + 1)..closeIdx];
        }
    }

    private static int FindBalancedParen(string str, int openPos)
    {
        int depth = 0;
        for (int i = openPos; i < str.Length; i++)
        {
            if (str[i] == '(') depth++;
            else if (str[i] == ')')
            {
                depth--;
                if (depth == 0) return i;
            }
            else if (str[i] == '"')
            {
                var closeQ = FindClosingQuote(str, i);
                if (closeQ >= 0) i = closeQ;
            }
            else if (str[i] == '\'')
            {
                var closeQ = str.IndexOf('\'', i + 1);
                if (closeQ >= 0) i = closeQ;
            }
        }
        return -1;
    }

    private static int FindClosingQuote(string str, int openPos)
    {
        for (int i = openPos + 1; i < str.Length; i++)
        {
            if (str[i] == '\\') { i++; continue; }
            if (str[i] == '"') return i;
        }
        return -1;
    }

    private static List<Dictionary<string, object?>> ParseAttributesBalanced(string content)
    {
        var attrs = new List<Dictionary<string, object?>>();
        int pos = 0;
        int len = content.Length;
        while (pos < len)
        {
            var atIdx = content.IndexOf('@', pos);
            if (atIdx < 0) break;
            pos = atIdx + 1;
            int nameStart = pos;
            while (pos < len && (char.IsLetterOrDigit(content[pos]) || content[pos] == '_')) pos++;
            var name = content[nameStart..pos];
            if (name.Length == 0) continue;
            string? args = null;
            if (pos < len && content[pos] == '(')
            {
                var closeP = FindBalancedParen(content, pos);
                if (closeP >= 0)
                {
                    args = content[(pos + 1)..closeP];
                    pos = closeP + 1;
                }
            }
            var attr = new Dictionary<string, object?> { ["name"] = name };
            if (args != null) attr["args"] = args;
            attrs.Add(attr);
        }
        return attrs;
    }

    private static Dictionary<string, object?> ParseNestedItem(string content)
    {
        var kv = Regex.Match(content, @"^([\w]+)\s*:\s*(.+)$");
        if (kv.Success)
            return new() { ["key"] = kv.Groups[1].Value, ["value"] = kv.Groups[2].Value.Trim(), ["raw_content"] = content };
        return new() { ["raw_content"] = content };
    }
}
