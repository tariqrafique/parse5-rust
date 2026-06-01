import { initParse5Wasm } from '../../parse5-wasm/src/index.js';

export {
  ErrorCodes,
  Parser,
  Token,
  Tokenizer,
  TokenizerMode,
  defaultTreeAdapter,
  foreignContent,
  html,
  parse,
  parseFragment,
  serialize,
  serializeOuter,
} from '../../parse5-wasm/src/index.js';

await initParse5Wasm();
