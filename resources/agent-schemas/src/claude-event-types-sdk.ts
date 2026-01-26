import { collectFromSdkTypes } from "./claude-event-types.js";

const result = collectFromSdkTypes();
console.log(JSON.stringify(result, null, 2));
