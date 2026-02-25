import { describe, it, expect } from 'vitest';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { runCLI } from '../src/cli.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturesDir = join(__dirname, 'fixtures');

describe('cli', () => {
  it('should parse a file and output valid JSON', async () => {
    const output = await runCLI(['parse', join(fixturesDir, 'simple.m3l.md')]);
    const ast = JSON.parse(output);
    expect(ast.models).toBeDefined();
    expect(ast.enums).toBeDefined();
    expect(ast.sources).toBeDefined();
  });

  it('should parse a directory', async () => {
    const output = await runCLI(['parse', join(fixturesDir, 'multi')]);
    const ast = JSON.parse(output);
    expect(ast.sources.length).toBeGreaterThanOrEqual(2);
    expect(ast.models.length).toBeGreaterThanOrEqual(2);
  });

  it('should validate and show human-readable output', async () => {
    const output = await runCLI(['validate', join(fixturesDir, 'simple.m3l.md')]);
    expect(output).toContain('0 error');
  });

  it('should validate with --format json', async () => {
    const output = await runCLI(['validate', join(fixturesDir, 'simple.m3l.md'), '--format', 'json']);
    const result = JSON.parse(output);
    expect(result.summary).toBeDefined();
    expect(result.summary.errors).toBe(0);
  });

  it('should validate with --strict flag', async () => {
    const output = await runCLI(['validate', join(fixturesDir, 'simple.m3l.md'), '--strict', '--format', 'json']);
    const result = JSON.parse(output);
    expect(result.summary).toBeDefined();
  });
});
