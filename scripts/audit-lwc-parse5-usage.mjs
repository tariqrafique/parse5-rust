import fs from 'node:fs';
import path from 'node:path';

import { ErrorCodes } from '../packages/parse5/src/index.js';

const root = path.resolve(import.meta.dirname, '..');
const lwcRoot = path.join(root, 'reference', 'lwc');

function read(file) {
  return fs.readFileSync(file, 'utf8');
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function walk(dir, predicate, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === 'node_modules' || entry.name === 'dist' || entry.name === '.git') {
      continue;
    }

    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(abs, predicate, out);
    } else if (predicate(abs)) {
      out.push(abs);
    }
  }
  return out;
}

assert(fs.existsSync(lwcRoot), 'reference/lwc is missing; run ./scripts/setup-reference.sh');

const packageJsons = walk(
  lwcRoot,
  (file) => path.basename(file) === 'package.json' && !file.includes(`${path.sep}playground${path.sep}`)
);

const parse5PackageUsers = [];
for (const file of packageJsons) {
  const json = JSON.parse(read(file));
  const deps = {
    ...json.dependencies,
    ...json.devDependencies,
    ...json.peerDependencies,
  };
  const names = Object.keys(deps).filter((name) => name === 'parse5' || name === '@parse5/tools' || name.startsWith('parse5-'));
  if (names.length > 0) {
    parse5PackageUsers.push({
      package: json.name,
      file: path.relative(root, file),
      deps: Object.fromEntries(names.map((name) => [name, deps[name]])),
    });
  }
}

assert(
  parse5PackageUsers.length === 2,
  `expected exactly 2 LWC packages to mention parse5 packages, found ${parse5PackageUsers.length}`
);
assert(
  parse5PackageUsers.some((entry) => entry.package === '@lwc/template-compiler' && entry.deps.parse5 === '^8.0.1' && entry.deps['@parse5/tools'] === '^0.7.0'),
  '@lwc/template-compiler parse5 dependency surface changed'
);
assert(
  parse5PackageUsers.some((entry) => entry.package === '@lwc/engine-server' && entry.deps.parse5 === '^8.0.1'),
  '@lwc/engine-server parse5 dependency surface changed'
);

const sourceFiles = walk(lwcRoot, (file) => /\.(?:mjs|cjs|js|ts|tsx)$/.test(file));
const importRe = /from\s+['"]([^'"]+)['"]|import\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
const parse5Imports = [];

for (const file of sourceFiles) {
  const text = read(file);
  for (const match of text.matchAll(importRe)) {
    const specifier = match[1] || match[2];
    if (specifier === 'parse5' || specifier === '@parse5/tools' || specifier.startsWith('parse5-')) {
      parse5Imports.push({
        file: path.relative(root, file),
        specifier,
      });
    }
  }
}

const importedSiblingPackages = parse5Imports.filter(({ specifier }) => specifier.startsWith('parse5-'));
assert(
  importedSiblingPackages.length === 0,
  `LWC now imports parse5 sibling packages: ${JSON.stringify(importedSiblingPackages, null, 2)}`
);

const parse5ErrorCodes = Object.values(ErrorCodes);

const lwcParse5Errors = read(path.join(lwcRoot, 'packages', '@lwc', 'template-compiler', 'src', 'parser', 'parse5Errors.ts'));
const lwcKnownErrorCodes = [...lwcParse5Errors.matchAll(/'([^']+)'/g)].map((match) => match[1]);

const missingFromLwc = parse5ErrorCodes.filter((code) => !lwcKnownErrorCodes.includes(code));
const extraInLwc = lwcKnownErrorCodes.filter((code) => !parse5ErrorCodes.includes(code));

assert(parse5ErrorCodes.length === 60, `expected parse5 to expose 60 error codes, found ${parse5ErrorCodes.length}`);
assert(missingFromLwc.length === 0, `LWC does not account for parse5 error codes: ${missingFromLwc.join(', ')}`);
assert(extraInLwc.length === 0, `LWC tracks non-parse5 error codes: ${extraInLwc.join(', ')}`);

const parserIndex = read(path.join(lwcRoot, 'packages', '@lwc', 'template-compiler', 'src', 'parser', 'index.ts'));
const parse5ToolsSymbols = [...parserIndex.matchAll(/parse5Tools\.([A-Za-z0-9_]+)/g)].map((match) => match[1]);
const runtimeTools = [...new Set(parse5ToolsSymbols.filter((symbol) => symbol.startsWith('is')))].sort();

assert(
  JSON.stringify(runtimeTools) === JSON.stringify(['isCommentNode', 'isElementNode', 'isTemplateNode', 'isTextNode']),
  `unexpected @parse5/tools runtime symbols: ${runtimeTools.join(', ')}`
);

console.log('LWC parse5 usage audit passed.');
console.log(JSON.stringify({ parse5PackageUsers, parse5Imports, runtimeTools }, null, 2));
