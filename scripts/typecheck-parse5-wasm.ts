/// <reference path="./node-stream.d.ts" />

import {
  Parser,
  type ParserOptions,
  type SerializerOptions,
  type TreeAdapter,
  type TreeAdapterTypeMap,
  defaultTreeAdapter,
  parse,
  parseFragment,
  serialize,
  serializeOuter,
} from '../packages/parse5/src/index.js';
import { ParserStream } from '../packages/parse5-parser-stream/src/index.js';
import { PlainTextConversionStream } from '../packages/parse5-plain-text-conversion-stream/src/index.js';
import { SAXParser, type StartTag } from '../packages/parse5-sax-parser/src/index.js';
import { ParserFeedbackSimulator } from '../packages/parse5-sax-parser/src/parser-feedback-simulator.js';
import { RewritingStream } from '../packages/parse5-html-rewriting-stream/src/index.js';
import {
  adapter as htmlparser2Adapter,
  type Htmlparser2TreeAdapterMap,
} from '../packages/parse5-htmlparser2-tree-adapter/src/index.js';

type CustomMap = TreeAdapterTypeMap<
  { kind: string },
  { kind: string; children: Array<{ kind: string }> },
  { kind: string },
  { kind: 'document'; children: Array<{ kind: string }> },
  { kind: 'fragment'; children: Array<{ kind: string }> },
  { kind: 'element'; children: Array<{ kind: string }> },
  { kind: 'comment' },
  { kind: 'text' },
  { kind: 'template'; children: Array<{ kind: string }> },
  { kind: 'doctype' }
>;

const customAdapter = defaultTreeAdapter as unknown as TreeAdapter<CustomMap>;
const parserOptions: ParserOptions<CustomMap> = { treeAdapter: customAdapter };
const serializerOptions: SerializerOptions<CustomMap> = { treeAdapter: customAdapter };

const document = parse<CustomMap>('<!doctype html>', parserOptions);
document.kind satisfies 'document';

const fragment = parseFragment<CustomMap>('<p>x</p>', parserOptions);
fragment.kind satisfies 'fragment';

const parser = new Parser<CustomMap>(parserOptions, document);
const fragmentParser = Parser.getFragmentParser<CustomMap>(null, parserOptions);
const stream = new ParserStream<CustomMap>(parserOptions, parser);
const fragmentStream = ParserStream.getFragmentStream<CustomMap>(null, parserOptions);
const plainTextStream = new PlainTextConversionStream<CustomMap>(parserOptions);
const sax = new SAXParser({ sourceCodeLocationInfo: true });
const feedback = new ParserFeedbackSimulator({}, {});
const rewriter = new RewritingStream();
const htmlparser2Document = parse<Htmlparser2TreeAdapterMap>('<p>x</p>', {
  treeAdapter: htmlparser2Adapter,
});

parser.document.kind satisfies 'document';
fragmentParser.getFragment().kind satisfies 'fragment';
stream.document.kind satisfies 'document';
fragmentStream.getFragment().kind satisfies 'fragment';
plainTextStream.document.kind satisfies 'document';
sax.on('startTag', (tag: StartTag) => tag.tagName satisfies string);
feedback.tokenizer.state satisfies number;
rewriter.on('text', (text, raw) => {
  text.text satisfies string;
  raw satisfies string;
});
htmlparser2Document.children satisfies Htmlparser2TreeAdapterMap['childNode'][];

serialize<CustomMap>(document, serializerOptions) satisfies string;
serializeOuter<CustomMap>({ kind: 'text' }, serializerOptions) satisfies string;
