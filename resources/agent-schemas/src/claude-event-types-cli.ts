import { collectFromCli } from "./claude-event-types.js";

const promptArg = process.argv.slice(2).find((arg) => arg.startsWith("--prompt="));
const timeoutArg = process.argv.slice(2).find((arg) => arg.startsWith("--timeoutMs="));

const prompt = promptArg?.split("=")[1] ?? "Reply with exactly OK.";
const timeoutMs = timeoutArg ? Number(timeoutArg.split("=")[1]) : 20000;

collectFromCli(prompt, timeoutMs).then((result) => {
  console.log(JSON.stringify(result, null, 2));
});
