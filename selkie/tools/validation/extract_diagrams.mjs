#!/usr/bin/env node
/**
 * Extract mermaid diagrams from Cypress test files
 *
 * Usage: node extract_diagrams.mjs /path/to/mermaid/cypress > cypress_diagrams.json
 */

import { readFileSync, readdirSync, statSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

function findTestFiles(dir) {
  const files = [];

  function walk(currentDir) {
    const entries = readdirSync(currentDir);
    for (const entry of entries) {
      const fullPath = join(currentDir, entry);
      const stat = statSync(fullPath);
      if (stat.isDirectory()) {
        walk(fullPath);
      } else if (entry.endsWith('.spec.js') || entry.endsWith('.spec.ts')) {
        files.push(fullPath);
      }
    }
  }

  walk(dir);
  return files;
}

// Process JavaScript escape sequences in a string
function unescapeJsString(str) {
  // Process escape sequences as JavaScript would
  // Note: Order matters - process \\ before other escapes to avoid double processing
  return str
    .replace(/\\\\/g, '\x00')     // Temporarily replace \\ with a placeholder
    .replace(/\\n/g, '\n')
    .replace(/\\r/g, '\r')
    .replace(/\\t/g, '\t')
    .replace(/\\'/g, "'")
    .replace(/\\"/g, '"')
    .replace(/\\`/g, '`')          // Handle escaped backticks in template literals
    .replace(/\x00/g, '\\');       // Restore backslashes
}

function extractDiagrams(content, sourceFile) {
  const diagrams = [];

  // Match it(...) or describe(...) blocks with imgSnapshotTest
  // We need to find test names and their corresponding diagram strings

  // Pattern to match: it('test name', ... `diagram content` ...)
  // Note: We need to handle escaped backticks (\`) inside the template literal
  const itBlockRegex = /it\s*\(\s*['"]([^'"]+)['"]\s*,\s*(?:async\s*)?\(\s*\)\s*=>\s*\{[^}]*imgSnapshotTest\s*\(\s*`((?:[^`\\]|\\.)*)`/g;

  let match;
  while ((match = itBlockRegex.exec(content)) !== null) {
    const testName = match[1];
    let diagram = match[2];

    // The diagram in the JS file is a template literal read as raw text
    // We need to process escape sequences to get the actual content
    diagram = unescapeJsString(diagram);
    diagram = diagram.trim();

    // Detect diagram type from content
    let diagramType = 'unknown';
    if (diagram.includes('sequenceDiagram')) diagramType = 'sequence';
    else if (diagram.includes('classDiagram')) diagramType = 'class';
    else if (/^(graph|flowchart)/m.test(diagram)) diagramType = 'flowchart';
    else if (diagram.includes('stateDiagram')) diagramType = 'state';
    else if (diagram.includes('erDiagram')) diagramType = 'er';
    else if (diagram.includes('journey')) diagramType = 'journey';
    else if (diagram.includes('gantt')) diagramType = 'gantt';
    else if (diagram.includes('pie')) diagramType = 'pie';
    else if (diagram.includes('gitGraph')) diagramType = 'git';
    else if (diagram.includes('mindmap')) diagramType = 'mindmap';
    else if (diagram.includes('timeline')) diagramType = 'timeline';
    else if (diagram.includes('quadrantChart')) diagramType = 'quadrant';
    else if (diagram.includes('sankey')) diagramType = 'sankey';
    else if (diagram.includes('xychart')) diagramType = 'xychart';
    else if (diagram.includes('block-beta')) diagramType = 'block';
    else if (diagram.includes('packet-beta')) diagramType = 'packet';
    else if (diagram.includes('architecture')) diagramType = 'architecture';
    else if (diagram.includes('requirementDiagram')) diagramType = 'requirement';
    else if (diagram.includes('C4')) diagramType = 'c4';
    else if (diagram.includes('kanban')) diagramType = 'kanban';
    else if (diagram.includes('treemap')) diagramType = 'treemap';
    else if (diagram.includes('radar')) diagramType = 'radar';

    diagrams.push({
      test_name: testName,
      diagram_type: diagramType,
      diagram: diagram,
      source_file: sourceFile
    });
  }

  return diagrams;
}

async function main() {
  const mermaidCypressDir = process.argv[2] || '/Users/btucker/projects/mermaid/cypress';

  console.error(`Scanning ${mermaidCypressDir} for test files...`);

  const testFiles = findTestFiles(mermaidCypressDir);
  console.error(`Found ${testFiles.length} test files`);

  const allDiagrams = [];

  for (const file of testFiles) {
    try {
      const content = readFileSync(file, 'utf-8');
      const diagrams = extractDiagrams(content, file);
      allDiagrams.push(...diagrams);
    } catch (error) {
      console.error(`Error processing ${file}: ${error.message}`);
    }
  }

  console.error(`Extracted ${allDiagrams.length} diagrams`);

  // Output as JSON
  const output = {
    count: allDiagrams.length,
    diagrams: allDiagrams
  };

  console.log(JSON.stringify(output, null, 2));
}

main().catch(console.error);
