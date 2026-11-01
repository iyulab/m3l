# @iyulab/m3l-napi

The native Node.js addon behind [`@iyulab/m3l`](https://www.npmjs.com/package/@iyulab/m3l) — the
[M3L](https://github.com/iyulab/m3l) parser compiled from the Rust `m3l-core` crate (napi-rs), with one prebuilt binary
per platform (Windows x64, Linux x64, macOS x64 and arm64).

**Install `@iyulab/m3l` instead.** It depends on this package and adds an ES module entry (`import { parse } from
'@iyulab/m3l'`), TypeScript declarations for the argument and result types, and the documentation. This package is a
CommonJS module whose exports are the addon's own function table, so from an ES module it can only be imported as a
default (`import addon from '@iyulab/m3l-napi'`).

The functions are `parse`, `parseMulti`, `validate`, `validateMulti` and `lint`; each takes strings and returns a JSON
string. See the [`@iyulab/m3l` README](https://www.npmjs.com/package/@iyulab/m3l) for what they take and return.
