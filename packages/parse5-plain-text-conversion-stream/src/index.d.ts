import {
  type DefaultTreeAdapterMap,
  type ParserOptions,
  type TreeAdapterTypeMap,
} from '../../parse5/src/index.js';
import { ParserStream } from '../../parse5-parser-stream/src/index.js';

export class PlainTextConversionStream<
  T extends TreeAdapterTypeMap = DefaultTreeAdapterMap,
> extends ParserStream<T> {
  constructor(options?: ParserOptions<T>);
}
