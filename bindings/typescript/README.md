# @iyulab/m3l

The [M3L](https://github.com/iyulab/m3l) parser for Node.js — read a Markdown-based data model (`.m3l.md`) and get the
model back as JSON, with diagnostics. The parsing is done by the Rust `m3l-core` library through a native addon
(`@iyulab/m3l-napi`, installed automatically for Windows x64, Linux x64 and macOS x64/arm64).

Requires Node.js 22 or later.

```sh
npm install @iyulab/m3l
```

## Usage

The functions take and return **strings**: JSON in, JSON out. Parse the result yourself.

```js
import { parse, validate } from '@iyulab/m3l';

const source = `# Namespace: demo

## Customer
- id: identifier @pk
- name: string(100) @not_null
`;

const parsed = JSON.parse(parse(source, 'demo.m3l.md'));
if (parsed.success) {
  console.log(parsed.data.models.map((m) => m.name)); // [ 'Customer' ]
}

const checked = JSON.parse(validate(source, '{}'));
console.log(checked.data.errors, checked.data.warnings); // [] []
```

CommonJS works the same way:

```js
const { parse } = require('@iyulab/m3l');
```

## Functions

Every function returns a JSON string shaped `{ "success": boolean, "data"?: …, "error"?: string }`. `success: false`
means the call itself failed (for example, the JSON you passed was malformed); problems **in the model** are reported in
`data`, with `success: true`.

| Function | Arguments | `data` |
|---|---|---|
| `parse(content, filename)` | M3L text; a file name used in diagnostics | the AST (`M3lAst`): `models`, `enums`, `interfaces`, `views`, `errors`, `warnings`, … |
| `parseMulti(filesJson)` | JSON array of `{ "content", "filename" }` | the merged AST of all files |
| `validate(content, optionsJson)` | M3L text; JSON options (`{}`; `strict`, `filename`) | `{ "errors": […], "warnings": […] }` |
| `validateMulti(filesJson, optionsJson)` | as `parseMulti`, then options | as `validate`, every diagnostic carries its own `file` |
| `lint(content, configJson)` | M3L text; JSON config (`{}` or `{ "rules": { "<rule>": "off" \| "warn" \| "error" } }`) | `{ "diagnostics": […], "file_count": n }` |

A diagnostic has `code` (for example `M3L-E009`), `severity`, `file`, `line`, `col` and `message`:

```js
const bad = '# Namespace: demo\n\n## Customer\n- id: identifier @pk\n- name: strng(100)\n';
JSON.parse(validate(bad, '{}')).data.errors[0];
// { code: 'M3L-E009', severity: 'error', file: 'input.m3l.md', line: 5, col: 1,
//   message: 'Undefined type "strng" in field "name" of model "Customer"' }
```

Use the multi-file functions when a model spans files: an inherited model, an interface reference or a custom attribute
registered in one file only resolves when the whole set is read together.

## Types

`index.d.ts` declares the argument and result types (`M3lResult<T>`, `M3lAst`, `ValidateResult`, `LintResult`,
`FileInput`, …) for use with `JSON.parse`:

```ts
import { parse, type M3lAst, type M3lResult } from '@iyulab/m3l';

const result = JSON.parse(parse(text, 'a.m3l.md')) as M3lResult<M3lAst>;
```

## More

- The language specification, the command-line tool and the other bindings: <https://github.com/iyulab/m3l>
- License: MIT
