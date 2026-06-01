export interface Attribute {
  name: string;
  namespace?: string;
  prefix?: string;
  value: string;
}

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

export interface Document {
  nodeName: '#document';
  mode: string;
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
  sourceCodeLocation?: ElementLocation | null;
  parentNode: ParentNode | null;
  childNodes: ChildNode[];
}

export interface Template extends Element {
  nodeName: 'template';
  tagName: 'template';
  content: DocumentFragment;
}

export interface CommentNode {
  nodeName: '#comment';
  parentNode: ParentNode | null;
  data: string;
  sourceCodeLocation?: Location | null;
}

export interface TextNode {
  nodeName: '#text';
  parentNode: ParentNode | null;
  value: string;
  sourceCodeLocation?: Location | null;
}

export interface DocumentType {
  nodeName: '#documentType';
  parentNode: ParentNode | null;
  name: string;
  publicId: string;
  systemId: string;
  sourceCodeLocation?: Location | null;
}

export type ParentNode = Document | DocumentFragment | Element | Template;
export type ChildNode = Element | Template | CommentNode | TextNode | DocumentType;
export type Node = ParentNode | ChildNode;

export declare const NS: {
  HTML: string;
  XML: string;
  MATHML: string;
  SVG: string;
  XLINK: string;
  XMLNS: string;
};

export function setAttribute(node: Element, name: string, value: string): void;
export function getAttribute(node: Element, name: string): string | null;
export function hasAttribute(node: Element, name: string): boolean;
export function removeAttribute(node: Element, name: string): void;
export function getAttributeIndex(node: Element, name: string): number;

export function createElement(
  tagName: string,
  attrs?: Record<string, string> | Attribute[],
  namespaceURI?: string
): Element;
export function createTextNode(value: string): TextNode;
export function createDocumentFragment(): DocumentFragment;
export function createTemplateNode(content?: DocumentFragment): Template;
export function createDocument(): Document;
export function createCommentNode(data: string): CommentNode;

export function isDocument(node: Node): node is Document;
export function isDocumentFragment(node: Node): node is DocumentFragment;
export function isTemplateNode(node: Node): node is Template;
export function isElementNode(node: Node): node is Element;
export function isCommentNode(node: Node): node is CommentNode;
export function isDocumentTypeNode(node: Node): node is DocumentType;
export function isTextNode(node: Node): node is TextNode;
export function isParentNode(node: Node): node is ParentNode;
export function isChildNode(node: Node): node is ChildNode;

export function getTextContent(node: Node): string;
export function setTextContent(node: Node, text: string): void;

export function query<T extends Node = Node>(root: Node, condition: (node: Node) => boolean): T | null;
export function walkChildren(node: Node): IterableIterator<Node>;
export function queryAll<T extends Node = Node>(
  root: Node,
  condition?: (node: Node) => boolean
): IterableIterator<T>;
export function ancestors(node: Node): IterableIterator<Node>;
export function previousSiblings(node: Node): IterableIterator<Node>;
export function nextSiblings(node: Node): IterableIterator<Node>;

export interface Visitor {
  'pre:node'?: (node: Node, parent?: ParentNode) => boolean | void;
  node?: (node: Node, parent?: ParentNode) => void;
  document?: (node: Document) => void;
  documentFragment?: (node: DocumentFragment, parent?: ParentNode) => void;
  element?: (node: Element, parent?: ParentNode) => void;
  template?: (node: Template, parent?: ParentNode) => void;
  comment?: (node: CommentNode, parent?: ParentNode) => void;
  text?: (node: TextNode, parent?: ParentNode) => void;
  documentType?: (node: DocumentType, parent?: ParentNode) => void;
}

export function traverse(node: Node, visitor: Visitor, parent?: ParentNode): void;

export function removeNode(node: Node): void;
export function appendChild(parentNode: ParentNode, newNode: ChildNode): void;
export function spliceChildren(
  node: Node,
  start: number,
  deleteCount: number,
  ...children: ChildNode[]
): ChildNode[];
export function replaceWith(node: ChildNode, ...replacements: ChildNode[]): void;
