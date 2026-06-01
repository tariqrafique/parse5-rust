import assert from 'node:assert/strict';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(import.meta.dirname, '..');

const reference = await import(
  pathToFileURL(path.join(root, 'reference', 'parse5', 'packages', 'parse5', 'dist', 'index.js')).href
);
const port = await import(pathToFileURL(path.join(root, 'packages', 'parse5-wasm', 'src', 'index.js')).href);

await port.initParse5Wasm();

assert.deepEqual(
  Object.keys(reference)
    .sort()
    .filter((key) => !(key in port)),
  []
);

function withoutParentLinks(value) {
  if (Array.isArray(value)) {
    return value.map(withoutParentLinks);
  }

  if (!value || typeof value !== 'object') {
    return value;
  }

  const output = {};

  for (const key of Object.keys(value).sort()) {
    if (key !== 'parentNode') {
      output[key] = withoutParentLinks(value[key]);
    }
  }

  return output;
}

function compactToken(token) {
  const output = {
    type: token.type,
  };

  for (const key of ['name', 'tagName', 'chars', 'data', 'forceQuirks', 'publicId', 'systemId', 'selfClosing']) {
    if (key in token) {
      output[key] = token[key];
    }
  }

  if (Array.isArray(token.attrs)) {
    output.attrs = token.attrs;
  }

  if (token.location) {
    output.location = withoutParentLinks(token.location);
  }

  return output;
}

function tokenize(mod, html) {
  const tokens = [];
  const errors = [];

  const tokenizer = new mod.Tokenizer(
    { sourceCodeLocationInfo: true },
    {
      onCharacter: (token) => tokens.push(compactToken(token)),
      onComment: (token) => tokens.push(compactToken(token)),
      onDoctype: (token) => tokens.push(compactToken(token)),
      onEndTag: (token) => tokens.push(compactToken(token)),
      onEof: (token) => tokens.push(compactToken(token)),
      onNullCharacter: (token) => tokens.push(compactToken(token)),
      onParseError: (error) => errors.push(withoutParentLinks(error)),
      onStartTag: (token) => tokens.push(compactToken(token)),
      onWhitespaceCharacter: (token) => tokens.push(compactToken(token)),
    }
  );

  tokenizer.write(html, true);

  return {
    active: tokenizer.active,
    errors,
    lastStartTagName: tokenizer.lastStartTagName,
    state: tokenizer.state,
    tokens,
  };
}

function exercisePausedTokenizer(mod) {
  const seen = [];
  let tokenizer;

  tokenizer = new mod.Tokenizer(
    {},
    {
      onComment(token) {
        seen.push(['comment', token.data]);
        tokenizer.pause();
      },
      onDoctype(token) {
        seen.push(['doctype', token.name]);
      },
      onEof() {
        seen.push(['eof']);
      },
    }
  );

  tokenizer.write('<!--INIT-->', false, () => seen.push(['first write callback']));
  assert.equal(tokenizer.paused, true);
  assert.deepEqual(seen, [['comment', 'INIT']]);

  tokenizer.write('<!doctype foo>', true, () => seen.push(['second write callback']));
  assert.equal(tokenizer.paused, true);
  assert.deepEqual(seen, [['comment', 'INIT']]);

  tokenizer.resume(() => seen.push(['resume callback']));
  assert.equal(tokenizer.paused, false);
  assert.deepEqual(seen, [['comment', 'INIT'], ['doctype', 'foo'], ['eof'], ['resume callback']]);

  assert.throws(() => tokenizer.resume(), /Parser was already resumed/);
}

function parseDocumentWithParser(mod, chunks) {
  const parser = new mod.Parser({ sourceCodeLocationInfo: true });

  chunks.forEach((chunk, idx) => {
    parser.tokenizer.write(chunk, idx === chunks.length - 1);
  });

  return parser.document;
}

function parseFragmentWithParser(mod, fragmentContext, chunks) {
  const parser = mod.Parser.getFragmentParser(fragmentContext, { sourceCodeLocationInfo: true });

  chunks.forEach((chunk, idx) => {
    parser.tokenizer.write(chunk, idx === chunks.length - 1);
  });

  return parser.getFragment();
}

function createLocationScalingAdapter(mod) {
  function scaleLocation(location) {
    if (location === null) {
      return null;
    }

    return {
      startLine: location.startLine * 2,
      startCol: location.startCol * 2,
      startOffset: location.startOffset * 2,
      endLine: location.endLine * 2,
      endCol: location.endCol * 2,
      endOffset: location.endOffset * 2,
    };
  }

  return {
    ...mod.defaultTreeAdapter,
    setNodeSourceCodeLocation(node, location) {
      node.sourceCodeLocation = scaleLocation(location);
    },
    updateNodeSourceCodeLocation(node, endLocation) {
      if (node.sourceCodeLocation) {
        node.sourceCodeLocation = {
          ...node.sourceCodeLocation,
          endLine: endLocation.endLine * 2,
          endCol: endLocation.endCol * 2,
          endOffset: endLocation.endOffset * 2,
        };
      }
    },
  };
}

function traceStackEvents(mod, input) {
  const events = [];
  const document = mod.parse(input, {
    treeAdapter: {
      ...mod.defaultTreeAdapter,
      onItemPush(node) {
        events.push(['push', node.nodeName]);
      },
      onItemPop(node, newTop) {
        events.push(['pop', node.nodeName, newTop?.nodeName ?? null]);
      },
    },
  });

  return {
    events,
    html: mod.serialize(document),
  };
}

function exerciseScriptHandler(mod, resumeSynchronously) {
  const parser = new mod.Parser();
  const calls = [];
  const callbacks = [];

  parser.scriptHandler = (scriptElement) => {
    calls.push([scriptElement.tagName, scriptElement.childNodes?.[0]?.value ?? '']);
    parser.tokenizer.pause();
    parser.tokenizer.insertHtmlAtCurrentPos('<p>written</p>');

    if (resumeSynchronously) {
      parser.tokenizer.resume(() => callbacks.push('resume'));
    }
  };

  parser.tokenizer.write(
    '<!doctype html><script>document.write("<p>written</p>")</script><p>after</p>',
    true,
    () => callbacks.push('write')
  );

  const pausedAfterWrite = parser.tokenizer.paused;

  if (!resumeSynchronously) {
    parser.tokenizer.resume(() => callbacks.push('resume'));
  }

  return {
    calls,
    callbacks,
    html: mod.serialize(parser.document),
    pausedAfterWrite,
  };
}

function observeScriptHandlerWithoutPause(mod) {
  const parser = new mod.Parser();
  const calls = [];
  const callbacks = [];

  parser.scriptHandler = (scriptElement) => {
    calls.push([scriptElement.tagName, scriptElement.childNodes?.[0]?.value ?? '']);
  };

  parser.tokenizer.write(
    '<!doctype html><script>seen()</script><p>after</p>',
    true,
    () => callbacks.push('write')
  );

  return {
    calls,
    callbacks,
    html: mod.serialize(parser.document),
    pausedAfterWrite: parser.tokenizer.paused,
  };
}

const parserOptions = { sourceCodeLocationInfo: true };
const documentInput =
  '<!doctype html><html><head><title>x</title></head><body><template><p id="x">Hi</p></template><svg><foreignObject><p>A</p></foreignObject></svg></body></html>';

assert.deepEqual(
  withoutParentLinks(port.parse(documentInput, parserOptions)),
  withoutParentLinks(reference.parse(documentInput, parserOptions))
);

const parseErrors = { reference: [], port: [] };
const parseErrorInput = '<!doctype html><html><head></head><body><p></div></body></html>';

assert.equal(
  port.serialize(
    port.parse(parseErrorInput, {
      sourceCodeLocationInfo: true,
      onParseError: (error) => parseErrors.port.push(withoutParentLinks(error)),
    })
  ),
  reference.serialize(
    reference.parse(parseErrorInput, {
      sourceCodeLocationInfo: true,
      onParseError: (error) => parseErrors.reference.push(withoutParentLinks(error)),
    })
  )
);
assert.deepEqual(parseErrors.port, parseErrors.reference);

const parserChunks = [
  '<!doctype html><html><head>',
  '<title>x</title></head><body>',
  '<p data-x="1">hello</p></body></html>',
];

assert.deepEqual(
  withoutParentLinks(parseDocumentWithParser(port, parserChunks)),
  withoutParentLinks(parseDocumentWithParser(reference, parserChunks))
);
assert.deepEqual(observeScriptHandlerWithoutPause(port), observeScriptHandlerWithoutPause(reference));
assert.deepEqual(exerciseScriptHandler(port, false), exerciseScriptHandler(reference, false));
assert.deepEqual(exerciseScriptHandler(port, true), exerciseScriptHandler(reference, true));

const customAdapterInput = '<!doctype><body>Testing location</body>';
const customAdapterOptions = (mod) => ({
  treeAdapter: createLocationScalingAdapter(mod),
  sourceCodeLocationInfo: true,
});

assert.deepEqual(
  withoutParentLinks(port.parse(customAdapterInput, customAdapterOptions(port))),
  withoutParentLinks(reference.parse(customAdapterInput, customAdapterOptions(reference)))
);

assert.deepEqual(
  withoutParentLinks(port.Parser.parse(customAdapterInput, customAdapterOptions(port))),
  withoutParentLinks(reference.Parser.parse(customAdapterInput, customAdapterOptions(reference)))
);

assert.deepEqual(traceStackEvents(port, '<p><p>'), traceStackEvents(reference, '<p><p>'));
assert.deepEqual(
  traceStackEvents(port, '<table><tr><td>x<td>y</table>'),
  traceStackEvents(reference, '<table><tr><td>x<td>y</table>')
);

const externalDocument = port.defaultTreeAdapter.createDocument();
const externalParser = new port.Parser({ sourceCodeLocationInfo: true }, externalDocument);
externalParser.tokenizer.write('<!doctype html><html><body><p>external</p></body></html>', true);
assert.equal(externalParser.document, externalDocument);
assert.equal(port.serialize(externalDocument), '<!DOCTYPE html><html><head></head><body><p>external</p></body></html>');

const tableContext = reference.defaultTreeAdapter.createElement('table', reference.html.NS.HTML, []);
const fragmentInput = '<caption>x</caption><tbody><tr><td>A<td>B</tbody>';

assert.deepEqual(
  withoutParentLinks(port.parseFragment(tableContext, fragmentInput, parserOptions)),
  withoutParentLinks(reference.parseFragment(tableContext, fragmentInput, parserOptions))
);

assert.deepEqual(
  withoutParentLinks(parseFragmentWithParser(port, tableContext, ['<tbody><tr>', '<td>A<td>B</tbody>'])),
  withoutParentLinks(parseFragmentWithParser(reference, tableContext, ['<tbody><tr>', '<td>A<td>B</tbody>']))
);

const serializationInput = '<section data-x="a&nbsp;b"><script>1 < 2 && 3 > 2</script><br></section>';
assert.equal(
  port.serialize(port.parseFragment(serializationInput)),
  reference.serialize(reference.parseFragment(serializationInput))
);

const tokenizerInput = '<!doctype html><!--x--><a href="b">c</a>';
assert.deepEqual(tokenize(port, tokenizerInput), tokenize(reference, tokenizerInput));

exercisePausedTokenizer(port);

console.log('parse5-wasm generated runtime tests passed.');
