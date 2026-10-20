// Turns src/client.ts into the classic browser script dist/assets/site.js.
import { stripTypeScriptTypes } from 'node:module';

/** Types stripped, `export` removed, wrapped in a function. Throws if the source imports anything. */
export function clientScript(source: string): string {
  const js = stripTypeScriptTypes(source, { mode: 'strip' }).replace(/^export /gm, '');
  if (/^\s*import\s/m.test(js)) throw new Error('src/client.ts must not import anything');
  return `// BETA Cutting Room website motion. Generated from src/client.ts.\n(function () {\n'use strict';\n${js}\n})();\n`;
}

/** True when browser code could talk to a server (the site must make no runtime requests). */
export function usesNetwork(js: string): boolean {
  return /\b(?:fetch|XMLHttpRequest|WebSocket|EventSource|sendBeacon)\b/.test(js);
}
