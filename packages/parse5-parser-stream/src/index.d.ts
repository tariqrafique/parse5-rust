import { Writable } from 'node:stream';

import {
  Parser,
  type DefaultTreeAdapterMap,
  type ParserOptions,
  type TreeAdapterTypeMap,
} from '../../parse5/src/index.js';

export class ParserStream<T extends TreeAdapterTypeMap = DefaultTreeAdapterMap> extends Writable {
  static getFragmentStream<T extends TreeAdapterTypeMap>(
    fragmentContext?: T['parentNode'] | null,
    options?: ParserOptions<T>
  ): ParserStream<T>;

  parser: Parser<T>;

  constructor(options?: ParserOptions<T>, parser?: Parser<T>);

  get document(): T['document'];
  getFragment(): T['documentFragment'];

  on(
    event: 'script',
    handler: (scriptElement: T['element'], documentWrite: (html: string) => void, resume: () => void) => void
  ): this;
  on(event: string, handler: (...args: unknown[]) => void): this;
}
