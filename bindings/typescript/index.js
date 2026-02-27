/**
 * @iyulab/m3l — M3L parser (Rust native via NAPI)
 *
 * Thin wrapper that re-exports the native NAPI addon.
 * All parsing is performed by the Rust m3l-core library.
 */

const { parse, parseMulti, validate } = require('@iyulab/m3l-napi');

module.exports.parse = parse;
module.exports.parseMulti = parseMulti;
module.exports.validate = validate;
