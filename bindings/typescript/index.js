/**
 * @iyulab/m3l — M3L parser (Rust native via NAPI)
 *
 * Thin wrapper that re-exports the native NAPI addon.
 * All parsing is performed by the Rust m3l-core library.
 */

// Re-exported wholesale rather than named one by one. The enumerated form shipped a
// release where the native addon exported a function this file did not, so the package
// installed cleanly and the function was simply absent — the addon's export set is the
// contract, and restating it here was only a way to fall behind it. index.d.ts still
// declares the surface by hand, because types have no runtime to read them from.
module.exports = require('@iyulab/m3l-napi');
