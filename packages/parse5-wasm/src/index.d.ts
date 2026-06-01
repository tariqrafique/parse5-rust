export type DocumentMode = 'no-quirks' | 'quirks' | 'limited-quirks';

export interface Location {
  startLine: number;
  startCol: number;
  startOffset: number;
  endLine: number;
  endCol: number;
  endOffset: number;
}

export interface ElementLocation extends Location {
  attrs?: Record<string, Location>;
  startTag?: Location;
  endTag?: Location;
}

export interface ParserError extends Location {
  code: string;
}

export interface Attribute {
  name: string;
  value: string;
  namespace?: string;
  prefix?: string;
}

export type ParentNode = Document | DocumentFragment | Element | Template;
export type ChildNode = Element | Template | CommentNode | TextNode | DocumentType;
export type Node = ParentNode | ChildNode;

export interface Document {
  nodeName: '#document';
  mode: DocumentMode;
  childNodes: ChildNode[];
  sourceCodeLocation?: Location | null;
}

export interface DocumentFragment {
  nodeName: '#document-fragment';
  childNodes: ChildNode[];
  sourceCodeLocation?: Location | null;
}

export interface Element {
  nodeName: string;
  tagName: string;
  attrs: Attribute[];
  namespaceURI: string;
  parentNode: ParentNode | null;
  childNodes: ChildNode[];
  content?: DocumentFragment;
  sourceCodeLocation?: ElementLocation | null;
}

export interface Template extends Element {
  nodeName: 'template';
  tagName: 'template';
  content: DocumentFragment;
}

export interface CommentNode {
  nodeName: '#comment';
  data: string;
  parentNode: ParentNode | null;
  sourceCodeLocation?: Location | null;
}

export interface TextNode {
  nodeName: '#text';
  value: string;
  parentNode: ParentNode | null;
  sourceCodeLocation?: Location | null;
}

export interface DocumentType {
  nodeName: '#documentType';
  name: string;
  publicId: string;
  systemId: string;
  parentNode: ParentNode | null;
  sourceCodeLocation?: Location | null;
}

export type DefaultTreeAdapterMap = TreeAdapterTypeMap<
  Node,
  ParentNode,
  ChildNode,
  Document,
  DocumentFragment,
  Element,
  CommentNode,
  TextNode,
  Template,
  DocumentType
>;

export interface TreeAdapterTypeMap<
  TNode = unknown,
  TParentNode = unknown,
  TChildNode = unknown,
  TDocument = unknown,
  TDocumentFragment = unknown,
  TElement = unknown,
  TCommentNode = unknown,
  TTextNode = unknown,
  TTemplate = unknown,
  TDocumentType = unknown
> {
  node: TNode;
  parentNode: TParentNode;
  childNode: TChildNode;
  document: TDocument;
  documentFragment: TDocumentFragment;
  element: TElement;
  commentNode: TCommentNode;
  textNode: TTextNode;
  template: TTemplate;
  documentType: TDocumentType;
}

export interface TreeAdapter<T extends TreeAdapterTypeMap = TreeAdapterTypeMap> {
  adoptAttributes(recipient: T['element'], attrs: Attribute[]): void;
  appendChild(parentNode: T['parentNode'], newNode: T['childNode']): void;
  createCommentNode(data: string): T['commentNode'];
  createTextNode(value: string): T['textNode'];
  createDocument(): T['document'];
  createDocumentFragment(): T['documentFragment'];
  createElement(tagName: string, namespaceURI: string, attrs: Attribute[]): T['element'];
  detachNode(node: T['childNode']): void;
  getAttrList(element: T['element']): Attribute[];
  getChildNodes(node: T['parentNode']): T['childNode'][];
  getCommentNodeContent(commentNode: T['commentNode']): string;
  getDocumentMode(document: T['document']): DocumentMode;
  getDocumentTypeNodeName(doctypeNode: T['documentType']): string;
  getDocumentTypeNodePublicId(doctypeNode: T['documentType']): string;
  getDocumentTypeNodeSystemId(doctypeNode: T['documentType']): string;
  getFirstChild(node: T['parentNode']): T['childNode'] | null;
  getNamespaceURI(element: T['element']): string;
  getNodeSourceCodeLocation(node: T['node']): ElementLocation | undefined | null;
  getParentNode(node: T['node']): T['parentNode'] | null;
  getTagName(element: T['element']): string;
  getTextNodeContent(textNode: T['textNode']): string;
  getTemplateContent(templateElement: T['template']): T['documentFragment'];
  insertBefore(parentNode: T['parentNode'], newNode: T['childNode'], referenceNode: T['childNode']): void;
  insertText(parentNode: T['parentNode'], text: string): void;
  insertTextBefore(parentNode: T['parentNode'], text: string, referenceNode: T['childNode']): void;
  isCommentNode(node: T['node']): node is T['commentNode'];
  isDocumentTypeNode(node: T['node']): node is T['documentType'];
  isElementNode(node: T['node']): node is T['element'];
  isTextNode(node: T['node']): node is T['textNode'];
  setDocumentMode(document: T['document'], mode: DocumentMode): void;
  setDocumentType(document: T['document'], name: string, publicId: string, systemId: string): void;
  setNodeSourceCodeLocation(node: T['node'], location: ElementLocation | null): void;
  updateNodeSourceCodeLocation(node: T['node'], location: Partial<ElementLocation>): void;
  setTemplateContent(templateElement: T['template'], contentElement: T['documentFragment']): void;
  onItemPush?: (item: T['element']) => void;
  onItemPop?: (item: T['element'], newTop: T['parentNode']) => void;
}

export interface ParserOptions<T extends TreeAdapterTypeMap = DefaultTreeAdapterMap> {
  scriptingEnabled?: boolean;
  sourceCodeLocationInfo?: boolean;
  treeAdapter?: TreeAdapter<T>;
  onParseError?: ((error: ParserError) => void) | null;
}

export interface SerializerOptions<T extends TreeAdapterTypeMap = DefaultTreeAdapterMap> {
  treeAdapter?: TreeAdapter<T>;
  scriptingEnabled?: boolean;
}

export interface TokenizerOptions {
  sourceCodeLocationInfo?: boolean;
}

export interface DoctypeToken {
  type: 6;
  name: string | null;
  forceQuirks: boolean;
  publicId: string | null;
  systemId: string | null;
  location: Location | null;
}

export interface TagToken {
  type: 3 | 4;
  tagName: string;
  tagID: number;
  selfClosing: boolean;
  ackSelfClosing: boolean;
  attrs: Attribute[];
  location: (Location & { attrs?: Record<string, Location> }) | null;
}

export interface CommentToken {
  type: 5;
  data: string;
  location: Location | null;
}

export interface EOFToken {
  type: 7;
  location: Location | null;
}

export interface CharacterToken {
  type: 0 | 1 | 2;
  chars: string;
  location: Location | null;
}

export type TokenValue = DoctypeToken | TagToken | CommentToken | EOFToken | CharacterToken;

export interface TokenHandler {
  onComment?: (token: CommentToken) => void;
  onDoctype?: (token: DoctypeToken) => void;
  onStartTag?: (token: TagToken) => void;
  onEndTag?: (token: TagToken) => void;
  onEof?: (token: EOFToken) => void;
  onCharacter?: (token: CharacterToken) => void;
  onNullCharacter?: (token: CharacterToken) => void;
  onWhitespaceCharacter?: (token: CharacterToken) => void;
  onParseError?: (error: ParserError) => void;
}

export function initParse5Wasm(): Promise<unknown>;
export function parse<T extends TreeAdapterTypeMap = DefaultTreeAdapterMap>(
  html: string,
  options?: ParserOptions<T>
): T['document'];
export function parseFragment<T extends TreeAdapterTypeMap = DefaultTreeAdapterMap>(
  html: string,
  options?: ParserOptions<T>
): T['documentFragment'];
export function parseFragment<T extends TreeAdapterTypeMap = DefaultTreeAdapterMap>(
  fragmentContext: T['parentNode'] | null,
  html: string,
  options: ParserOptions<T>
): T['documentFragment'];
export function serialize<T extends TreeAdapterTypeMap = DefaultTreeAdapterMap>(
  node: T['parentNode'],
  options?: SerializerOptions<T>
): string;
export function serializeOuter<T extends TreeAdapterTypeMap = DefaultTreeAdapterMap>(
  node: T['node'],
  options?: SerializerOptions<T>
): string;

export class Parser<T extends TreeAdapterTypeMap = DefaultTreeAdapterMap> {
  treeAdapter: TreeAdapter<T>;
  onParseError: ((error: ParserError) => void) | null;
  options: Required<ParserOptions<T>>;
  document: T['document'];
  fragmentContext: T['element'] | null;
  scriptHandler: ((pendingScript: T['element']) => void) | null;
  stopped: boolean;
  tokenizer: {
    preprocessor: { bufferWaterline: number };
    paused: boolean;
    active: boolean;
    pause(): void;
    resume(writeCallback?: () => void): void;
    write(chunk: string, isLastChunk: boolean, writeCallback?: () => void): void;
    insertHtmlAtCurrentPos(chunk: string): void;
  };

  constructor(
    options?: ParserOptions<T>,
    document?: T['document'],
    fragmentContext?: T['element'] | null,
    scriptHandler?: ((pendingScript: T['element']) => void) | null
  );
  initPlainTextConversion(): void;
  static parse<T extends TreeAdapterTypeMap = DefaultTreeAdapterMap>(
    html: string,
    options?: ParserOptions<T>
  ): T['document'];
  static getFragmentParser<T extends TreeAdapterTypeMap = DefaultTreeAdapterMap>(
    fragmentContext?: T['parentNode'] | null,
    options?: ParserOptions<T>
  ): Parser<T>;
  getFragment(): T['documentFragment'];
}

export interface TokenizerPreprocessorFacade {
  bufferWaterline: number;
  readonly html: string;
  readonly droppedBufferSize: number;
  willDropParsedChunk(): boolean;
}

export class Tokenizer {
  preprocessor: TokenizerPreprocessorFacade;
  paused: boolean;
  inForeignNode: boolean;
  lastStartTagName: string;
  active: boolean;
  state: number;

  constructor(options?: TokenizerOptions, handler?: TokenHandler);
  pause(): void;
  resume(writeCallback?: () => void): void;
  write(chunk: string, isLastChunk: boolean, writeCallback?: () => void): void;
  insertHtmlAtCurrentPos(chunk: string): void;
}

export const ErrorCodes: Record<string, string>;
export const NS: Record<string, string>;
export const DOCUMENT_MODE: Record<string, string>;
export const ATTRS: Record<string, string>;
export const TAG_NAMES: Record<string, string>;
export const TAG_ID: Record<string | number, string | number>;
export const SPECIAL_ELEMENTS: Record<string, Set<number>>;
export const NUMBERED_HEADERS: Set<number>;
export function getTagID(tagName: string): number;
export function hasUnescapedText(tagName: string, scriptingEnabled: boolean): boolean;
export const TokenizerMode: Record<string, number>;
export const html: {
  NS: typeof NS;
  ATTRS: typeof ATTRS;
  DOCUMENT_MODE: typeof DOCUMENT_MODE;
  TAG_NAMES: typeof TAG_NAMES;
  TAG_ID: typeof TAG_ID;
  SPECIAL_ELEMENTS: typeof SPECIAL_ELEMENTS;
  NUMBERED_HEADERS: typeof NUMBERED_HEADERS;
  getTagID: typeof getTagID;
  hasUnescapedText: typeof hasUnescapedText;
};
export const foreignContent: {
  SVG_TAG_NAMES_ADJUSTMENT_MAP: Map<string, string>;
  causesExit(startTagToken: TagToken): boolean;
  adjustTokenMathMLAttrs(token: TagToken): void;
  adjustTokenSVGAttrs(token: TagToken): void;
  adjustTokenXMLAttrs(token: TagToken): void;
  adjustTokenSVGTagName(token: TagToken): void;
  isIntegrationPoint(tagID: number, namespaceURI: string, attrs: Attribute[], foreignNamespaceURI?: string): boolean;
};
export const Token: {
  TokenType: Record<string, number>;
  getTokenAttr(token: TagToken, attrName: string): string | null;
};
export const defaultTreeAdapter: TreeAdapter<DefaultTreeAdapterMap>;
