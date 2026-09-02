import { spawn } from "node:child_process";

export type CliResult = {
  status: number;
  stdout: string;
  stderr: string;
};

export function datumaKBin(): string {
  return process.env.DATUMA_K ?? "datuma_k";
}

export function runDatumaK(args: string[], cwd: string): Promise<CliResult> {
  return new Promise((resolve, reject) => {
    const child = spawn(datumaKBin(), args, { cwd, env: process.env });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", reject);
    child.on("close", (status) => {
      resolve({ status: status ?? 1, stdout, stderr });
    });
  });
}

export function jsonToolResult(stdout: string, stderr: string, status: number) {
  const trimmed = stdout.trim();
  try {
    const parsed = JSON.parse(trimmed) as { ok?: boolean };
    const failed = status !== 0 || parsed.ok === false;
    return {
      content: [{ type: "text" as const, text: trimmed || stderr || "no output" }],
      isError: failed,
    };
  } catch {
    return {
      content: [
        {
          type: "text" as const,
          text: stderr.trim() || trimmed || `datuma_k exited ${status}`,
        },
      ],
      isError: true,
    };
  }
}
