export const NS = {
  HTML: 'http://www.w3.org/1999/xhtml',
  XML: 'http://www.w3.org/XML/1998/namespace',
  MATHML: 'http://www.w3.org/1998/Math/MathML',
  SVG: 'http://www.w3.org/2000/svg',
  XLINK: 'http://www.w3.org/1999/xlink',
  XMLNS: 'http://www.w3.org/2000/xmlns/',
};

const namespaceMap = {
  HTML: NS.HTML,
  XML: NS.XML,
  MATHML: NS.MATHML,
  SVG: NS.SVG,
  XLINK: NS.XLINK,
  XMLNS: NS.XMLNS,
};

export function setAttribute(node, name, value) {
  removeAttribute(node, name);
  node.attrs.push({ name, value });
}

export function getAttribute(node, name) {
  return node.attrs.find((attr) => attr.name === name)?.value ?? null;
}

export function hasAttribute(node, name) {
  return node.attrs.some((attr) => attr.name === name);
}

export function removeAttribute(node, name) {
  const index = getAttributeIndex(node, name);

  if (index > -1) {
    node.attrs.splice(index, 1);
  }
}

export function getAttributeIndex(node, name) {
  return node.attrs.findIndex((attr) => attr.name === name);
}

export function createElement(tagName, attrs = [], namespaceURI = NS.HTML) {
  const normalisedAttrs = [];
  const normalisedNamespace = namespaceMap[String(namespaceURI).toUpperCase()] ?? namespaceURI;

  if (Array.isArray(attrs)) {
    for (const attr of attrs) {
      normalisedAttrs.push(attr);
    }
  } else {
    for (const name in attrs) {
      if (Object.prototype.hasOwnProperty.call(attrs, name)) {
        normalisedAttrs.push({ name, value: attrs[name] });
      }
    }
  }

  return {
    nodeName: tagName,
    tagName,
    attrs: normalisedAttrs,
    namespaceURI: normalisedNamespace,
    childNodes: [],
    parentNode: null,
  };
}

export function createTextNode(value) {
  return {
    nodeName: '#text',
    value,
    parentNode: null,
  };
}

export function createDocumentFragment() {
  return {
    nodeName: '#document-fragment',
    childNodes: [],
  };
}

export function createTemplateNode(content) {
  return {
    nodeName: 'template',
    tagName: 'template',
    content: content ?? createDocumentFragment(),
    parentNode: null,
    attrs: [],
    namespaceURI: NS.HTML,
    childNodes: [],
  };
}

export function createDocument() {
  return {
    nodeName: '#document',
    mode: 'no-quirks',
    childNodes: [],
  };
}

export function createCommentNode(data) {
  return {
    nodeName: '#comment',
    data,
    parentNode: null,
  };
}

export function isDocument(node) {
  return node?.nodeName === '#document';
}

export function isDocumentFragment(node) {
  return node?.nodeName === '#document-fragment';
}

export function isTemplateNode(node) {
  return node?.nodeName === 'template';
}

export function isElementNode(node) {
  return Object.prototype.hasOwnProperty.call(node ?? {}, 'tagName');
}

export function isCommentNode(node) {
  return node?.nodeName === '#comment';
}

export function isDocumentTypeNode(node) {
  return node?.nodeName === '#documentType';
}

export function isTextNode(node) {
  return node?.nodeName === '#text';
}

export function isParentNode(node) {
  return isDocument(node) || isDocumentFragment(node) || isElementNode(node) || isTemplateNode(node);
}

export function isChildNode(node) {
  return (
    isElementNode(node) ||
    isTemplateNode(node) ||
    isCommentNode(node) ||
    isTextNode(node) ||
    isDocumentTypeNode(node)
  );
}

export function getTextContent(node) {
  if (isCommentNode(node)) {
    return node.data;
  }

  if (isTextNode(node)) {
    return node.value;
  }

  let content = '';

  for (const child of queryAll(node, isTextNode)) {
    content += getTextContent(child);
  }

  return content;
}

export function setTextContent(node, text) {
  if (isCommentNode(node)) {
    node.data = text;
  } else if (isTextNode(node)) {
    node.value = text;
  } else if (isParentNode(node)) {
    node.childNodes = [];
    appendChild(node, createTextNode(text));
  }
}

export function query(root, condition) {
  for (const child of queryAll(root, condition)) {
    return child;
  }

  return null;
}

export function* walkChildren(node) {
  if (isParentNode(node)) {
    for (const child of node.childNodes) {
      yield child;
      yield* walkChildren(child);
    }
  }
}

export function* queryAll(root, condition) {
  for (const child of walkChildren(root)) {
    if (!condition || condition(child)) {
      yield child;
    }
  }
}

export function* ancestors(node) {
  let current = node;

  while (current !== null) {
    yield current;
    current = isChildNode(current) ? current.parentNode : null;
  }
}

export function* previousSiblings(node) {
  if (!isChildNode(node) || !node.parentNode) {
    return;
  }

  const children = node.parentNode.childNodes;
  const index = children.indexOf(node);

  for (let i = index - 1; i >= 0; i--) {
    yield children[i];
  }
}

export function* nextSiblings(node) {
  if (!isChildNode(node) || !node.parentNode) {
    return;
  }

  const children = node.parentNode.childNodes;
  const index = children.indexOf(node);

  for (let i = index + 1; i < children.length; i++) {
    yield children[i];
  }
}

export function traverse(node, visitor, parent) {
  const shouldVisitChildren =
    typeof visitor['pre:node'] !== 'function' || visitor['pre:node'](node, parent) !== false;

  if (shouldVisitChildren && isParentNode(node)) {
    for (const child of node.childNodes) {
      traverse(child, visitor, node);
    }
  }

  if (typeof visitor.node === 'function') {
    visitor.node(node, parent);
  }
  if (typeof visitor.document === 'function' && isDocument(node)) {
    visitor.document(node);
  }
  if (typeof visitor.documentFragment === 'function' && isDocumentFragment(node)) {
    visitor.documentFragment(node, parent);
  }
  if (typeof visitor.element === 'function' && isElementNode(node)) {
    visitor.element(node, parent);
  }
  if (typeof visitor.template === 'function' && isTemplateNode(node)) {
    visitor.template(node, parent);
  }
  if (typeof visitor.comment === 'function' && isCommentNode(node)) {
    visitor.comment(node, parent);
  }
  if (typeof visitor.text === 'function' && isTextNode(node)) {
    visitor.text(node, parent);
  }
  if (typeof visitor.documentType === 'function' && isDocumentTypeNode(node)) {
    visitor.documentType(node, parent);
  }
}

export function removeNode(node) {
  if (!isChildNode(node) || !node.parentNode) {
    return;
  }

  const index = node.parentNode.childNodes.indexOf(node);

  if (index > -1) {
    node.parentNode.childNodes.splice(index, 1);
  }

  node.parentNode = null;
}

export function appendChild(parentNode, newNode) {
  parentNode.childNodes.push(newNode);
  newNode.parentNode = parentNode;
}

export function spliceChildren(node, start, deleteCount, ...children) {
  if (isParentNode(node)) {
    return node.childNodes.splice(start, deleteCount, ...children);
  }

  return [];
}

export function replaceWith(node, ...replacements) {
  if (node.parentNode) {
    const parentNode = node.parentNode;
    const index = parentNode.childNodes.indexOf(node);

    if (index > -1) {
      spliceChildren(parentNode, index, 1, ...replacements);

      for (const replacement of replacements) {
        replacement.parentNode = parentNode;
      }
    }

    node.parentNode = null;
  }
}
