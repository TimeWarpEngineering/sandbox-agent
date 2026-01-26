import { collectFromDocs } from "./claude-event-types.js";

const urlsArg = process.argv.slice(2).find((arg) => arg.startsWith("--urls="));
const urls = urlsArg ? urlsArg.split("=")[1]!.split(",") : undefined;

collectFromDocs(urls ?? []).then((result) => {
  console.log(JSON.stringify(result, null, 2));
});
