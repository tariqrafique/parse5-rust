export declare const ElementType: {
  readonly Root: 'root';
  readonly Text: 'text';
  readonly Directive: 'directive';
  readonly Comment: 'comment';
  readonly Script: 'script';
  readonly Style: 'style';
  readonly Tag: 'tag';
  readonly CDATA: 'cdata';
  readonly Doctype: 'doctype';
};

export type ElementTypeValue = (typeof ElementType)[keyof typeof ElementType];

export declare class Node {
  parent: ParentNode | null;
  prev: ChildNode | null;
  next: ChildNode | null;
  startIndex: number | null;
  endIndex: number | null;
  sourceCodeLocation?: unknown;
  get parentNode(): ParentNode | null;
  set parentNode(parent: ParentNode | null);
  get previousSibling(): ChildNode | null;
  set previousSibling(previous: ChildNode | null);
  get nextSibling(): ChildNode | null;
  set nextSibling(next: ChildNode | null);
  cloneNode(recursive?: boolean): AnyNode;
}

export declare class DataNode extends Node {
  data: string;
  constructor(data: string);
  get nodeValue(): string;
  set nodeValue(data: string);
}

export declare class Text extends DataNode {
  type: typeof ElementType.Text;
  get nodeType(): 3;
}

export declare class Comment extends DataNode {
  type: typeof ElementType.Comment;
  get nodeType(): 8;
}

export declare class ProcessingInstruction extends DataNode {
  type: typeof ElementType.Directive;
  name: string;
  constructor(name: string, data: string);
  get nodeType(): 1;
}

export declare class NodeWithChildren extends Node {
  children: ChildNode[];
  constructor(children: ChildNode[]);
  get firstChild(): ChildNode | null;
  get lastChild(): ChildNode | null;
  get childNodes(): ChildNode[];
  set childNodes(children: ChildNode[]);
}

export declare class CDATA extends NodeWithChildren {
  type: typeof ElementType.CDATA;
  get nodeType(): 4;
}

export declare class Document extends NodeWithChildren {
  type: typeof ElementType.Root;
  'x-mode'?: string;
  get nodeType(): 9;
}

export declare class Element extends NodeWithChildren {
  name: string;
  attribs: Record<string, string>;
  type: typeof ElementType.Tag | typeof ElementType.Script | typeof ElementType.Style;
  namespace?: string;
  'x-attribsNamespace'?: Record<string, string | undefined>;
  'x-attribsPrefix'?: Record<string, string | undefined>;
  constructor(name: string, attribs: Record<string, string>, children?: ChildNode[], type?: Element['type']);
  get nodeType(): 1;
  get tagName(): string;
  set tagName(name: string);
  get attributes(): Array<{ name: string; value: string; namespace?: string; prefix?: string }>;
}

export type AnyNode = Document | Element | Text | Comment | ProcessingInstruction | CDATA;
export type ParentNode = Document | Element | CDATA;
export type ChildNode = Element | Text | Comment | ProcessingInstruction | CDATA;

export declare function isTag(node: unknown): node is Element;
export declare function isCDATA(node: unknown): node is CDATA;
export declare function isText(node: unknown): node is Text;
export declare function isComment(node: unknown): node is Comment;
export declare function isDirective(node: unknown): node is ProcessingInstruction;
export declare function isDocument(node: unknown): node is Document;
export declare function hasChildren(node: unknown): node is ParentNode;
export declare function cloneNode(node: AnyNode, recursive?: boolean): AnyNode;
