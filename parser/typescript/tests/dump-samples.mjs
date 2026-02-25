/**
 * dump-samples.mjs
 *
 * Parses all 5 M3L sample files with the TypeScript parser and dumps
 * the resulting ASTs as JSON files for comparison.
 *
 * Usage: node tests/dump-samples.mjs
 */

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  parseFileString,
  resolve as resolveAST,
} from '../dist/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const OUTPUT_DIR = resolve(__dirname, 'conformance-output');
mkdirSync(OUTPUT_DIR, { recursive: true });

// Define sample files to parse
const SAMPLES_DIR = resolve(__dirname, '../../../samples');

const singleFiles = [
  { path: resolve(SAMPLES_DIR, '01-ecommerce.m3l.md'), name: '01-ecommerce' },
  { path: resolve(SAMPLES_DIR, '02-blog-cms.m3l.md'), name: '02-blog-cms' },
  { path: resolve(SAMPLES_DIR, '03-types-showcase.m3l.md'), name: '03-types-showcase' },
];

const multiFiles = {
  name: '04-multi-file',
  files: [
    resolve(SAMPLES_DIR, 'multi/base.m3l.md'),
    resolve(SAMPLES_DIR, 'multi/inventory.m3l.md'),
  ],
};

function printSummary(label, ast) {
  const modelCount = ast.models?.length ?? 0;
  const enumCount = ast.enums?.length ?? 0;
  const interfaceCount = ast.interfaces?.length ?? 0;
  const viewCount = ast.views?.length ?? 0;
  const errorCount = ast.errors?.length ?? 0;
  const warningCount = ast.warnings?.length ?? 0;

  console.log(`  [${label}]`);
  console.log(`    Models:     ${modelCount}`);
  console.log(`    Enums:      ${enumCount}`);
  console.log(`    Interfaces: ${interfaceCount}`);
  console.log(`    Views:      ${viewCount}`);
  console.log(`    Errors:     ${errorCount}`);
  console.log(`    Warnings:   ${warningCount}`);

  if (errorCount > 0) {
    for (const err of ast.errors) {
      console.log(`      ERROR ${err.code}: ${err.message} (${err.file}:${err.line})`);
    }
  }
  if (warningCount > 0) {
    for (const warn of ast.warnings) {
      console.log(`      WARN ${warn.code}: ${warn.message} (${warn.file}:${warn.line})`);
    }
  }
  console.log('');
}

console.log('=== M3L Sample AST Dump ===\n');

// Parse single files
for (const sample of singleFiles) {
  const content = readFileSync(sample.path, 'utf-8');
  const parsed = parseFileString(content, sample.path);
  const ast = resolveAST([parsed]);

  const outPath = resolve(OUTPUT_DIR, `${sample.name}.json`);
  writeFileSync(outPath, JSON.stringify(ast, null, 2), 'utf-8');
  console.log(`Written: ${outPath}`);
  printSummary(sample.name, ast);
}

// Parse multi-file (both files parsed separately, then resolved together)
{
  const parsedFiles = multiFiles.files.map(filePath => {
    const content = readFileSync(filePath, 'utf-8');
    return parseFileString(content, filePath);
  });
  const ast = resolveAST(parsedFiles);

  const outPath = resolve(OUTPUT_DIR, `${multiFiles.name}.json`);
  writeFileSync(outPath, JSON.stringify(ast, null, 2), 'utf-8');
  console.log(`Written: ${outPath}`);
  printSummary(multiFiles.name, ast);
}

console.log('=== Done ===');
