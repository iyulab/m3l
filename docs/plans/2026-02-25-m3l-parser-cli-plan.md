# M3L Parser & CLI — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** TypeScript로 M3L 파서 라이브러리와 CLI 도구를 구축하여, `.m3l.md` 파일을 JSON AST로 변환하고 검증한다.

**Architecture:** 마크다운 기반 M3L 문서를 Lexer → Parser → Resolver → Validator 파이프라인으로 처리. 멀티 파일 프로젝트를 단일 병합 AST로 출력. CLI와 라이브러리 API를 단일 npm 패키지로 제공.

**Tech Stack:** TypeScript 5.x, Node.js, vitest, commander (CLI)

---

## Task 1: Project Scaffolding

**Files:**
- Create: `parser/package.json`
- Create: `parser/tsconfig.json`
- Create: `parser/vitest.config.ts`
- Create: `parser/src/index.ts`

**Step 1: Create package.json**

```json
{
  "name": "@iyulab/m3l",
  "version": "0.1.0",
  "description": "M3L parser and CLI tool",
  "type": "module",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "bin": { "m3l": "dist/cli.js" },
  "scripts": {
    "build": "tsc",
    "test": "vitest run",
    "test:watch": "vitest",
    "lint": "tsc --noEmit"
  },
  "dependencies": {
    "commander": "^13.0.0",
    "yaml": "^2.7.0",
    "fast-glob": "^3.3.0"
  },
  "devDependencies": {
    "typescript": "^5.7.0",
    "vitest": "^3.0.0",
    "@types/node": "^22.0.0"
  },
  "engines": { "node": ">=20" },
  "files": ["dist"],
  "license": "MIT"
}
```

**Step 2: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "Node16",
    "moduleResolution": "Node16",
    "outDir": "dist",
    "rootDir": "src",
    "declaration": true,
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["src"],
  "exclude": ["node_modules", "dist", "tests"]
}
```

**Step 3: Create vitest.config.ts**

```typescript
import { defineConfig } from 'vitest/config';
export default defineConfig({
  test: {
    include: ['tests/**/*.test.ts'],
  },
});
```

**Step 4: Create stub index.ts**

```typescript
export { lex } from './lexer.js';
export { parse, parseString } from './parser.js';
export type * from './types.js';
```

**Step 5: Run npm install**

```bash
cd parser && npm install
```

Expected: `node_modules/` created, `package-lock.json` generated.

**Step 6: Commit**

```bash
git add parser/package.json parser/tsconfig.json parser/vitest.config.ts parser/src/index.ts
git commit -m "feat(parser): scaffold project with TypeScript, vitest, commander"
```

---

## Task 2: AST Type Definitions

**Files:**
- Create: `parser/src/types.ts`
- Test: `parser/tests/types.test.ts`

**Step 1: Write type definitions**

Core types for the entire AST:

```typescript
// Source location for error reporting
export interface SourceLocation {
  file: string;
  line: number;
  col: number;
}

// --- Token types ---
export type TokenType =
  | 'namespace'        // # Namespace: ...  or  # Title
  | 'model'            // ## ModelName : Parents
  | 'enum'             // ## EnumName ::enum
  | 'interface'        // ## InterfaceName ::interface
  | 'view'             // ## ViewName ::view
  | 'section'          // ### SectionName
  | 'field'            // - fieldName: type @attrs
  | 'enum_value'       // - value: "desc" (inside enum)
  | 'nested_item'      // - key: value (nested under field)
  | 'blockquote'       // > description text
  | 'horizontal_rule'  // ---
  | 'blank'            // empty line
  | 'text';            // plain text (model description)

export interface Token {
  type: TokenType;
  raw: string;
  line: number;
  indent: number;
  data?: Record<string, unknown>;
}

// --- AST types ---
export type FieldKind = 'stored' | 'computed' | 'lookup' | 'rollup';

export interface FieldAttribute {
  name: string;
  args?: unknown[];
}

export interface EnumValue {
  name: string;
  description?: string;
  type?: string;
  value?: unknown;
}

export interface FieldNode {
  name: string;
  label?: string;
  type?: string;
  params?: (string | number)[];
  nullable: boolean;
  array: boolean;
  kind: FieldKind;
  default_value?: string;
  description?: string;
  attributes: FieldAttribute[];
  framework_attrs?: string[];
  // Derived field specifics
  lookup?: { path: string };
  rollup?: { target: string; fk: string; aggregate: string; field?: string; where?: string };
  computed?: { expression: string };
  // Inline enum
  enum_values?: EnumValue[];
  // Nested object fields
  fields?: FieldNode[];
  // Source location
  loc: SourceLocation;
}

export interface SectionNode {
  name: string;
  items: unknown[];
  loc: SourceLocation;
}

export interface ModelNode {
  name: string;
  label?: string;
  type: 'model' | 'enum' | 'interface' | 'view';
  source: string;
  line: number;
  inherits: string[];
  description?: string;
  fields: FieldNode[];
  sections: {
    indexes: unknown[];
    relations: unknown[];
    behaviors: unknown[];
    metadata: Record<string, unknown>;
    [key: string]: unknown;
  };
  // View-specific
  materialized?: boolean;
  source_def?: ViewSourceDef;
  refresh?: { strategy: string; interval?: string };
  loc: SourceLocation;
}

export interface ViewSourceDef {
  from: string;
  joins?: { model: string; on: string }[];
  where?: string;
  order_by?: string;
  group_by?: string[];
}

export interface EnumNode {
  name: string;
  label?: string;
  type: 'enum';
  source: string;
  line: number;
  description?: string;
  values: EnumValue[];
  loc: SourceLocation;
}

export interface ProjectInfo {
  name?: string;
  version?: string;
}

export interface Diagnostic {
  code: string;
  severity: 'error' | 'warning';
  file: string;
  line: number;
  col: number;
  message: string;
}

export interface M3LAST {
  project: ProjectInfo;
  sources: string[];
  models: ModelNode[];
  enums: EnumNode[];
  interfaces: ModelNode[];
  views: ModelNode[];
  errors: Diagnostic[];
  warnings: Diagnostic[];
}
```

**Step 2: Write basic type test**

```typescript
import { describe, it, expect } from 'vitest';
import type { M3LAST, FieldNode, ModelNode } from '../src/types.js';

describe('types', () => {
  it('should allow constructing a minimal AST', () => {
    const ast: M3LAST = {
      project: {},
      sources: [],
      models: [],
      enums: [],
      interfaces: [],
      views: [],
      errors: [],
      warnings: [],
    };
    expect(ast.models).toEqual([]);
  });
});
```

**Step 3: Run test**

```bash
cd parser && npx vitest run tests/types.test.ts
```

Expected: PASS

**Step 4: Commit**

```bash
git add parser/src/types.ts parser/tests/types.test.ts
git commit -m "feat(parser): add AST type definitions"
```

---

## Task 3: Lexer — Tokenize Markdown into M3L Tokens

**Files:**
- Create: `parser/src/lexer.ts`
- Create: `parser/tests/lexer.test.ts`
- Create: `parser/tests/fixtures/simple.m3l.md`

**Step 1: Create test fixture**

`parser/tests/fixtures/simple.m3l.md`:
```markdown
# Library System

## Author : BaseModel
- name(Author Name): string(100) @not_null @idx
- bio(Biography): text?

> Stores author information.

## BookStatus ::enum
- available: "Available"
- borrowed: "Borrowed"
```

**Step 2: Write lexer tests**

```typescript
import { describe, it, expect } from 'vitest';
import { lex } from '../src/lexer.js';

describe('lexer', () => {
  it('should tokenize namespace (H1)', () => {
    const tokens = lex('# Library System\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('namespace');
    expect(tokens[0].data?.name).toBe('Library System');
  });

  it('should tokenize model with inheritance (H2)', () => {
    const tokens = lex('## Author : BaseModel\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('model');
    expect(tokens[0].data?.name).toBe('Author');
    expect(tokens[0].data?.inherits).toEqual(['BaseModel']);
  });

  it('should tokenize enum definition', () => {
    const tokens = lex('## BookStatus ::enum\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('enum');
    expect(tokens[0].data?.name).toBe('BookStatus');
  });

  it('should tokenize view definition', () => {
    const tokens = lex('## OverdueLoans ::view\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('view');
    expect(tokens[0].data?.name).toBe('OverdueLoans');
  });

  it('should tokenize view with materialized', () => {
    const tokens = lex('## PopularBooks ::view @materialized\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('view');
    expect(tokens[0].data?.materialized).toBe(true);
  });

  it('should tokenize field with label, type, attributes', () => {
    const tokens = lex('- name(Author Name): string(100) @not_null @idx\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('field');
    expect(tokens[0].data?.name).toBe('name');
    expect(tokens[0].data?.label).toBe('Author Name');
    expect(tokens[0].data?.type_name).toBe('string');
    expect(tokens[0].data?.type_params).toEqual(['100']);
    expect(tokens[0].data?.nullable).toBe(false);
  });

  it('should tokenize nullable field', () => {
    const tokens = lex('- bio(Biography): text?\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('field');
    expect(tokens[0].data?.nullable).toBe(true);
  });

  it('should tokenize field with default value', () => {
    const tokens = lex('- status: string = "active"\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('field');
    expect(tokens[0].data?.default_value).toBe('"active"');
  });

  it('should tokenize blockquote', () => {
    const tokens = lex('> Stores author information.\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('blockquote');
    expect(tokens[0].data?.text).toBe('Stores author information.');
  });

  it('should tokenize section header (H3)', () => {
    const tokens = lex('### Indexes\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('section');
    expect(tokens[0].data?.name).toBe('Indexes');
  });

  it('should tokenize horizontal rule', () => {
    const tokens = lex('---\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('horizontal_rule');
  });

  it('should tokenize nested item (indented list)', () => {
    const tokens = lex('  - type: string(100)\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('nested_item');
    expect(tokens[0].indent).toBeGreaterThan(0);
  });

  it('should tokenize field with framework attrs in backticks', () => {
    const tokens = lex('- password: string(100) `[JsonIgnore]`\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('field');
    expect(tokens[0].data?.framework_attrs).toContain('[JsonIgnore]');
  });

  it('should tokenize enum value', () => {
    const tokens = lex('- active: "Active account"\n', 'test.m3l.md');
    // Enum values look like fields but have string-only "type" —
    // context determines interpretation in parser
    expect(tokens[0].type).toBe('field');
  });

  it('should handle inline enum field with type', () => {
    const tokens = lex('- status(Status): enum = "available"\n', 'test.m3l.md');
    expect(tokens[0].type).toBe('field');
    expect(tokens[0].data?.type_name).toBe('enum');
    expect(tokens[0].data?.default_value).toBe('"available"');
  });

  it('should tokenize full fixture file', () => {
    const content = [
      '# Library System',
      '',
      '## Author : BaseModel',
      '- name(Author Name): string(100) @not_null @idx',
      '- bio(Biography): text?',
      '',
      '> Stores author information.',
      '',
      '## BookStatus ::enum',
      '- available: "Available"',
      '- borrowed: "Borrowed"',
    ].join('\n');
    const tokens = lex(content, 'test.m3l.md');
    const types = tokens.filter(t => t.type !== 'blank').map(t => t.type);
    expect(types).toEqual([
      'namespace', 'model', 'field', 'field', 'blockquote', 'enum', 'field', 'field'
    ]);
  });
});
```

**Step 3: Implement lexer**

The lexer processes each line and produces tokens. It recognizes:
- `# ...` → namespace
- `## Name : Parents` → model
- `## Name ::enum` → enum
- `## Name ::interface` → interface
- `## Name ::view` → view
- `### Section` → section
- `- field: type @attr` → field
- `  - key: value` → nested_item
- `> text` → blockquote
- `---` → horizontal_rule
- blank lines → blank
- other → text

Key regex patterns:
```
H1:       /^# (.+)$/
H2_TYPED: /^## (\w[\w.]*(?:\(.*?\))?)(?:\s*::(\w+))?(?:\s+(.+))?$/
H2_MODEL: /^## (\w[\w.]*(?:\(.*?\))?)(?:\s*:\s*(.+))?$/
H3:       /^### (.+)$/
FIELD:    /^(\s*)- (.+)$/
QUOTE:    /^> (.+)$/
HR:       /^---+$/
```

Field line sub-parsing:
```
FIELD_FULL: /^(\w+)(?:\(([^)]*)\))?(?:\s*:\s*(.+))?$/
```

Within the type+attrs portion:
- Type: `(\w+)(?:\(([^)]*)\))?(\?)?(\[\])?`
- Default: `= (.+?)(?=\s+@|\s+`|$)`
- Attributes: `@(\w+)(?:\(([^)]*)\))?`
- Framework: `` `\[([^\]]+)\]` ``
- Description: `"([^"]+)"`

**Step 4: Run tests**

```bash
cd parser && npx vitest run tests/lexer.test.ts
```

Expected: All PASS

**Step 5: Commit**

```bash
git add parser/src/lexer.ts parser/tests/lexer.test.ts parser/tests/fixtures/simple.m3l.md
git commit -m "feat(parser): implement lexer for M3L tokenization"
```

---

## Task 4: Parser — Tokens to Single-File AST

**Files:**
- Create: `parser/src/parser.ts`
- Create: `parser/tests/parser.test.ts`
- Create: `parser/tests/fixtures/library.m3l.md` (copy of samples/sample_library.m3l.md)

**Step 1: Write parser tests**

```typescript
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { lex } from '../src/lexer.js';
import { parseTokens } from '../src/parser.js';

describe('parser', () => {
  it('should parse a simple model', () => {
    const tokens = lex([
      '# Test',
      '## User : BaseModel',
      '- name: string(100) @not_null',
      '- email: string(320)? @unique',
      '> User account',
    ].join('\n'), 'test.m3l.md');
    const result = parseTokens(tokens, 'test.m3l.md');
    expect(result.models).toHaveLength(1);
    const user = result.models[0];
    expect(user.name).toBe('User');
    expect(user.inherits).toEqual(['BaseModel']);
    expect(user.fields).toHaveLength(2);
    expect(user.fields[0].name).toBe('name');
    expect(user.fields[0].nullable).toBe(false);
    expect(user.fields[1].nullable).toBe(true);
    expect(user.description).toBe('User account');
  });

  it('should parse an enum', () => {
    const tokens = lex([
      '## Status ::enum',
      '- active: "Active"',
      '- inactive: "Inactive"',
    ].join('\n'), 'test.m3l.md');
    const result = parseTokens(tokens, 'test.m3l.md');
    expect(result.enums).toHaveLength(1);
    expect(result.enums[0].values).toHaveLength(2);
    expect(result.enums[0].values[0].name).toBe('active');
  });

  it('should parse inline enum on a field', () => {
    const tokens = lex([
      '## Order',
      '- status: enum = "pending"',
      '  - pending: "Pending"',
      '  - shipped: "Shipped"',
    ].join('\n'), 'test.m3l.md');
    const result = parseTokens(tokens, 'test.m3l.md');
    const field = result.models[0].fields[0];
    expect(field.type).toBe('enum');
    expect(field.enum_values).toHaveLength(2);
  });

  it('should parse sections (### Indexes, Relations, Metadata)', () => {
    const tokens = lex([
      '## Product',
      '- name: string(200)',
      '### Indexes',
      '- name_search(Name Search)',
      '  - fields: [name]',
      '  - fulltext: true',
      '### Metadata',
      '- domain: "retail"',
    ].join('\n'), 'test.m3l.md');
    const result = parseTokens(tokens, 'test.m3l.md');
    const product = result.models[0];
    expect(product.sections.indexes).toHaveLength(1);
    expect(product.sections.metadata).toHaveProperty('domain', 'retail');
  });

  it('should parse view with source section', () => {
    const tokens = lex([
      '## OverdueLoans ::view',
      '> Currently overdue loans',
      '### Source',
      '- from: Loan',
      '- where: "status = \'ongoing\'"',
      '- order_by: due_date asc',
      '- book_title: string @lookup(book_id.title)',
    ].join('\n'), 'test.m3l.md');
    const result = parseTokens(tokens, 'test.m3l.md');
    expect(result.views).toHaveLength(1);
    const view = result.views[0];
    expect(view.source_def?.from).toBe('Loan');
    expect(view.fields).toHaveLength(1);
  });

  it('should parse lookup field', () => {
    const tokens = lex([
      '## Book',
      '- publisher_id: identifier @fk(Publisher.id)',
      '# Lookup',
      '- publisher_name: string @lookup(publisher_id.name)',
    ].join('\n'), 'test.m3l.md');
    const result = parseTokens(tokens, 'test.m3l.md');
    const lookupField = result.models[0].fields.find(f => f.name === 'publisher_name');
    expect(lookupField?.kind).toBe('lookup');
    expect(lookupField?.lookup?.path).toBe('publisher_id.name');
  });

  it('should parse rollup field', () => {
    const tokens = lex([
      '## Author',
      '- name: string(100)',
      '# Rollup',
      '- book_count: integer @rollup(BookAuthor.author_id, count)',
    ].join('\n'), 'test.m3l.md');
    const result = parseTokens(tokens, 'test.m3l.md');
    const rollup = result.models[0].fields.find(f => f.name === 'book_count');
    expect(rollup?.kind).toBe('rollup');
    expect(rollup?.rollup?.target).toBe('BookAuthor');
    expect(rollup?.rollup?.fk).toBe('author_id');
    expect(rollup?.rollup?.aggregate).toBe('count');
  });

  it('should parse computed field', () => {
    const tokens = lex([
      '## Book',
      '# Computed',
      '- is_available: boolean @computed("status = \'available\'")',
    ].join('\n'), 'test.m3l.md');
    const result = parseTokens(tokens, 'test.m3l.md');
    const computed = result.models[0].fields.find(f => f.name === 'is_available');
    expect(computed?.kind).toBe('computed');
    expect(computed?.computed?.expression).toBe("status = 'available'");
  });

  it('should handle H1 sections (# Lookup, # Rollup, # Computed) as field kind context', () => {
    const tokens = lex([
      '## Author : BaseModel',
      '- name: string(100)',
      '# Rollup',
      '- book_count: integer @rollup(BookAuthor.author_id, count)',
      '# Lookup',
      '## Book',
      '- title: string(200)',
    ].join('\n'), 'test.m3l.md');
    const result = parseTokens(tokens, 'test.m3l.md');
    // # Rollup is a section header within the model, not a namespace
    // # Lookup before ## Book resets context
    expect(result.models).toHaveLength(2);
  });

  it('should parse materialized view with refresh', () => {
    const tokens = lex([
      '## PopularBooks ::view @materialized',
      '### Source',
      '- from: Book',
      '### Refresh',
      '- strategy: scheduled',
      '- interval: "daily 03:00"',
      '- title: string @from(Book.title)',
    ].join('\n'), 'test.m3l.md');
    const result = parseTokens(tokens, 'test.m3l.md');
    const view = result.views[0];
    expect(view.materialized).toBe(true);
    expect(view.refresh?.strategy).toBe('scheduled');
  });
});
```

**Step 2: Implement parser**

The parser walks through tokens sequentially, maintaining state:
- Current model/enum/view being built
- Current section (### header) context
- Current field kind context (# Lookup, # Rollup, # Computed within a model)

Key parsing logic:
1. `namespace` token → set project name or namespace
2. `model/enum/interface/view` token → finalize previous element, start new one
3. `section` token → switch current section context (indexes, relations, metadata, source, refresh)
4. `field` token → add to current element's fields (respecting kind context)
5. `nested_item` token → add to last field (inline enum values, extended format, section detail)
6. `blockquote` token → set description on current element
7. `horizontal_rule` → ignore (whitespace)
8. `text` → set description on current model if no fields yet

Special handling:
- `# Lookup`, `# Rollup`, `# Computed` — these use H1 but are section markers WITHIN a model (between ## headers). The parser must recognize them as kind-context setters, not namespace definitions.
- View `### Source` section — the `from:`, `where:`, `join:`, `order_by:`, `group_by:` items become `source_def`. Fields listed AFTER source directives (no `from:`/`where:` prefix) become view fields.
- Inline enum — when a field has type `enum` and nested items follow, collect them as `enum_values`.

**Step 3: Run tests**

```bash
cd parser && npx vitest run tests/parser.test.ts
```

Expected: All PASS

**Step 4: Commit**

```bash
git add parser/src/parser.ts parser/tests/parser.test.ts
git commit -m "feat(parser): implement single-file parser (tokens → AST)"
```

---

## Task 5: Reader — File Collection

**Files:**
- Create: `parser/src/reader.ts`
- Create: `parser/tests/reader.test.ts`
- Create: `parser/tests/fixtures/multi/a.m3l.md`
- Create: `parser/tests/fixtures/multi/b.m3l.md`

**Step 1: Write reader tests**

```typescript
import { describe, it, expect } from 'vitest';
import { readM3LFiles, readM3LString } from '../src/reader.js';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturesDir = join(__dirname, 'fixtures');

describe('reader', () => {
  it('should read a single file', async () => {
    const files = await readM3LFiles(join(fixturesDir, 'simple.m3l.md'));
    expect(files).toHaveLength(1);
    expect(files[0].path).toContain('simple.m3l.md');
    expect(files[0].content).toContain('# Library System');
  });

  it('should scan a directory for .m3l.md files', async () => {
    const files = await readM3LFiles(join(fixturesDir, 'multi'));
    expect(files.length).toBeGreaterThanOrEqual(2);
  });

  it('should read string content', () => {
    const file = readM3LString('# Test\n## Model\n- name: string', 'inline.m3l.md');
    expect(file.path).toBe('inline.m3l.md');
    expect(file.content).toContain('# Test');
  });
});
```

**Step 2: Implement reader**

The reader:
1. Single `.m3l.md` file → return `[{ path, content }]`
2. Directory → glob `**/*.m3l.md` using fast-glob
3. String content → wrap as file object
4. Config file (`m3l.config.yaml`) → read sources patterns, glob each

```typescript
export interface M3LFile {
  path: string;
  content: string;
}
```

**Step 3: Create multi-file fixtures**

`parser/tests/fixtures/multi/a.m3l.md`:
```markdown
# Common

## BaseModel
- id: identifier @pk @auto_increment
- created_at: timestamp = now()
```

`parser/tests/fixtures/multi/b.m3l.md`:
```markdown
## User : BaseModel
- name: string(100) @not_null
- email: string(320) @unique
```

**Step 4: Run tests**

```bash
cd parser && npx vitest run tests/reader.test.ts
```

Expected: All PASS

**Step 5: Commit**

```bash
git add parser/src/reader.ts parser/tests/reader.test.ts parser/tests/fixtures/multi/
git commit -m "feat(parser): implement file reader with directory scan"
```

---

## Task 6: Resolver — Multi-File AST Merge

**Files:**
- Create: `parser/src/resolver.ts`
- Create: `parser/tests/resolver.test.ts`

**Step 1: Write resolver tests**

```typescript
import { describe, it, expect } from 'vitest';
import { lex } from '../src/lexer.js';
import { parseTokens } from '../src/parser.js';
import { resolve } from '../src/resolver.js';

function parseSource(content: string, file: string) {
  const tokens = lex(content, file);
  return parseTokens(tokens, file);
}

describe('resolver', () => {
  it('should merge multiple file ASTs', () => {
    const a = parseSource('## BaseModel\n- id: identifier @pk', 'a.m3l.md');
    const b = parseSource('## User : BaseModel\n- name: string(100)', 'b.m3l.md');
    const merged = resolve([a, b]);
    expect(merged.models).toHaveLength(2);
    expect(merged.sources).toEqual(['a.m3l.md', 'b.m3l.md']);
  });

  it('should resolve inheritance (copy parent fields)', () => {
    const a = parseSource('## BaseModel\n- id: identifier @pk\n- created_at: timestamp = now()', 'a.m3l.md');
    const b = parseSource('## User : BaseModel\n- name: string(100)', 'b.m3l.md');
    const merged = resolve([a, b]);
    const user = merged.models.find(m => m.name === 'User')!;
    // User should have inherited fields + own fields
    expect(user.fields.length).toBeGreaterThanOrEqual(3);
  });

  it('should detect duplicate model names as error', () => {
    const a = parseSource('## User\n- name: string', 'a.m3l.md');
    const b = parseSource('## User\n- email: string', 'b.m3l.md');
    const merged = resolve([a, b]);
    expect(merged.errors.some(e => e.code === 'E005')).toBe(true);
  });

  it('should categorize enums, interfaces, views into separate arrays', () => {
    const src = parseSource([
      '## Status ::enum',
      '- active: "Active"',
      '## Searchable ::interface',
      '- search_text: text',
      '## Dashboard ::view',
      '### Source',
      '- from: User',
    ].join('\n'), 'test.m3l.md');
    const merged = resolve([src]);
    expect(merged.enums).toHaveLength(1);
    expect(merged.interfaces).toHaveLength(1);
    expect(merged.views).toHaveLength(1);
  });

  it('should report unresolved inheritance references', () => {
    const src = parseSource('## User : NonExistent\n- name: string', 'test.m3l.md');
    const merged = resolve([src]);
    expect(merged.errors.some(e => e.code === 'E007')).toBe(true);
  });
});
```

**Step 2: Implement resolver**

The resolver:
1. Collects all models/enums/interfaces/views from all file ASTs
2. Builds a name → node map
3. Resolves inheritance: copies parent fields into child (respecting order)
4. Detects duplicate names (E005)
5. Detects unresolved references (E007)
6. Produces single merged `M3LAST`

**Step 3: Run tests**

```bash
cd parser && npx vitest run tests/resolver.test.ts
```

Expected: All PASS

**Step 4: Commit**

```bash
git add parser/src/resolver.ts parser/tests/resolver.test.ts
git commit -m "feat(parser): implement multi-file resolver with inheritance"
```

---

## Task 7: Validator — Semantic Validation

**Files:**
- Create: `parser/src/validator.ts`
- Create: `parser/tests/validator.test.ts`

**Step 1: Write validator tests**

```typescript
import { describe, it, expect } from 'vitest';
import { lex } from '../src/lexer.js';
import { parseTokens } from '../src/parser.js';
import { resolve } from '../src/resolver.js';
import { validate } from '../src/validator.js';

function buildAST(content: string, file = 'test.m3l.md') {
  const tokens = lex(content, file);
  const parsed = parseTokens(tokens, file);
  return resolve([parsed]);
}

describe('validator', () => {
  it('E001: @rollup FK missing @reference', () => {
    const ast = buildAST([
      '## Author',
      '- name: string(100)',
      '# Rollup',
      '- book_count: integer @rollup(Book.author_id, count)',
      '## Book',
      '- title: string(200)',
      '- author_id: identifier',  // no @fk or @reference
    ].join('\n'));
    const result = validate(ast);
    expect(result.errors.some(e => e.code === 'E001')).toBe(true);
  });

  it('E002: @lookup path FK missing @reference', () => {
    const ast = buildAST([
      '## Order',
      '- customer_id: identifier',  // no @reference
      '# Lookup',
      '- customer_name: string @lookup(customer_id.name)',
    ].join('\n'));
    const result = validate(ast);
    expect(result.errors.some(e => e.code === 'E002')).toBe(true);
  });

  it('E005: duplicate field names within a model', () => {
    const ast = buildAST([
      '## User',
      '- name: string(100)',
      '- name: string(200)',
    ].join('\n'));
    const result = validate(ast);
    expect(result.errors.some(e => e.code === 'E005')).toBe(true);
  });

  it('should pass with valid references', () => {
    const ast = buildAST([
      '## Customer',
      '- id: identifier @pk',
      '- name: string(100)',
      '## Order',
      '- customer_id: identifier @fk(Customer.id)',
      '# Lookup',
      '- customer_name: string @lookup(customer_id.name)',
    ].join('\n'));
    const result = validate(ast);
    expect(result.errors.filter(e => e.code === 'E002')).toHaveLength(0);
  });

  it('W001: field line length >80 chars (strict mode)', () => {
    const longLine = '- very_long_field_name_here: string(200) @not_null @unique @searchable @index "A very long description that exceeds eighty characters"';
    const ast = buildAST(`## Model\n${longLine}`);
    const result = validate(ast, { strict: true });
    expect(result.warnings.some(w => w.code === 'W001')).toBe(true);
  });

  it('should not report warnings without strict mode', () => {
    const longLine = '- very_long_field_name_here: string(200) @not_null @unique @searchable @index "A very long description"';
    const ast = buildAST(`## Model\n${longLine}`);
    const result = validate(ast, { strict: false });
    expect(result.warnings.filter(w => w.code === 'W001')).toHaveLength(0);
  });
});
```

**Step 2: Implement validator**

Validation rules:
- E001: For each @rollup, check that the FK field in the target model has @reference/@fk
- E002: For each @lookup, check that the first segment FK has @reference/@fk
- E003: Circular reference detection (inheritance chain, lookup chain)
- E004: View @from references model not found
- E005: Duplicate model names (already in resolver) + duplicate field names
- E006: @import target not found (future — Reader handles this)
- E007: Unresolved type/model reference (already in resolver)

Strict warnings:
- W001: Field raw line > 80 chars
- W002: Object nesting > 3 levels
- W003: `[Attr]` without backtick
- W004: Lookup chain > 3 hops
- W005: View nesting > 2 levels
- W006: Inline enum missing `values:` key

**Step 3: Run tests**

```bash
cd parser && npx vitest run tests/validator.test.ts
```

Expected: All PASS

**Step 4: Commit**

```bash
git add parser/src/validator.ts parser/tests/validator.test.ts
git commit -m "feat(parser): implement semantic validator with strict mode"
```

---

## Task 8: CLI — Parse and Validate Commands

**Files:**
- Create: `parser/src/cli.ts`
- Create: `parser/tests/cli.test.ts`

**Step 1: Write CLI tests**

```typescript
import { describe, it, expect } from 'vitest';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { runCLI } from '../src/cli.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturesDir = join(__dirname, 'fixtures');

describe('cli', () => {
  it('should parse a file and output JSON', async () => {
    const output = await runCLI(['parse', join(fixturesDir, 'simple.m3l.md')]);
    const ast = JSON.parse(output);
    expect(ast.models).toBeDefined();
  });

  it('should parse a directory', async () => {
    const output = await runCLI(['parse', join(fixturesDir, 'multi')]);
    const ast = JSON.parse(output);
    expect(ast.sources.length).toBeGreaterThanOrEqual(2);
  });

  it('should validate and show human-readable output', async () => {
    const output = await runCLI(['validate', join(fixturesDir, 'simple.m3l.md')]);
    expect(output).toContain('0 errors');
  });

  it('should validate with --format json', async () => {
    const output = await runCLI(['validate', join(fixturesDir, 'simple.m3l.md'), '--format', 'json']);
    const result = JSON.parse(output);
    expect(result.summary).toBeDefined();
  });
});
```

**Step 2: Implement CLI**

```typescript
#!/usr/bin/env node
import { Command } from 'commander';

const program = new Command()
  .name('m3l')
  .description('M3L parser and validator')
  .version('0.1.0');

program
  .command('parse [path]')
  .description('Parse M3L files and output JSON AST')
  .option('-o, --output <file>', 'Write output to file')
  .action(async (path, options) => { /* ... */ });

program
  .command('validate [path]')
  .description('Validate M3L files')
  .option('--strict', 'Enable strict style guidelines')
  .option('--format <format>', 'Output format: human (default) or json')
  .action(async (path, options) => { /* ... */ });
```

Also export `runCLI()` function for testing.

**Step 3: Run tests**

```bash
cd parser && npx vitest run tests/cli.test.ts
```

Expected: All PASS

**Step 4: Commit**

```bash
git add parser/src/cli.ts parser/tests/cli.test.ts
git commit -m "feat(parser): implement CLI with parse and validate commands"
```

---

## Task 9: Library API & Final Integration

**Files:**
- Modify: `parser/src/index.ts`
- Create: `parser/tests/integration.test.ts`

**Step 1: Write integration test with sample_library.m3l.md**

```typescript
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseString } from '../src/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

describe('integration', () => {
  it('should parse the full library sample', () => {
    const content = readFileSync(
      join(__dirname, '../../samples/sample_library.m3l.md'), 'utf-8'
    );
    const ast = parseString(content, 'sample_library.m3l.md');

    // Models: Timestampable, BaseModel, Author, Publisher, Book, BookAuthor,
    //         Category, BookCategory, Member, Loan, Reservation
    expect(ast.models.length).toBeGreaterThanOrEqual(10);

    // Views: OverdueLoans, MemberActivity, PopularBooks
    expect(ast.views).toHaveLength(3);

    // Check a specific lookup field
    const book = ast.models.find(m => m.name === 'Book');
    expect(book).toBeDefined();
    const lookup = book!.fields.find(f => f.name === 'publisher_name');
    expect(lookup?.kind).toBe('lookup');

    // Check a rollup field
    const author = ast.models.find(m => m.name === 'Author');
    const rollup = author!.fields.find(f => f.name === 'book_count');
    expect(rollup?.kind).toBe('rollup');

    // Check materialized view
    const popular = ast.views.find(v => v.name === 'PopularBooks');
    expect(popular?.materialized).toBe(true);

    // No errors expected in a valid sample
    expect(ast.errors).toHaveLength(0);
  });
});
```

**Step 2: Finalize index.ts with public API**

```typescript
export { lex } from './lexer.js';
export { parseTokens } from './parser.js';
export { resolve } from './resolver.js';
export { validate } from './validator.js';
export { readM3LFiles, readM3LString } from './reader.js';
export type * from './types.js';

// High-level API
export async function parse(path: string): Promise<M3LAST> { /* ... */ }
export function parseString(content: string, filename?: string): M3LAST { /* ... */ }
```

**Step 3: Run all tests**

```bash
cd parser && npx vitest run
```

Expected: All PASS

**Step 4: Commit**

```bash
git add parser/src/index.ts parser/tests/integration.test.ts
git commit -m "feat(parser): add public API and integration tests"
```

---

## Task 10: Build Verification & Final Cleanup

**Step 1: Build TypeScript**

```bash
cd parser && npm run build
```

Expected: `dist/` created with `.js` and `.d.ts` files, no errors.

**Step 2: Run full test suite**

```bash
cd parser && npm test
```

Expected: All tests pass.

**Step 3: Test CLI manually**

```bash
cd parser && node dist/cli.js parse ../samples/sample_library.m3l.md | head -50
cd parser && node dist/cli.js validate ../samples/sample_library.m3l.md
```

Expected: Valid JSON output, "0 errors" validation.

**Step 4: Final commit**

```bash
git add -A parser/
git commit -m "feat(parser): complete M3L parser v0.1.0 with CLI"
```
