import { execSync } from "child_process";
import { createNormalizedSchema, type NormalizedSchema } from "./normalize.js";
import type { JSONSchema7 } from "json-schema";

export async function extractClaudeSchema(): Promise<NormalizedSchema> {
  console.log("Extracting Claude Code schema via CLI...");

  try {
    // Run claude CLI with --json-schema flag to get the schema
    const output = execSync("claude --output-format json --json-schema", {
      encoding: "utf-8",
      timeout: 30000,
      stdio: ["pipe", "pipe", "pipe"],
    });

    // Parse the JSON output
    const parsed = JSON.parse(output);

    // Extract definitions from the schema
    const definitions: Record<string, JSONSchema7> = {};

    if (parsed.definitions) {
      for (const [name, def] of Object.entries(parsed.definitions)) {
        definitions[name] = def as JSONSchema7;
      }
    } else if (parsed.$defs) {
      for (const [name, def] of Object.entries(parsed.$defs)) {
        definitions[name] = def as JSONSchema7;
      }
    } else {
      // The output might be a single schema, use it as the root
      definitions["Schema"] = parsed as JSONSchema7;
    }

    console.log(`  [ok] Extracted ${Object.keys(definitions).length} types from CLI`);

    return createNormalizedSchema("claude", "Claude Code SDK Schema", definitions);
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.log(`  [warn] CLI extraction failed: ${errorMessage}`);
    console.log("  [fallback] Using embedded schema definitions");
    return createFallbackSchema();
  }
}

function createFallbackSchema(): NormalizedSchema {
  // Fallback schema based on known SDK structure
  const definitions: Record<string, JSONSchema7> = {
    SDKMessage: {
      type: "object",
      properties: {
        type: { type: "string", enum: ["user", "assistant", "result"] },
        content: { type: "string" },
        timestamp: { type: "string", format: "date-time" },
      },
      required: ["type"],
    },
    SDKResultMessage: {
      type: "object",
      properties: {
        type: { type: "string", const: "result" },
        result: { type: "object" },
        error: { type: "string" },
        duration_ms: { type: "number" },
      },
      required: ["type"],
    },
    Options: {
      type: "object",
      properties: {
        model: { type: "string" },
        maxTokens: { type: "number" },
        temperature: { type: "number" },
        systemPrompt: { type: "string" },
        tools: { type: "array", items: { type: "string" } },
        allowedTools: { type: "array", items: { type: "string" } },
        workingDirectory: { type: "string" },
      },
    },
    BashInput: {
      type: "object",
      properties: {
        command: { type: "string" },
        timeout: { type: "number" },
        workingDirectory: { type: "string" },
      },
      required: ["command"],
    },
    FileEditInput: {
      type: "object",
      properties: {
        path: { type: "string" },
        oldText: { type: "string" },
        newText: { type: "string" },
      },
      required: ["path", "oldText", "newText"],
    },
    FileReadInput: {
      type: "object",
      properties: {
        path: { type: "string" },
        startLine: { type: "number" },
        endLine: { type: "number" },
      },
      required: ["path"],
    },
    FileWriteInput: {
      type: "object",
      properties: {
        path: { type: "string" },
        content: { type: "string" },
      },
      required: ["path", "content"],
    },
    GlobInput: {
      type: "object",
      properties: {
        pattern: { type: "string" },
        path: { type: "string" },
      },
      required: ["pattern"],
    },
    GrepInput: {
      type: "object",
      properties: {
        pattern: { type: "string" },
        path: { type: "string" },
        include: { type: "string" },
      },
      required: ["pattern"],
    },
  };

  console.log(`  [ok] Using fallback schema with ${Object.keys(definitions).length} definitions`);

  return createNormalizedSchema("claude", "Claude Code SDK Schema", definitions);
}
