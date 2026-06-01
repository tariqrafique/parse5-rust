import { Writable } from 'node:stream';

import { Parser } from '../../parse5/src/index.js';

export class ParserStream extends Writable {
  static getFragmentStream(fragmentContext = null, options) {
    const parser = Parser.getFragmentParser(fragmentContext, options);
    return new ParserStream(options, parser);
  }

  #lastChunkWritten = false;
  #writeCallback = undefined;
  #pendingHtmlInsertions = [];

  get document() {
    return this.parser.document;
  }

  constructor(options, parser = new Parser(options)) {
    super({ decodeStrings: false });

    this.parser = parser;

    const resume = () => {
      for (let i = this.#pendingHtmlInsertions.length - 1; i >= 0; i--) {
        this.parser.tokenizer.insertHtmlAtCurrentPos(this.#pendingHtmlInsertions[i]);
      }

      this.#pendingHtmlInsertions.length = 0;
      this.parser.tokenizer.resume(this.#writeCallback);
    };

    const documentWrite = (html) => {
      if (!this.parser.stopped) {
        this.#pendingHtmlInsertions.push(html);
      }
    };

    this.parser.scriptHandler = (scriptElement) => {
      if (this.listenerCount('script') > 0) {
        this.parser.tokenizer.pause();
        this.emit('script', scriptElement, documentWrite, resume);
      }
    };
  }

  getFragment() {
    return this.parser.getFragment();
  }

  _write(chunk, _encoding, callback) {
    if (typeof chunk !== 'string') {
      throw new TypeError('Parser can work only with string streams.');
    }

    this.#writeCallback = callback;
    this.parser.tokenizer.write(chunk, this.#lastChunkWritten, this.#writeCallback);
  }

  end(chunk, encoding, callback) {
    this.#lastChunkWritten = true;
    return super.end(chunk || '', encoding, callback);
  }
}
