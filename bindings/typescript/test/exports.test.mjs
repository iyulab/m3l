// The functions this package exposes are named in four places: the Rust addon (the contract),
// the ES module entry, the declaration file, and the README's table. The CommonJS entry
// re-exports the addon wholesale and names nothing; the other three are written by hand,
// so this test fails when they stop agreeing with the addon source.
//
// It reads sources only — no native build and no installed addon — so it runs anywhere Node does:
//   node --test bindings/typescript/test

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const read = (relative) => readFileSync(fileURLToPath(new URL(relative, import.meta.url)), 'utf8');

const camel = (snake) => snake.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase());

/** JS names of the `#[napi]` functions: `js_name` when given, else the camelCased Rust name. */
function addonFunctions() {
  const source = read('../../../crates/m3l-napi/src/lib.rs');
  const names = [];
  for (const m of source.matchAll(/#\[napi(?:\(([^)]*)\))?\]\s*pub fn (\w+)/g)) {
    const jsName = /js_name\s*=\s*"([^"]+)"/.exec(m[1] ?? '');
    names.push(jsName ? jsName[1] : camel(m[2]));
  }
  return names.sort();
}

test('the addon source exports at least one function (the reader still matches the source)', () => {
  assert.ok(addonFunctions().length >= 5, `read only: ${addonFunctions().join(', ')}`);
});

test('index.mjs names every function the addon exports, and no other', () => {
  const declaration = /export const \{([^}]*)\} = addon;/.exec(read('../index.mjs'));
  assert.ok(declaration, 'index.mjs has no `export const { … } = addon;` line');
  const named = declaration[1].split(',').map((s) => s.trim()).filter(Boolean).sort();
  assert.deepEqual(named, addonFunctions());
});

test('index.d.ts declares every function the addon exports, and no other', () => {
  const declared = [...read('../index.d.ts').matchAll(/^export function (\w+)/gm)].map((m) => m[1]).sort();
  assert.deepEqual(declared, addonFunctions());
});

test('the README documents every function the addon exports', () => {
  const readme = read('../README.md');
  for (const name of addonFunctions()) {
    assert.ok(readme.includes(`\`${name}(`), `README.md has no row for ${name}()`);
  }
});

test('package.json exposes the ES module entry next to the CommonJS one', () => {
  const pkg = JSON.parse(read('../package.json'));
  assert.equal(pkg.exports['.'].import, './index.mjs');
  assert.equal(pkg.exports['.'].require, './index.js');
  for (const file of ['index.js', 'index.mjs', 'index.d.ts']) {
    assert.ok(pkg.files.includes(file), `files omits ${file}`);
  }
});
