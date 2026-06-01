export const ElementType = {
  Root: 'root',
  Text: 'text',
  Directive: 'directive',
  Comment: 'comment',
  Script: 'script',
  Style: 'style',
  Tag: 'tag',
  CDATA: 'cdata',
  Doctype: 'doctype',
};

export class Node {
  parent = null;
  prev = null;
  next = null;
  startIndex = null;
  endIndex = null;

  get parentNode() {
    return this.parent;
  }

  set parentNode(parent) {
    this.parent = parent;
  }

  get previousSibling() {
    return this.prev;
  }

  set previousSibling(previous) {
    this.prev = previous;
  }

  get nextSibling() {
    return this.next;
  }

  set nextSibling(next) {
    this.next = next;
  }

  cloneNode(recursive = false) {
    return cloneNode(this, recursive);
  }
}

export class DataNode extends Node {
  constructor(data) {
    super();
    this.data = data;
  }

  get nodeValue() {
    return this.data;
  }

  set nodeValue(data) {
    this.data = data;
  }
}

export class Text extends DataNode {
  type = ElementType.Text;

  get nodeType() {
    return 3;
  }
}

export class Comment extends DataNode {
  type = ElementType.Comment;

  get nodeType() {
    return 8;
  }
}

export class ProcessingInstruction extends DataNode {
  type = ElementType.Directive;

  constructor(name, data) {
    super(data);
    this.name = name;
  }

  get nodeType() {
    return 1;
  }
}

export class NodeWithChildren extends Node {
  constructor(children) {
    super();
    this.children = children;
  }

  get firstChild() {
    return this.children[0] ?? null;
  }

  get lastChild() {
    return this.children.length > 0 ? this.children[this.children.length - 1] : null;
  }

  get childNodes() {
    return this.children;
  }

  set childNodes(children) {
    this.children = children;
  }
}

export class CDATA extends NodeWithChildren {
  type = ElementType.CDATA;

  get nodeType() {
    return 4;
  }
}

export class Document extends NodeWithChildren {
  type = ElementType.Root;

  get nodeType() {
    return 9;
  }
}

export class Element extends NodeWithChildren {
  constructor(
    name,
    attribs,
    children = [],
    type = name === 'script' ? ElementType.Script : name === 'style' ? ElementType.Style : ElementType.Tag
  ) {
    super(children);
    this.name = name;
    this.attribs = attribs;
    this.type = type;
  }

  get nodeType() {
    return 1;
  }

  get tagName() {
    return this.name;
  }

  set tagName(name) {
    this.name = name;
  }

  get attributes() {
    return Object.keys(this.attribs).map((name) => ({
      name,
      value: this.attribs[name],
      namespace: this['x-attribsNamespace']?.[name],
      prefix: this['x-attribsPrefix']?.[name],
    }));
  }
}

export function isTag(node) {
  return node?.type === ElementType.Tag || node?.type === ElementType.Script || node?.type === ElementType.Style;
}

export function isCDATA(node) {
  return node?.type === ElementType.CDATA;
}

export function isText(node) {
  return node?.type === ElementType.Text;
}

export function isComment(node) {
  return node?.type === ElementType.Comment;
}

export function isDirective(node) {
  return node?.type === ElementType.Directive;
}

export function isDocument(node) {
  return node?.type === ElementType.Root;
}

export function hasChildren(node) {
  return Object.hasOwn(node, 'children');
}

export function cloneNode(node, recursive = false) {
  let result;

  if (isText(node)) {
    result = new Text(node.data);
  } else if (isComment(node)) {
    result = new Comment(node.data);
  } else if (isTag(node)) {
    const children = recursive ? cloneChildren(node.children) : [];
    result = new Element(node.name, { ...node.attribs }, children);

    for (const child of children) {
      child.parent = result;
    }

    if (node.namespace != null) {
      result.namespace = node.namespace;
    }

    if (node['x-attribsNamespace']) {
      result['x-attribsNamespace'] = { ...node['x-attribsNamespace'] };
    }

    if (node['x-attribsPrefix']) {
      result['x-attribsPrefix'] = { ...node['x-attribsPrefix'] };
    }
  } else if (isCDATA(node)) {
    const children = recursive ? cloneChildren(node.children) : [];
    result = new CDATA(children);

    for (const child of children) {
      child.parent = result;
    }
  } else if (isDocument(node)) {
    const children = recursive ? cloneChildren(node.children) : [];
    result = new Document(children);

    for (const child of children) {
      child.parent = result;
    }

    if (node['x-mode']) {
      result['x-mode'] = node['x-mode'];
    }
  } else if (isDirective(node)) {
    result = new ProcessingInstruction(node.name, node.data);

    if (node['x-name'] != null) {
      result['x-name'] = node['x-name'];
      result['x-publicId'] = node['x-publicId'];
      result['x-systemId'] = node['x-systemId'];
    }
  } else {
    throw new Error(`Not implemented yet: ${node.type}`);
  }

  result.startIndex = node.startIndex;
  result.endIndex = node.endIndex;

  if (node.sourceCodeLocation != null) {
    result.sourceCodeLocation = node.sourceCodeLocation;
  }

  return result;
}

function cloneChildren(childs) {
  const children = childs.map((child) => cloneNode(child, true));

  for (let index = 1; index < children.length; index++) {
    children[index].prev = children[index - 1];
    children[index - 1].next = children[index];
  }

  return children;
}
