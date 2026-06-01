# LWC parse5 Requirements

This file captures the parse5 API surface that LWC currently needs from the local `reference/lwc` checkout.

## Source Dependencies

Only two LWC packages reference parse5-related packages directly:

- `packages/@lwc/template-compiler`
  - `devDependencies`: `parse5@^8.0.1`, `@parse5/tools@^0.7.0`
  - Rollup bundles both packages into the compiler output via `scripts/shared/bundled-dependencies.js`.
- `packages/@lwc/engine-server`
  - `devDependencies`: `parse5@^8.0.1`
  - Usage is limited to tests that parse and reserialize generated server HTML.

No LWC package imports these parse5 monorepo sibling packages:

- `parse5-parser-stream`
- `parse5-sax-parser`
- `parse5-html-rewriting-stream`
- `parse5-htmlparser2-tree-adapter`
- `parse5-plain-text-conversion-stream`

## Locked Versions

The current `reference/lwc/yarn.lock` resolves:

- `parse5@8.0.1`
- `entities@8.0.0`
- `@parse5/tools@0.7.0`

## Must-Port Runtime Surface

For LWC template compiler compatibility, the wasm-backed `parse5` package must provide:

- `parseFragment(html, options?)`
- `defaultTreeAdapter`
- `html.NS`
- `ErrorCodes`

For broader LWC tests, also provide:

- `serialize(node, options?)`

`parse` and `serializeOuter` are not directly imported by LWC source, but they are part of the expected parse5 package surface and should remain in scope for parse5 compatibility tests.

## parseFragment Requirements

`@lwc/template-compiler/src/parser/html.ts` calls:

```ts
parse5.parseFragment(source, {
    sourceCodeLocationInfo: true,
    onParseError,
});
```

That means the port must support:

- fragment parsing without an explicit context element
- `sourceCodeLocationInfo: true`
- `onParseError` callbacks
- default tree adapter output

LWC relies heavily on exact offsets. It slices the original template source using parse5 locations to recover raw text and raw attributes, so decoded tree values are not enough.

## Node Shape Requirements

The default tree adapter must produce parse5-compatible nodes:

- document fragment: `{ nodeName: '#document-fragment', childNodes }`
- element: `{ nodeName, tagName, attrs, namespaceURI, parentNode, childNodes, sourceCodeLocation? }`
- template element: normal element fields plus `content`
- text: `{ nodeName: '#text', value, parentNode, sourceCodeLocation? }`
- comment: `{ nodeName: '#comment', data, parentNode, sourceCodeLocation? }`
- document type: `{ nodeName: '#documentType', name, publicId, systemId, parentNode, sourceCodeLocation? }`

Parent links are required. `@parse5/tools` and LWC walk from recovered implicit nodes back to their element parents.

## Location Requirements

LWC consumes these parse5 location fields:

- `startLine`
- `startCol`
- `startOffset`
- `endLine`
- `endCol`
- `endOffset`
- `ElementLocation.startTag`
- `ElementLocation.endTag`
- `ElementLocation.attrs`

Attribute locations are required for every parsed attribute. LWC indexes the attribute location map by `attributeName(attr).toLowerCase()`, where `attributeName` returns `prefix ? `${prefix}:${name}` : name`.

Important LWC behaviors tied to locations:

- root `<template>` must have `sourceCodeLocation`
- text and comment nodes must have `sourceCodeLocation`
- raw text is read with `source.slice(location.startOffset, location.endOffset)`
- raw attribute text is read with each attribute location
- implicit/recovered elements may have missing location; LWC warns and falls back to ancestor/parent locations

## Attribute Requirements

`Token.Attribute` shape must match parse5:

```ts
{
    name: string;
    value: string;
    namespace?: string;
    prefix?: string;
}
```

LWC depends on:

- lowercased HTML attribute names for case validation
- exact attribute values after parse5 entity decoding
- `prefix` and `namespace` for foreign/SVG attributes such as `xlink:href`
- duplicate attribute parse errors

## @parse5/tools Usage

LWC imports `@parse5/tools` as a namespace and uses:

- runtime: `isElementNode`
- runtime: `isTextNode`
- runtime: `isCommentNode`
- runtime: `isTemplateNode`
- type-only: `Element`
- type-only: `Template`
- type-only: `ParentNode`
- type-only: `DocumentFragment`
- type-only: `TextNode`
- type-only: `CommentNode`

`@parse5/tools@0.7.0` implements its runtime type guards mostly by re-exporting `parse5.defaultTreeAdapter` methods. We do not need to port the whole package for LWC, but our `defaultTreeAdapter` must be compatible enough that the published `@parse5/tools` package works with our parse5 replacement.

Because LWC bundles `@parse5/tools`, a local source rebuild may also touch the package's other exports. To be robust, `defaultTreeAdapter` should include the standard parse5 methods used by `@parse5/tools`: node creation, `appendChild`, `detachNode`, and node type guards.

## Error Code Requirements

LWC's `error-codes.spec.ts` asserts every value in `parse5.ErrorCodes` is listed in `parser/parse5Errors.ts`. For parse5 `8.0.1`, there are exactly 60 codes and LWC accounts for all of them.

The port must not add extra `ErrorCodes` values for LWC compatibility, and must emit parse errors with parse5-compatible:

- `code`
- `startLine`
- `startCol`
- `startOffset`
- `endLine`
- `endCol`
- `endOffset`

## Serialization Requirements

`@lwc/engine-server` tests import:

```ts
import { parseFragment, serialize } from 'parse5';
```

They parse generated server HTML and compare `serialize(parseFragment(html))` against expected normalized HTML. This exercises:

- void HTML element serialization
- SVG serialization
- namespace-sensitive serialization
- default tree adapter serialization

This is test-only in LWC, but it should be included in the compatibility target.

## Out of Scope for LWC Milestone

These parse5 features are not required by direct LWC usage:

- streaming parser APIs
- SAX parser package
- HTML rewriting stream package
- htmlparser2 tree adapter package
- plain text conversion stream package
- custom tree adapter support beyond keeping parse5's default adapter behavior compatible with `@parse5/tools`
