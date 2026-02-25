import { describe, it, expect } from 'vitest';
import { readM3LFiles, readM3LString } from '../src/reader.js';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturesDir = join(__dirname, 'fixtures');

describe('reader', () => {
  it('should read a single file', async () => {
    const files = await readM3LFiles(join(fixturesDir, 'simple.m3l.md'));
    expect(files).toHaveLength(1);
    expect(files[0].path).toContain('simple.m3l.md');
    expect(files[0].content).toContain('# Library System');
  });

  it('should scan a directory for .m3l.md files', async () => {
    const files = await readM3LFiles(join(fixturesDir, 'multi'));
    expect(files.length).toBeGreaterThanOrEqual(2);
    const paths = files.map(f => f.path);
    expect(paths.some(p => p.includes('a.m3l.md'))).toBe(true);
    expect(paths.some(p => p.includes('b.m3l.md'))).toBe(true);
  });

  it('should read string content', () => {
    const file = readM3LString('# Test\n## Model\n- name: string', 'inline.m3l.md');
    expect(file.path).toBe('inline.m3l.md');
    expect(file.content).toContain('# Test');
  });

  it('should return files sorted by path', async () => {
    const files = await readM3LFiles(join(fixturesDir, 'multi'));
    const paths = files.map(f => f.path);
    const sorted = [...paths].sort();
    expect(paths).toEqual(sorted);
  });
});
