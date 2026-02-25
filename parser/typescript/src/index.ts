export { lex } from './lexer.js';
export { parseTokens, parseString as parseFileString, STANDARD_ATTRIBUTES } from './parser.js';
export { resolve, AST_VERSION, PARSER_VERSION } from './resolver.js';
export { validate } from './validator.js';
export { readM3LFiles, readM3LString, readProjectConfig } from './reader.js';
export type * from './types.js';

import type { M3LAST, ValidateOptions } from './types.js';
import { readM3LFiles, readProjectConfig } from './reader.js';
import { parseString as parseFileContent } from './parser.js';
import { resolve } from './resolver.js';
import { validate as validateAST } from './validator.js';
import { resolve as resolvePath } from 'path';

/**
 * High-level API: Parse M3L files from a path (file or directory) into a merged AST.
 */
export async function parse(inputPath: string): Promise<M3LAST> {
  const resolved = resolvePath(inputPath);
  const files = await readM3LFiles(resolved);

  if (files.length === 0) {
    throw new Error(`No .m3l.md or .m3l files found at: ${inputPath}`);
  }

  const parsedFiles = files.map(f => parseFileContent(f.content, f.path));

  let projectInfo: { name?: string; version?: string } | undefined;
  try {
    const config = await readProjectConfig(resolved);
    if (config) projectInfo = config;
  } catch {
    // No config
  }

  return resolve(parsedFiles, projectInfo || undefined);
}

/**
 * High-level API: Parse M3L content string into AST.
 */
export function parseString(content: string, filename: string = 'inline.m3l.md'): M3LAST {
  const parsed = parseFileContent(content, filename);
  return resolve([parsed]);
}

/**
 * High-level API: Validate M3L files from a path.
 */
export async function validateFiles(
  inputPath: string,
  options?: ValidateOptions
): Promise<{ ast: M3LAST; errors: import('./types.js').Diagnostic[]; warnings: import('./types.js').Diagnostic[] }> {
  const ast = await parse(inputPath);
  const result = validateAST(ast, options);
  return { ast, ...result };
}
