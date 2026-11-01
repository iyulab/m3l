/**
 * @iyulab/m3l — ES module entry.
 *
 * The CommonJS entry (`index.js`) re-exports the native addon wholesale, which is the right
 * shape for `require` but gives an ES module no names to import: Node learns a CommonJS
 * module's named exports by reading its source, and `module.exports = <the addon>` names
 * none. So `import { parse } from '@iyulab/m3l'` failed while the types promised it.
 *
 * The names below are therefore written out, once, for ES modules only. That is the
 * enumeration the CommonJS entry avoids because it fell behind the addon before, so it is
 * not left to review: `test/exports.test.mjs` fails when the addon's export table, this
 * file and `index.d.ts` stop naming the same functions.
 */
import addon from './index.js';

export const { parse, parseMulti, validate, validateMulti, lint } = addon;

export default addon;
