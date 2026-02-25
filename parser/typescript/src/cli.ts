#!/usr/bin/env node

import { Command } from 'commander';
import { writeFileSync } from 'fs';
import { resolve as resolvePath } from 'path';
import { readM3LFiles, readProjectConfig } from './reader.js';
import { parseString as parseFileContent } from './parser.js';
import { resolve } from './resolver.js';
import { validate } from './validator.js';
import type { M3LAST, Diagnostic, ValidateOptions } from './types.js';

const program = new Command()
  .name('m3l')
  .description('M3L parser and validator — parse .m3l.md files into JSON AST')
  .version('0.1.0');

program
  .command('parse [path]')
  .description('Parse M3L files and output JSON AST')
  .option('-o, --output <file>', 'Write output to file instead of stdout')
  .action(async (inputPath?: string, options?: { output?: string }) => {
    try {
      const output = await runParse(inputPath || '.', options?.output);
      if (!options?.output) {
        process.stdout.write(output + '\n');
      }
    } catch (err) {
      process.stderr.write(`Error: ${(err as Error).message}\n`);
      process.exit(1);
    }
  });

program
  .command('validate [path]')
  .description('Validate M3L files')
  .option('--strict', 'Enable strict style guidelines')
  .option('--format <format>', 'Output format: human (default) or json', 'human')
  .action(async (inputPath?: string, options?: { strict?: boolean; format?: string }) => {
    try {
      const output = await runValidate(
        inputPath || '.',
        { strict: options?.strict },
        options?.format || 'human'
      );
      process.stdout.write(output + '\n');
      // Exit with error code if there are errors
      if (output.includes('"errors":') || output.match(/\d+ error/)) {
        const errorCount = extractErrorCount(output, options?.format || 'human');
        if (errorCount > 0) process.exit(1);
      }
    } catch (err) {
      process.stderr.write(`Error: ${(err as Error).message}\n`);
      process.exit(1);
    }
  });

// Entry point for direct execution
const args = process.argv.slice(2);
if (args.length > 0) {
  program.parse();
}

/**
 * Run parse command — also used for testing.
 */
export async function runParse(inputPath: string, outputFile?: string): Promise<string> {
  const ast = await buildAST(inputPath);
  const json = JSON.stringify(ast, null, 2);

  if (outputFile) {
    writeFileSync(resolvePath(outputFile), json, 'utf-8');
    return `Written to ${outputFile}`;
  }

  return json;
}

/**
 * Run validate command — also used for testing.
 */
export async function runValidate(
  inputPath: string,
  options: ValidateOptions = {},
  format: string = 'human'
): Promise<string> {
  const ast = await buildAST(inputPath);
  const result = validate(ast, options);

  if (format === 'json') {
    return JSON.stringify({
      diagnostics: [...result.errors, ...result.warnings],
      summary: {
        errors: result.errors.length,
        warnings: result.warnings.length,
        files: ast.sources.length,
      },
    }, null, 2);
  }

  // Human-readable format
  return formatHumanDiagnostics(result.errors, result.warnings, ast.sources.length);
}

/**
 * Run CLI with args — for testing.
 */
export async function runCLI(args: string[]): Promise<string> {
  const cmd = args[0];
  const rest = args.slice(1);

  if (cmd === 'parse') {
    const inputPath = rest.find(a => !a.startsWith('-')) || '.';
    const outputIdx = rest.indexOf('-o');
    const outputFile = outputIdx >= 0 ? rest[outputIdx + 1] : undefined;
    const outputIdx2 = rest.indexOf('--output');
    const outputFile2 = outputIdx2 >= 0 ? rest[outputIdx2 + 1] : undefined;
    return runParse(inputPath, outputFile || outputFile2);
  }

  if (cmd === 'validate') {
    const inputPath = rest.find(a => !a.startsWith('-')) || '.';
    const strict = rest.includes('--strict');
    const formatIdx = rest.indexOf('--format');
    const format = formatIdx >= 0 ? rest[formatIdx + 1] : 'human';
    return runValidate(inputPath, { strict }, format);
  }

  throw new Error(`Unknown command: ${cmd}`);
}

// --- Internal ---

async function buildAST(inputPath: string): Promise<M3LAST> {
  const resolved = resolvePath(inputPath);
  const files = await readM3LFiles(resolved);

  if (files.length === 0) {
    throw new Error(`No .m3l.md files found at: ${inputPath}`);
  }

  const parsedFiles = files.map(f => parseFileContent(f.content, f.path));

  // Try to read project config
  let projectInfo: { name?: string; version?: string } | undefined;
  try {
    const config = await readProjectConfig(resolved);
    if (config) projectInfo = config;
  } catch {
    // No config — that's fine
  }

  return resolve(parsedFiles, projectInfo || undefined);
}

function formatHumanDiagnostics(
  errors: Diagnostic[],
  warnings: Diagnostic[],
  fileCount: number
): string {
  const lines: string[] = [];

  for (const d of [...errors, ...warnings]) {
    const severity = d.severity === 'error' ? 'error' : 'warning';
    lines.push(`${d.file}:${d.line}:${d.col} ${severity}[${d.code}]: ${d.message}`);
  }

  lines.push(`${errors.length} error${errors.length !== 1 ? 's' : ''}, ${warnings.length} warning${warnings.length !== 1 ? 's' : ''} in ${fileCount} file${fileCount !== 1 ? 's' : ''}.`);

  return lines.join('\n');
}

function extractErrorCount(output: string, format: string): number {
  if (format === 'json') {
    try {
      const parsed = JSON.parse(output);
      return parsed.summary?.errors || 0;
    } catch {
      return 0;
    }
  }
  const match = output.match(/(\d+) error/);
  return match ? parseInt(match[1], 10) : 0;
}
