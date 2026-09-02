import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";

const ENV_KEYS = ["ROOT_DIRECTORY", "DTCT_DIRECTORY", "NGIN_DIRECTORY", "DEF_DIRECTORY"] as const;

type EnvFile = Record<string, string>;

export async function listProject(root: string) {
  const env = await readEnv(path.join(root, ".env"));
  const rootDir = resolveDir(root, env.ROOT_DIRECTORY ?? ".");
  const dtctDir = resolveDir(rootDir, env.DTCT_DIRECTORY ?? ".");
  const nginDir = resolveDir(rootDir, env.NGIN_DIRECTORY ?? ".");
  const defDir = resolveDir(rootDir, env.DEF_DIRECTORY ?? ".");
  const dtct = await listSuffix(dtctDir, ".dtct");
  const ngin = await listSuffix(nginDir, ".ngin");
  const defNgin = (await listSuffix(defDir, ".ngin")).filter((file) => file.endsWith(".def.ngin"));
  const keywords = path.join(dtctDir, "keywords.md");
  const hasKeywords = await fileExists(keywords);
  return {
    root: rootDir,
    env: {
      ROOT_DIRECTORY: env.ROOT_DIRECTORY ?? ".",
      DTCT_DIRECTORY: env.DTCT_DIRECTORY ?? ".",
      NGIN_DIRECTORY: env.NGIN_DIRECTORY ?? ".",
      DEF_DIRECTORY: env.DEF_DIRECTORY ?? ".",
    },
    dtct_dir: dtctDir,
    ngin_dir: nginDir,
    def_dir: defDir,
    dtct,
    ngin: ngin.filter((file) => !file.endsWith(".def.ngin")),
    def_ngin: defNgin,
    keywords: hasKeywords ? keywords : null,
  };
}

async function readEnv(file: string): Promise<EnvFile> {
  if (!(await fileExists(file))) {
    return {};
  } else {
    const env: EnvFile = {};
    for (const line of (await readFile(file, "utf8")).split(/\r?\n/)) {
      const trimmed = line.trim();
      if (trimmed.length === 0 || trimmed.startsWith("#")) {
        continue;
      } else {
        const eq = trimmed.indexOf("=");
        if (eq > 0) {
          const key = trimmed.slice(0, eq).trim();
          const value = trimmed.slice(eq + 1).trim();
          if ((ENV_KEYS as readonly string[]).includes(key) && value.length > 0) {
            env[key] = value;
          }
        }
      }
    }
    return env;
  }
}

function resolveDir(base: string, rel: string): string {
  if (path.isAbsolute(rel)) {
    return rel;
  } else if (rel === "" || rel === ".") {
    return base;
  } else {
    return path.join(base, rel);
  }
}

async function listSuffix(dir: string, suffix: string): Promise<string[]> {
  if (!(await dirExists(dir))) {
    return [];
  } else {
    const out: string[] = [];
    await walk(dir, suffix, out);
    out.sort();
    return out;
  }
}

async function walk(dir: string, suffix: string, out: string[]): Promise<void> {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      await walk(full, suffix, out);
    } else if (entry.isFile() && entry.name.endsWith(suffix)) {
      out.push(full);
    }
  }
}

async function fileExists(file: string): Promise<boolean> {
  try {
    return (await stat(file)).isFile();
  } catch {
    return false;
  }
}

async function dirExists(dir: string): Promise<boolean> {
  try {
    return (await stat(dir)).isDirectory();
  } catch {
    return false;
  }
}
