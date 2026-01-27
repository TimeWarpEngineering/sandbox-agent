import { execSync } from "child_process";
import { existsSync, readFileSync, rmSync, readdirSync } from "fs";
import { join } from "path";
import { createNormalizedSchema, type NormalizedSchema } from "./normalize.js";
import type { JSONSchema7 } from "json-schema";

function normalizeCodexRefs(value: JSONSchema7): JSONSchema7 {
  if (Array.isArray(value)) {
    return value.map((item) => normalizeCodexRefs(item as JSONSchema7)) as JSONSchema7;
  }

  if (value && typeof value === "object") {
    const next: Record<string, JSONSchema7> = {};
    for (const [key, child] of Object.entries(value)) {
      if (key === "$ref" && typeof child === "string") {
        next[key] = child.replace("#/definitions/v2/", "#/definitions/") as JSONSchema7;
        continue;
      }
      next[key] = normalizeCodexRefs(child as JSONSchema7);
    }
    return next as JSONSchema7;
  }

  return value;
}

export async function extractCodexSchema(): Promise<NormalizedSchema> {
  console.log("Extracting Codex schema via CLI...");

  const tempDir = join(import.meta.dirname, "..", ".temp-codex-schemas");

  try {
    // Run codex CLI to generate JSON schema
    execSync(`codex app-server generate-json-schema --out "${tempDir}"`, {
      encoding: "utf-8",
      timeout: 30000,
      stdio: ["pipe", "pipe", "pipe"],
    });

    // Read generated schema files from temp directory
    const definitions: Record<string, JSONSchema7> = {};

    if (existsSync(tempDir)) {
      const files = readdirSync(tempDir).filter((f) => f.endsWith(".json"));

      for (const file of files) {
        const filePath = join(tempDir, file);
        const content = readFileSync(filePath, "utf-8");
        const schema = JSON.parse(content);

        // Extract the name from the file (e.g., "ThreadEvent.json" -> "ThreadEvent")
        const name = file.replace(".json", "");

        if (schema.definitions) {
          for (const [defName, def] of Object.entries(schema.definitions)) {
            definitions[defName] = normalizeCodexRefs(def as JSONSchema7);
          }
        } else if (schema.$defs) {
          for (const [defName, def] of Object.entries(schema.$defs)) {
            definitions[defName] = normalizeCodexRefs(def as JSONSchema7);
          }
        } else {
          definitions[name] = normalizeCodexRefs(schema as JSONSchema7);
        }
      }

      // Clean up temp directory
      rmSync(tempDir, { recursive: true, force: true });
    }

    if (Object.keys(definitions).length === 0) {
      console.log("  [warn] No schemas extracted from CLI, using fallback");
      return createFallbackSchema();
    }

    console.log(`  [ok] Extracted ${Object.keys(definitions).length} types from CLI`);

    return createNormalizedSchema("codex", "Codex SDK Schema", definitions);
  } catch (error) {
    // Clean up temp directory on error
    if (existsSync(tempDir)) {
      rmSync(tempDir, { recursive: true, force: true });
    }

    const errorMessage = error instanceof Error ? error.message : String(error);
    console.log(`  [warn] CLI extraction failed: ${errorMessage}`);
    console.log("  [fallback] Using embedded schema definitions");
    return createFallbackSchema();
  }
}

function createFallbackSchema(): NormalizedSchema {
  // Fallback schema based on known SDK structure
  const definitions: Record<string, JSONSchema7> = {
    ThreadEvent: {
      type: "object",
      properties: {
        type: {
          type: "string",
          enum: ["thread.created", "thread.updated", "item.created", "item.updated", "error"],
        },
        thread_id: { type: "string" },
        item: { $ref: "#/definitions/ThreadItem" },
        error: { type: "object" },
      },
      required: ["type"],
    },
    ThreadItem: {
      type: "object",
      properties: {
        id: { type: "string" },
        type: { type: "string", enum: ["message", "function_call", "function_result"] },
        role: { type: "string", enum: ["user", "assistant", "system"] },
        content: {
          oneOf: [{ type: "string" }, { type: "array", items: { type: "object" } }],
        },
        status: { type: "string", enum: ["pending", "in_progress", "completed", "failed"] },
      },
      required: ["id", "type"],
    },
    CodexOptions: {
      type: "object",
      properties: {
        apiKey: { type: "string" },
        model: { type: "string" },
        baseURL: { type: "string" },
        maxTokens: { type: "number" },
        temperature: { type: "number" },
      },
    },
    ThreadOptions: {
      type: "object",
      properties: {
        instructions: { type: "string" },
        tools: { type: "array", items: { type: "object" } },
        model: { type: "string" },
        workingDirectory: { type: "string" },
      },
    },
    Input: {
      type: "object",
      properties: {
        type: { type: "string", enum: ["text", "file", "image"] },
        content: { type: "string" },
        path: { type: "string" },
        mimeType: { type: "string" },
      },
      required: ["type"],
    },
    ResponseItem: {
      type: "object",
      properties: {
        type: { type: "string" },
        id: { type: "string" },
        content: { type: "string" },
        function_call: { $ref: "#/definitions/FunctionCall" },
      },
    },
    FunctionCall: {
      type: "object",
      properties: {
        name: { type: "string" },
        arguments: { type: "string" },
        call_id: { type: "string" },
      },
      required: ["name", "arguments"],
    },
    Message: {
      type: "object",
      properties: {
        role: { type: "string", enum: ["user", "assistant", "system"] },
        content: { type: "string" },
      },
      required: ["role", "content"],
    },
  };

  console.log(`  [ok] Using fallback schema with ${Object.keys(definitions).length} definitions`);

  return createNormalizedSchema("codex", "Codex SDK Schema", definitions);
}
