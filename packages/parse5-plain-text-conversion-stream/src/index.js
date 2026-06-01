import { ParserStream } from '../../parse5-parser-stream/src/index.js';

export class PlainTextConversionStream extends ParserStream {
  constructor(options) {
    super(options);
    this.parser.initPlainTextConversion();
  }
}

