import fs from 'node:fs';
import path from 'node:path';

import { html, TokenizerMode } from '../packages/parse5/src/index.js';

export function makeChunks(str, minSize = 1, maxSize = 10) {
  if (str.length === 0) {
    return [''];
  }

  const chunks = [];
  let start = 0;
  let end = 1;
  let idx = 0;

  while (start < str.length) {
    chunks.push(str.substring(start, end));
    start = end;

    const span = maxSize - minSize + 1;
    const nextSize = minSize + ((idx * 7) % span);
    end = Math.min(end + nextSize, str.length);
    idx++;
  }

  return chunks;
}

export function normalizeNewLine(str) {
  return str.replace(/\r\n/g, '\n');
}

export function removeNewLines(str) {
  return str.replace(/[\n\r]/g, '');
}

export function loadSAXParserTestData(root) {
  const dataDir = path.join(root, 'fixtures/parse5/test/data/sax');

  return fs.readdirSync(dataDir).map((name) => ({
    name,
    src: normalizeNewLine(fs.readFileSync(path.join(dataDir, name, 'src.html'), 'utf8')),
    expected: normalizeNewLine(fs.readFileSync(path.join(dataDir, name, 'expected.html'), 'utf8')),
  }));
}

function unicodeUnescape(str) {
  return str.replace(/\\[Uu]\w{4}/g, (match) => String.fromCharCode(Number.parseInt(match.slice(2), 16)));
}

function unescapeTokenizationDescriptor(descr) {
  descr.input = unicodeUnescape(descr.input);

  for (const tokenEntry of descr.output) {
    if (tokenEntry[1]) {
      tokenEntry[1] = unicodeUnescape(tokenEntry[1]);
    }
  }
}

function tokenizerStateName(name) {
  return name.slice(0, -6).replace(' ', '_').toUpperCase();
}

export function loadParserFeedbackTokenizationTests(root) {
  const dataDir = path.join(root, 'fixtures/parse5/test/data/parser-feedback');
  const tests = [];
  let idx = 0;

  for (const fileName of fs.readdirSync(dataDir).sort()) {
    if (path.extname(fileName) !== '.test') {
      continue;
    }

    const setName = fileName.replace('.test', '');
    const testSet = JSON.parse(fs.readFileSync(path.join(dataDir, fileName), 'utf8'));

    for (const descr of testSet.tests ?? []) {
      const mutableDescr = structuredClone(descr);

      if (mutableDescr.doubleEscaped) {
        unescapeTokenizationDescriptor(mutableDescr);
      }

      for (const initialStateName of mutableDescr.initialStates ?? ['Data state']) {
        const stateName = tokenizerStateName(initialStateName);
        const initialState = TokenizerMode[stateName];

        if (typeof initialState !== 'number') {
          throw new Error(`Unknown tokenizer state ${initialStateName}`);
        }

        tests.push({
          idx: ++idx,
          setName,
          name: mutableDescr.description,
          input: mutableDescr.input,
          expected: mutableDescr.output,
          expectedErrors: mutableDescr.errors ?? [],
          initialState,
          initialStateName,
          lastStartTag: mutableDescr.lastStartTag,
        });
      }
    }
  }

  return tests;
}

function createFragmentContext(tagName, treeAdapter) {
  if (!tagName) {
    return null;
  }

  let namespace = html.NS.HTML;
  const parts = tagName.split(' ');

  if (parts.length > 1) {
    tagName = parts[1];

    if (parts[0] === 'svg') {
      namespace = html.NS.SVG;
    } else if (parts[0] === 'math') {
      namespace = html.NS.MATHML;
    }
  }

  return treeAdapter.createElement(tagName, namespace, []);
}

export function parseDatFile(testSet, treeAdapter) {
  const testDescrs = [];
  let currentDirective = '';
  let currentDescr = {};

  for (const [idx, line] of testSet.split(/\r?\n/).entries()) {
    if (line === '#data') {
      currentDescr = { '#line': idx + 1 };
      testDescrs.push(currentDescr);
    }

    if (line[0] === '#') {
      currentDirective = line;
      currentDescr[currentDirective] = [];
    } else {
      currentDescr[currentDirective].push(line);
    }
  }

  return testDescrs.map((descr) => {
    const fragmentContextTagName = descr['#document-fragment']?.[0];

    return {
      input: descr['#data'].join('\n'),
      expected: descr['#document'].join('\n'),
      expectedErrors: descr['#new-errors'] ?? [],
      lineNum: descr['#line'],
      scriptingEnabled: !descr['#script-off'],
      fragmentContext: createFragmentContext(fragmentContextTagName, treeAdapter),
    };
  });
}

export function loadTreeConstructionTestData(root, relativeSuitePath, treeAdapter) {
  const dataDir = path.join(root, relativeSuitePath);
  const tests = [];
  const dirName = path.basename(dataDir);

  for (const fileName of fs.readdirSync(dataDir).sort()) {
    if (path.extname(fileName) !== '.dat') {
      continue;
    }

    const testSet = fs.readFileSync(path.join(dataDir, fileName), 'utf8');
    const setName = fileName.replace('.dat', '');

    for (const [idx, test] of parseDatFile(testSet, treeAdapter).entries()) {
      tests.push({
        ...test,
        idx,
        setName,
        dirName,
      });
    }
  }

  return tests;
}

function getSerializedTreeIndent(indent) {
  return '|'.padEnd(indent + 2, ' ');
}

function getElementSerializedNamespaceURI(element, treeAdapter) {
  switch (treeAdapter.getNamespaceURI(element)) {
    case html.NS.SVG:
      return 'svg ';
    case html.NS.MATHML:
      return 'math ';
    default:
      return '';
  }
}

function serializeNodeList(nodes, indent, treeAdapter) {
  let str = '';

  for (let node of nodes) {
    str += getSerializedTreeIndent(indent);

    if (treeAdapter.isCommentNode(node)) {
      str += `<!-- ${treeAdapter.getCommentNodeContent(node)} -->\n`;
    } else if (treeAdapter.isTextNode(node)) {
      str += `"${treeAdapter.getTextNodeContent(node)}"\n`;
    } else if (treeAdapter.isDocumentTypeNode(node)) {
      const publicId = treeAdapter.getDocumentTypeNodePublicId(node);
      const systemId = treeAdapter.getDocumentTypeNodeSystemId(node);

      str += `<!DOCTYPE ${treeAdapter.getDocumentTypeNodeName(node) || ''}`;

      if (publicId || systemId) {
        str += ` "${publicId}" "${systemId}"`;
      }

      str += '>\n';
    } else {
      const tagName = treeAdapter.getTagName(node);
      str += `<${getElementSerializedNamespaceURI(node, treeAdapter) + tagName}>\n`;

      let childrenIndent = indent + 2;
      const serializedAttrs = treeAdapter.getAttrList(node).map((attr) => {
        let attrStr = getSerializedTreeIndent(childrenIndent);

        if (attr.prefix) {
          attrStr += `${attr.prefix} `;
        }

        attrStr += `${attr.name}="${attr.value}"\n`;
        return attrStr;
      });

      str += serializedAttrs.sort().join('');

      if (tagName === html.TAG_NAMES.TEMPLATE && treeAdapter.getNamespaceURI(node) === html.NS.HTML) {
        str += `${getSerializedTreeIndent(childrenIndent)}content\n`;
        childrenIndent += 2;
        node = treeAdapter.getTemplateContent(node);
      }

      str += serializeNodeList(treeAdapter.getChildNodes(node), childrenIndent, treeAdapter);
    }
  }

  return str;
}

export function serializeToDatFileFormat(rootNode, treeAdapter) {
  return serializeNodeList(treeAdapter.getChildNodes(rootNode), 0, treeAdapter);
}
