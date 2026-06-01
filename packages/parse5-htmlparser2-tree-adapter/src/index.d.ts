import type { TreeAdapter, TreeAdapterTypeMap } from '../../parse5/src/index.js';
import type {
  AnyNode,
  ChildNode,
  Comment,
  Document,
  Element,
  ParentNode,
  ProcessingInstruction,
  Text,
} from './domhandler.js';

export type {
  AnyNode,
  CDATA,
  ChildNode,
  Comment,
  DataNode,
  Document,
  Element,
  Node,
  NodeWithChildren,
  ParentNode,
  ProcessingInstruction,
  Text,
} from './domhandler.js';
export { ElementType } from './domhandler.js';

export type Htmlparser2TreeAdapterMap = TreeAdapterTypeMap<
  AnyNode,
  ParentNode,
  ChildNode,
  Document,
  Document,
  Element,
  Comment,
  Text,
  Element,
  ProcessingInstruction
>;

export declare function serializeDoctypeContent(name: string, publicId: string, systemId: string): string;
export declare const adapter: TreeAdapter<Htmlparser2TreeAdapterMap>;
