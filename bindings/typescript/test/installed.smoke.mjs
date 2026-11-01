// Run against an INSTALLED copy of the package (a registry install or a packed tarball),
// from a directory that has `@iyulab/m3l` in its node_modules — not from this source tree:
//
//   npm i @iyulab/m3l@<version> && node installed.smoke.mjs
//
// It exercises what a first-time user does: named imports from an ES module, the default
// import, and `require`, then one real parse. Exits non-zero on the first thing that is wrong.

import { createRequire } from 'node:module';
import * as ns from '@iyulab/m3l';
import addon from '@iyulab/m3l';

const names = ['parse', 'parseMulti', 'validate', 'validateMulti', 'lint'];
const cjs = createRequire(import.meta.url)('@iyulab/m3l');

for (const name of names) {
  if (typeof ns[name] !== 'function') throw new Error(`ES module named export missing: ${name}`);
  if (typeof cjs[name] !== 'function') throw new Error(`CommonJS export missing: ${name}`);
  if (addon[name] !== ns[name]) throw new Error(`default and named export differ: ${name}`);
}

const parsed = JSON.parse(ns.parse('# Namespace: demo\n\n## A\n- id: identifier @pk\n', 'a.m3l.md'));
if (!parsed.success || parsed.data.models[0]?.name !== 'A') {
  throw new Error(`parse returned an unexpected result: ${JSON.stringify(parsed).slice(0, 200)}`);
}

console.log(`installed package ok: ${Object.keys(cjs).sort().join(', ')}`);
