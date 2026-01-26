import { readFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import ts from "typescript";
import { load } from "cheerio";

type SourceResult = {
  source: string;
  types: string[];
  details?: Record<string, string[]>;
  error?: string;
};

const SDK_POSSIBLE_PATHS = [
  "node_modules/@anthropic-ai/claude-code/sdk-tools.d.ts",
  "node_modules/@anthropic-ai/claude-code/dist/index.d.ts",
  "node_modules/@anthropic-ai/claude-code/dist/types.d.ts",
  "node_modules/@anthropic-ai/claude-code/index.d.ts",
];

const DEFAULT_DOC_URLS = [
  "https://platform.claude.com/docs/en/messages-streaming",
  "https://platform.claude.com/docs/en/api/messages-streaming",
  "https://docs.anthropic.com/claude/reference/messages-streaming",
  "https://docs.anthropic.com/claude/reference/messages-streaming#events",
  "https://docs.anthropic.com/claude/docs/messages-streaming",
];

function moduleDir(): string {
  const metaDir = (import.meta as { dirname?: string }).dirname;
  if (typeof metaDir === "string") {
    return metaDir;
  }
  return dirname(fileURLToPath(import.meta.url));
}

function findSdkTypesPath(): string | null {
  const resourceDir = join(moduleDir(), "..");
  const repoRoot = join(moduleDir(), "..", "..", "..");
  const searchRoots = [resourceDir, repoRoot];

  for (const root of searchRoots) {
    for (const relativePath of SDK_POSSIBLE_PATHS) {
      const fullPath = join(root, relativePath);
      if (existsSync(fullPath)) {
        return fullPath;
      }
    }
  }
  return null;
}

function extractStringLiterals(node: ts.TypeNode): string[] {
  if (ts.isLiteralTypeNode(node) && ts.isStringLiteral(node.literal)) {
    return [node.literal.text];
  }
  if (ts.isUnionTypeNode(node)) {
    return node.types.flatMap((typeNode) => extractStringLiterals(typeNode));
  }
  return [];
}

function containerName(node: ts.Node): string | null {
  let current: ts.Node | undefined = node;
  while (current) {
    if (ts.isInterfaceDeclaration(current) && current.name) {
      return current.name.text;
    }
    if (ts.isTypeAliasDeclaration(current) && current.name) {
      return current.name.text;
    }
    current = current.parent;
  }
  return null;
}

function collectFromSdkTypes(): SourceResult {
  const path = findSdkTypesPath();
  if (!path) {
    return { source: "sdk", types: [], error: "Claude SDK types not found" };
  }
  const content = readFileSync(path, "utf8");
  const sourceFile = ts.createSourceFile(path, content, ts.ScriptTarget.Latest, true);
  const types = new Set<string>();
  const details: Record<string, string[]> = {};

  function visit(node: ts.Node): void {
    if (ts.isPropertySignature(node)) {
      const name = node.name && ts.isIdentifier(node.name) ? node.name.text : null;
      if (name === "type" && node.type) {
        const literals = extractStringLiterals(node.type);
        if (literals.length > 0) {
          const parentName = containerName(node) ?? "anonymous";
          if (/Event|Stream|Message/i.test(parentName)) {
            literals.forEach((value) => types.add(value));
            details[parentName] = (details[parentName] ?? []).concat(literals);
          }
        }
      }
    }
    ts.forEachChild(node, visit);
  }

  visit(sourceFile);
  return { source: "sdk", types: Array.from(types).sort(), details };
}

function collectFromCli(prompt: string, timeoutMs: number): Promise<SourceResult> {
  return new Promise((resolve) => {
    const result: SourceResult = { source: "cli", types: [] };
    const types = new Set<string>();
    const denoGlobal = (globalThis as {
      Deno?: {
        which?: (cmd: string) => string | null;
        Command?: new (
          cmd: string,
          options: { args: string[]; stdout: "piped"; stderr: "piped" },
        ) => { output: () => Promise<{ stdout: Uint8Array; stderr: Uint8Array; code: number }> };
      };
    }).Deno;

    if (denoGlobal?.which && !denoGlobal.which("claude")) {
      result.error = "claude binary not found in PATH";
      resolve(result);
      return;
    }

    if (denoGlobal?.Command) {
      const command = new denoGlobal.Command("claude", {
        args: ["--print", "--output-format", "stream-json", "--verbose", prompt],
        stdout: "piped",
        stderr: "piped",
      });
      try {
        command
          .output()
          .then(({ stdout, stderr, code }) => {
            const text = new TextDecoder().decode(stdout);
            for (const line of text.split("\n")) {
              const trimmed = line.trim();
              if (!trimmed) continue;
              try {
                const value = JSON.parse(trimmed);
                if (value && typeof value.type === "string") {
                  types.add(value.type);
                }
              } catch {
                // ignore non-json
              }
            }
            result.types = Array.from(types).sort();
            if (code !== 0) {
              result.error =
                new TextDecoder().decode(stderr).trim() ||
                `claude exited with code ${code}`;
            }
            resolve(result);
          })
          .catch((error) => {
            result.error = error instanceof Error ? error.message : String(error);
            resolve(result);
          });
      } catch (error) {
        result.error = error instanceof Error ? error.message : String(error);
        resolve(result);
      }
      return;
    }
    let child;
    try {
      child = spawn(
        "claude",
        ["--print", "--output-format", "stream-json", "--verbose", prompt],
        { stdio: ["ignore", "pipe", "pipe"] },
      );
    } catch (error) {
      result.error = error instanceof Error ? error.message : String(error);
      resolve(result);
      return;
    }

    if (!child.stdout || !child.stderr) {
      result.error = "claude stdout/stderr not available";
      resolve(result);
      return;
    }

    let stderr = "";
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
    }, timeoutMs);

    child.stdout.on("data", (chunk) => {
      const text = chunk.toString("utf8");
      for (const line of text.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        try {
          const value = JSON.parse(trimmed);
          if (value && typeof value.type === "string") {
            types.add(value.type);
          }
        } catch {
          // ignore non-json
        }
      }
    });

    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString("utf8");
    });

    child.on("close", (code) => {
      clearTimeout(timer);
      result.types = Array.from(types).sort();
      if (code !== 0) {
        result.error = stderr.trim() || `claude exited with code ${code}`;
      }
      resolve(result);
    });
  });
}

async function collectFromDocs(urls: string[]): Promise<SourceResult> {
  if (typeof fetch !== "function") {
    return { source: "docs", types: [], error: "fetch is not available in this runtime" };
  }
  const effectiveUrls = urls.length > 0 ? urls : DEFAULT_DOC_URLS;
  const types = new Set<string>();
  const extractFromText = (text: string) => {
    const typeMatches = text.match(/\"type\"\\s*:\\s*\"([^\"]+)\"/g) ?? [];
    for (const match of typeMatches) {
      const value = match.split(":")[1]?.trim().replace(/^\"|\"$/g, "");
      if (value) types.add(value);
    }
    const eventMatches = text.match(/event\\s*:\\s*([a-z_]+)/gi) ?? [];
    for (const match of eventMatches) {
      const value = match.split(":")[1]?.trim();
      if (value) types.add(value);
    }
  };
  for (const url of effectiveUrls) {
    try {
      const res = await fetch(url);
      if (!res.ok) {
        continue;
      }
      const html = await res.text();
      const $ = load(html);
      const blocks = $("pre, code")
        .map((_, el) => $(el).text())
        .get();
      for (const block of blocks) {
        extractFromText(block);
      }
      const nextData = $("#__NEXT_DATA__").text();
      if (nextData) {
        extractFromText(nextData);
      }
      extractFromText(html);
    } catch {
      // ignore per-url errors
    }
  }
  return { source: "docs", types: Array.from(types).sort() };
}

type Args = {
  source: "all" | "sdk" | "cli" | "docs";
  prompt: string;
  timeoutMs: number;
  urls: string[];
  json: boolean;
};

function parseArgs(): Args {
  const args = process.argv.slice(2);
  const sourceArg = args.find((arg) => arg.startsWith("--source="));
  const promptArg = args.find((arg) => arg.startsWith("--prompt="));
  const timeoutArg = args.find((arg) => arg.startsWith("--timeoutMs="));
  const urlsArg = args.find((arg) => arg.startsWith("--urls="));
  const json = args.includes("--json");

  return {
    source: (sourceArg?.split("=")[1] as Args["source"]) ?? "all",
    prompt: promptArg?.split("=")[1] ?? "Reply with exactly OK.",
    timeoutMs: timeoutArg ? Number(timeoutArg.split("=")[1]) : 20000,
    urls: urlsArg ? urlsArg.split("=")[1]!.split(",") : DEFAULT_DOC_URLS,
    json,
  };
}

function summarize(results: SourceResult[]): void {
  const counts = results.map((r) => ({ source: r.source, count: r.types.length }));
  const max = Math.max(...counts.map((c) => c.count), 0);
  const best = counts.filter((c) => c.count === max).map((c) => c.source);
  const union = Array.from(
    new Set(results.flatMap((r) => r.types))
  ).sort();

  console.log("Claude event type extraction");
  console.log("============================");
  for (const result of results) {
    console.log(`- ${result.source}: ${result.types.length} types${result.error ? " (error)" : ""}`);
  }
  console.log(`\nMost comprehensive: ${best.join(", ") || "none"}`);
  console.log(`Union (${union.length}): ${union.join(", ")}`);
}

async function main(): Promise<void> {
  const args = parseArgs();
  const results: SourceResult[] = [];

  if (args.source === "all" || args.source === "sdk") {
    results.push(collectFromSdkTypes());
  }
  if (args.source === "all" || args.source === "cli") {
    results.push(await collectFromCli(args.prompt, args.timeoutMs));
  }
  if (args.source === "all" || args.source === "docs") {
    results.push(await collectFromDocs(args.urls));
  }

  if (args.json) {
    console.log(JSON.stringify({ results }, null, 2));
    return;
  }

  summarize(results);
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});

export { collectFromCli, collectFromDocs, collectFromSdkTypes };
