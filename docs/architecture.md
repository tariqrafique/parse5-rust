# Architecture

This repository ports [`parse5`](https://github.com/inikulin/parse5) to Rust and ships it to JavaScript through WebAssembly. The compatibility target is `parse5@8.0.1`, in particular the surface `@lwc/template-compiler` uses (see [`lwc-parse5-requirements.md`](lwc-parse5-requirements.md)).

There are three layers:

```
 ┌────────────────────────────────────────────────────────────────────────┐
 │ npm packages (packages/*)                                              │
 │   parse5 · @parse5/tools · parse5-parser-stream · parse5-sax-parser    │
 │   parse5-html-rewriting-stream · parse5-plain-text-conversion-stream   │
 │   parse5-htmlparser2-tree-adapter                                      │
 │                 │ all import packages/parse5 → packages/parse5-wasm    │
 ├─────────────────┼──────────────────────────────────────────────────────┤
 │ JS bridge (packages/parse5-wasm/src/index.js)                          │
 │   parse5 API, constants, defaultTreeAdapter, serializer (pure JS),     │
 │   Parser/Tokenizer classes that drive native objects                   │
 │                 │ wasm-bindgen: strings in, JSON out                   │
 ├─────────────────┼──────────────────────────────────────────────────────┤
 │ Rust                                                                   │
 │   crates/parse5-wasm   bindings; tree/token → JSON                     │
 │   crates/parse5        tokenizer, tree builder, serializer, tree model │
 └────────────────────────────────────────────────────────────────────────┘
```

## Rust core: `crates/parse5`

The module layout mirrors upstream `packages/parse5/lib`, so a Rust file and its TypeScript counterpart can be read side by side.

| Module | Upstream | Role |
|---|---|---|
| `tokenizer/preprocessor.rs` | `tokenizer/preprocessor.ts` | Input buffer as UTF-16 code units; CR/LF normalization, surrogate handling, line/column tracking, chunked input, dropping parsed chunks |
| `tokenizer/mod.rs` | `tokenizer/index.ts` | The HTML tokenizer state machine (73 states) and character references |
| `parser/mod.rs` | `parser/index.ts` | Tree construction: insertion modes, adoption agency, foster parenting, templates, fragment parsing, location bookkeeping |
| `parser/open_element_stack.rs` | `parser/open-element-stack.ts` | Stack of open elements with scope queries |
| `parser/formatting_element_list.rs` | `parser/formatting-element-list.ts` | Active formatting elements, markers, Noah's Ark clause |
| `serializer/mod.rs` | `serializer/index.ts` | HTML serialization (`serialize`, `serialize_outer`) |
| `tree_adapters/default.rs` | `tree-adapters/default.ts` | The tree model and default-adapter free functions |
| `tree_adapters/interface.rs` | `tree-adapters/interface.ts` | `TreeAdapter` trait (defined, **not used by the parser**, see below) |
| `common/*` | `common/*` | Tag/namespace tables, error codes, doctype quirks, foreign-content adjustments, tokens, named character reference index |

### Data flow

```
&str ──encode_utf16──▶ Preprocessor ──code points──▶ Tokenizer ──Token──▶ Parser ──▶ NodeRef tree
                         (Vec<u16>)                   state machine        insertion modes
```

The parser pulls tokens from the tokenizer (`run_parsing_loop`) and gives it feedback. It switches tokenizer states for `<script>`, `<textarea>` and similar, and sets `in_foreign_node` for CDATA handling, exactly as upstream does.

**Offsets are UTF-16.** The preprocessor works on UTF-16 code units, not Rust `char`s, so every `startOffset`/`startCol` in errors and `sourceCodeLocation` matches what JavaScript parse5 reports. Lone surrogates are preserved rather than replaced. The cost is one transcoding of the input.

### Tree model

```rust
pub type NodeRef = Rc<RefCell<Node>>;
pub struct Node { parent: Option<Weak<RefCell<Node>>>, source_code_location: Option<ElementLocation>, pub data: NodeData, .. }
pub enum NodeData { Document{..}, DocumentFragment{..}, Element{ tag_name, attrs, namespace_uri, child_nodes, template_content }, Comment{..}, Text{..}, DocumentType{..} }
```

- Children are owned (`Vec<NodeRef>`); parents are `Weak` back-pointers, so the tree has no reference cycles.
- `Node` has a hand-written iterative `Drop`, and the serializer uses an explicit work stack. Deeply nested documents (the parsing algorithm will happily nest 100k `<div>`s) therefore cannot overflow the stack when they are freed or serialized. `tests/robustness.rs` guards this.
- `NodeData` holds owned `String`s for tag names and namespaces. That is simple, but it costs several heap allocations per element. See [performance](#performance).

**The parser is hard-wired to this model.** Upstream parse5 builds whatever a `TreeAdapter` produces. Here the `TreeAdapter` trait exists and `DefaultTreeAdapter` implements it, but `Parser` calls the default-tree free functions directly. Custom adapters are emulated in JavaScript (see below).

### Streaming and parser feedback

`Parser::write_chunk(html, is_last_chunk)` feeds input incrementally. When the tokenizer needs more input than it has (a tag, or a character reference split across chunks), it *hibernates*: it rewinds to the start of the construct and resumes on the next write. `Parser` also records:

- `errors`: parse5 error codes with UTF-16 locations.
- `stack_events`: every push to and pop from the stack of open elements. These drive `onItemPush`/`onItemPop` for custom tree adapters in JS. They are recorded unconditionally, even for a plain `parse()`.
- `script_events`: pause points for `parse5-parser-stream`'s `script` event and `document.write` emulation.

## wasm bindings: `crates/parse5-wasm`

The bindings are wasm-bindgen exports over the core crate:

| Export | Used by |
|---|---|
| `parse`, `parseWithErrors`, `parseFragment[WithContext][AndErrors]` | the one-shot `parse()` / `parseFragment()` in JS |
| `WasmParser` (`write`, `resume`, `insertHtmlAtCurrentPos`, `document`, `getFragment`, …) | the JS `Parser` class, and through it `parse5-parser-stream` and `parse5-plain-text-conversion-stream` |
| `WasmTokenizer` (`writeInput`, `nextToken`, state getters/setters) | the JS `Tokenizer` class, and through it `parse5-sax-parser` and `parse5-html-rewriting-stream` |

**Results cross the boundary as JSON.** `node_json.rs` streams the Rust tree directly to a JSON string through `serde::Serialize` impls, and the engine's native `JSON.parse` decodes it (`js_sys::JSON::parse`). Keys come out in the same order as upstream parse5's default tree adapter, so `Object.keys()` and `JSON.stringify()` of a node match the reference. Tokens, errors and stack events still go through `serde_json::Value` and are built per call.

Node references in events (stack events, script pauses) are sent as *paths* from the root, for example `[0, 1, "content", 3]`, because Rust nodes have no JS identity.

## JS bridge: `packages/parse5-wasm`

`src/index.js` is the parse5 API as JavaScript sees it. It must be initialized once (`await initParse5Wasm()`); the `packages/parse5` facade does this with top-level `await` so importing it behaves like importing parse5.

- **`parse` / `parseFragment` with the default adapter:** call the wasm export, `JSON.parse` the result, then `attachParentLinks` walks the tree to set `parentNode`. The returned objects are the default-adapter node shapes upstream produces.
- **Custom `treeAdapter`** (for example `parse5-htmlparser2-tree-adapter`): parse to the default tree as above, then `transformDefaultTreeNode` rebuilds it node by node through the adapter's `create*`/`appendChild`/`setTemplateContent` methods. `onItemPush`/`onItemPop` are replayed from the recorded stack events after parsing, not interleaved with tree construction as upstream does.
- **`Parser` class (streaming):** wraps a `WasmParser`. After every `write()`/`resume()` it re-exports the entire current document from wasm and re-adapts it, then replays events.
- **`Tokenizer` class:** wraps a `WasmTokenizer`, pulling one token per call. JS mirrors the preprocessor state (`preprocessor.html`, `pos`, `droppedBufferSize`) because `parse5-sax-parser` and `parse5-html-rewriting-stream` read raw source slices from it.
- **`serialize` / `serializeOuter`:** pure JavaScript over the JS tree. Serializing never crosses into wasm. The Rust serializer is used by the Rust tests and `parseFragmentAndSerialize` only.
- Constants and helpers (`html.TAG_ID`, `ErrorCodes`, `foreignContent`, `Token`, …) are plain JS copies of upstream's.

The sibling packages under `packages/` are thin JS ports of their upstream counterparts, built on the classes above.

## Testing

| Suite | What it checks | Needs |
|---|---|---|
| `cargo test` | html5lib tree-construction (incl. error codes; parse5's 6 expected foreign-fragment failures must still fail), html5lib tokenizer, parse5 location-info and serialization fixtures, robustness | `fixtures/` only |
| `npm test` | wasm facade, serialization goldens, typecheck, every sibling package, fixture inventory | a built wasm package |
| `npm run test:wasm-runtime` (first step of `npm test`) | export surface and output parity against **upstream parse5** | built `reference/parse5` (see README) |
| LWC integration | `@lwc/template-compiler` tests against the port | `reference/lwc` (see README) |

Fixtures are vendored under `fixtures/parse5/test/data`. `reference/` holds upstream checkouts for reading and parity runs only, and tests must not depend on its fixture data.

## Performance

Numbers below come from one Linux x86-64 container (Node 22, Rust 1.98 release with LTO) on the 216 KiB `location-info/wiki-42` fixture unless noted. Absolute times vary by machine; the ratios are the point. To reproduce:

```sh
npm run build:wasm
npm run bench:wasm                       # or: node scripts/bench-wasm.mjs other.html
```

Upstream columns need the built reference checkout described in the README.

### Native Rust

| | Time |
|---|---|
| `parse` | 11.6 ms |
| `parse` + `source_code_location_info` | 14.7 ms |
| `serialize` | 0.6 ms |

This is on par with or slightly faster than upstream parse5 running natively in V8 (about 12 ms and 16 ms). Throughput scales linearly with input size. The core crate is not the bottleneck.

### Through wasm, as JS callers see it

| Scenario | Upstream parse5 | wasm port | Port ÷ upstream |
|---|---|---|---|
| LWC-shaped: 1 KiB `parseFragment` + locations + `onParseError` | 0.05 ms | 0.15 ms | **3.2×** |
| `parse` | 12.1 ms | 33.4 ms | **2.8×** |
| `parse` + `sourceCodeLocationInfo` | 16.4 ms | 75.5 ms | **4.6×** |
| `parseFragment` | 12.2 ms | 32.2 ms | **2.6×** |
| `Parser`, 64 KiB chunks (stream path) | 12.4 ms | 2,993 ms | **242×** |
| `Parser`, 4 KiB chunks | 14.6 ms | 3,357 ms | **230×** |
| `Tokenizer` (SAX path) | 7.8 ms | 101.6 ms | **13×** |
| `serialize` (pure JS on both sides) | 1.9 ms | 4.5 ms | 2.3× |

Where the time goes in one wasm `parse`:

| Step | Default | With locations |
|---|---|---|
| Rust parse inside wasm (about 1.7× native) | ~19 ms | ~25 ms (est.) |
| Encode tree to JSON, copy it out and decode to UTF-16 (remainder) | ~6 ms | ~27 ms |
| `JSON.parse` alone | 6.5 ms (1.1 MB) | 21.9 ms (4.4 MB) |
| Total `wasm.parse` | 32 ms | 74 ms |

### Concerns: the wasm boundary

**1. The architecture has a floor above upstream.** Every parse builds the tree twice: once as Rust `Rc` nodes and once as JS objects, with a serialized copy in between. Even if the Rust parse cost nothing, `JSON.parse` of the result alone takes about half of upstream's entire parse time (6.5 of 12 ms). With location info it takes *longer* than all of upstream (22 ms against 16 ms). LWC always requests locations (`sourceCodeLocationInfo: true` plus `onParseError`). **With a copy-the-whole-tree design, the wasm port cannot be faster than parse5 for LWC's workload, however fast the Rust side gets.** The first-order question for this project is whether the goal is parity (the port works as a drop-in) or speed (it needs a different boundary; see recommendations).

**2. Location info quadruples the payload.** Every element carries `sourceCodeLocation` with `startTag`, `endTag` and per-attribute locations, each a 6-field object with long key names. `startTag.attrs` repeats the attribute locations. Text is a poor encoding for what are mostly small integers: 1.1 MB of JSON becomes 4.4 MB.

**3. Small inputs pay a fixed tax.** LWC parses many small templates. At 1 KiB the port is 3.2× slower, dominated by per-call overhead: option decoding through `serde_wasm_bindgen`, copying the source string into wasm memory (UTF-16 → UTF-8, then UTF-8 → UTF-16 again for the preprocessor), JSON encoding and decoding, and parent linking.

**4. Streaming is quadratic: 230× slower, and a bug rather than a design limit.** Two separate problems make `parse5-parser-stream` and `parse5-plain-text-conversion-stream` unusable on real pages:
- For each recorded stack event, `node_path` in `crates/parse5-wasm/src/lib.rs` finds the node by **depth-first search from the root**, cloning every child list on the way. That is O(events × nodes), and it runs even with the default tree adapter, where JS then throws the events away (`replayStackEvents` returns without a `pathMap`). In a profile of a single 216 KiB write it accounts for about three quarters of the time. Walking up parent pointers and skipping events nobody consumes would remove it.
- After each `write()`, JS re-exports and re-adapts the *whole* document (`#native.document()`), making a stream of *k* chunks O(k × n). It also replaces the JS objects for already-parsed nodes on every write, so node identity is not stable across chunks, unlike upstream.

**5. The tokenizer boundary is chatty (13×).** Each token costs about nine wasm calls: `setState`, `setLastStartTagName`, `setInForeignNode` and `setBufferWaterline` before `nextToken`, then four getters after it. Each token and its errors are also built as a `serde_json::Value` and encoded. JS additionally keeps its own copy of the input to emulate `preprocessor.html`. `parse5-sax-parser` and `parse5-html-rewriting-stream` inherit all of this. Batching tokens per `write()`, and syncing state only when the parser-feedback simulator actually changes it, would cut most of it.

**6. Deep documents fail where upstream succeeds.** The Rust core is safe at any depth, but the boundary is not. JSON encoding in wasm recurses once per nesting level and overflows the wasm stack between 5k and 10k levels deep (`RangeError: Maximum call stack size exceeded`); `attachParentLinks` and `transformDefaultTreeNode` in JS recurse the same way. Upstream parse5 builds the tree without recursion and parses 50k-deep input fine. Hostile or generated HTML can reach these depths.

**7. Custom tree adapters pay twice and behave differently.** The default tree is built, transferred and then rebuilt through the adapter. `onItemPush`/`onItemPop` fire after the tree is complete rather than during construction. Adapters that inspect the tree from those hooks see a different state than they would upstream.

**8. Stack events are recorded unconditionally.** Even a native Rust `parse()` pushes one `StackEvent` (with an `Rc` clone) per element push and pop and keeps them until the parser is dropped.

**9. The shipped artifact.** `parse5_wasm_bg.wasm` is 513 KiB (174 KiB gzipped) before `wasm-opt`, against upstream's pure-JS package. It needs async initialization, which the `parse5` facade hides behind top-level `await`, so consumers must support ESM top-level `await`. wasm also runs the Rust core about 1.7× slower than native (dlmalloc allocator, no SIMD enabled).

### Recommendations, roughly by payoff per effort

1. **Fix streaming (concern 4).** Compute event paths by walking `parent` pointers, and only record or emit stack events when a custom adapter or hooks will consume them. This is small and local, and removes the 200× regression. Then export only nodes that changed since the last write (or export once at the end) so streaming is O(n).
2. **Make tree transfer non-recursive (concern 6).** Encode JSON with an explicit stack in Rust, and make `attachParentLinks` and `transformDefaultTreeNode` iterative.
3. **Batch the tokenizer boundary (concern 5).** Return all tokens from a `write()` in one payload and sync state only when it changes.
4. **Replace JSON with a compact binary encoding (concerns 1–3).** For example, a flat node table in a `Uint32Array` (kind, parent, first child, next sibling, name/string indices, location fields) plus a deduplicated string table, decoded into parse5-shaped objects in a single JS loop. Locations become integers instead of 6-key objects. This is the only change listed here that attacks the floor in concern 1, and it still leaves the tree built twice.
5. **Decide what wasm is for.** If the goal is faster LWC template compilation, the tree probably has to stay on one side of the boundary: keep parsing in JS, or move the consumer (the template compiler's traversal) into Rust. A drop-in wasm replacement for parse5 is useful for parity and for sharing one parser with Rust tooling, but on today's evidence it should not be expected to be faster.
