import { readFileSync, statSync, existsSync } from 'fs';
import { join, resolve as resolvePath } from 'path';
import fg from 'fast-glob';

export interface M3LFile {
  path: string;
  content: string;
}

/**
 * Read M3L files from a path (file or directory).
 * If path is a directory, scans for **\/*.m3l.md files.
 * If an m3l.config.yaml exists in the directory, uses its sources patterns.
 */
export async function readM3LFiles(inputPath: string): Promise<M3LFile[]> {
  const resolved = resolvePath(inputPath);

  const stat = statSync(resolved);

  if (stat.isFile()) {
    return [readSingleFile(resolved)];
  }

  if (stat.isDirectory()) {
    // Check for m3l.config.yaml
    const configPath = join(resolved, 'm3l.config.yaml');
    if (existsSync(configPath)) {
      return readFromConfig(configPath, resolved);
    }

    // Default: scan for all .m3l.md files
    return scanDirectory(resolved);
  }

  throw new Error(`Path is neither a file nor a directory: ${resolved}`);
}

/**
 * Wrap a string content as an M3LFile.
 */
export function readM3LString(content: string, filename: string = 'inline.m3l.md'): M3LFile {
  return { path: filename, content };
}

function readSingleFile(filePath: string): M3LFile {
  const content = readFileSync(filePath, 'utf-8');
  return { path: filePath, content };
}

async function scanDirectory(dirPath: string): Promise<M3LFile[]> {
  const pattern = '**/*.{m3l.md,m3l}';
  const files = await fg(pattern, {
    cwd: dirPath,
    absolute: true,
    onlyFiles: true,
  });

  return files.sort().map(f => readSingleFile(f));
}

async function readFromConfig(configPath: string, baseDir: string): Promise<M3LFile[]> {
  const yamlContent = readFileSync(configPath, 'utf-8');

  // Dynamic import yaml to parse config
  const { parse: parseYaml } = await import('yaml');
  const config = parseYaml(yamlContent) as {
    name?: string;
    version?: string;
    sources?: string[];
  };

  if (!config.sources || config.sources.length === 0) {
    return scanDirectory(baseDir);
  }

  const allFiles: M3LFile[] = [];
  const seen = new Set<string>();

  for (const pattern of config.sources) {
    const files = await fg(pattern, {
      cwd: baseDir,
      absolute: true,
      onlyFiles: true,
    });

    for (const f of files.sort()) {
      if (!seen.has(f)) {
        seen.add(f);
        allFiles.push(readSingleFile(f));
      }
    }
  }

  return allFiles;
}

/**
 * Read project config from m3l.config.yaml if it exists.
 */
export async function readProjectConfig(dirPath: string): Promise<{
  name?: string;
  version?: string;
} | null> {
  const configPath = join(resolvePath(dirPath), 'm3l.config.yaml');
  if (!existsSync(configPath)) return null;

  const yamlContent = readFileSync(configPath, 'utf-8');
  const { parse: parseYaml } = await import('yaml');
  const config = parseYaml(yamlContent);
  return {
    name: config?.name,
    version: config?.version,
  };
}
