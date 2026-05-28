#!/usr/bin/env node
/**
 * Mermaid diagram validator using the official mermaid library.
 *
 * Usage:
 *   node validate_mermaid.mjs <diagram_text>
 *   echo "sequenceDiagram\nAlice->>Bob: Hi" | node validate_mermaid.mjs -
 *
 * Output (JSON):
 *   { "valid": true, "diagramType": "sequence" }
 *   { "valid": false, "error": "Parse error..." }
 */

import { readFileSync } from 'fs';
import { JSDOM } from 'jsdom';

// Set up minimal DOM environment for mermaid
const dom = new JSDOM('<!DOCTYPE html><html><body></body></html>');
global.window = dom.window;
global.document = dom.window.document;
Object.defineProperty(global, 'navigator', {
  value: dom.window.navigator,
  writable: true,
});

// Import mermaid after setting up DOM
const mermaid = (await import('mermaid')).default;

// Initialize mermaid
mermaid.initialize({
  startOnLoad: false,
  securityLevel: 'loose',
});

async function validateDiagram(text) {
  try {
    const result = await mermaid.parse(text, { suppressErrors: false });
    return {
      valid: true,
      diagramType: result?.diagramType || 'unknown',
    };
  } catch (error) {
    return {
      valid: false,
      error: error.message || String(error),
    };
  }
}

async function main() {
  let input;

  if (process.argv[2] === '-') {
    // Read from stdin
    input = readFileSync(0, 'utf-8');
  } else if (process.argv[2]) {
    // Read from file or use as direct input
    try {
      input = readFileSync(process.argv[2], 'utf-8');
    } catch {
      input = process.argv[2];
    }
  } else {
    console.error('Usage: node validate_mermaid.mjs <diagram_text_or_file>');
    process.exit(1);
  }

  const result = await validateDiagram(input);
  console.log(JSON.stringify(result));
}

main();
