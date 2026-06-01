import { SAXParser, type Comment, type Doctype, type EndTag, type SaxToken, type StartTag, type Text } from '../../parse5-sax-parser/src/index.js';

export class RewritingStream extends SAXParser {
  constructor();
  protected _transformChunk(chunk: string): string;
  private _getRawHtml;
  protected emitIfListenerExists(eventName: string, token: SaxToken): boolean;
  protected _emitToken(eventName: string, token: SaxToken): void;
  emitDoctype(token: Doctype): void;
  emitStartTag(token: StartTag): void;
  emitEndTag(token: EndTag): void;
  emitText(token: Text): void;
  emitComment(token: Comment): void;
  emitRaw(html: string): void;
}

export interface RewritingStream {
  on(event: 'startTag', listener: (startTag: StartTag, rawHtml: string) => void): this;
  on(event: 'endTag', listener: (endTag: EndTag, rawHtml: string) => void): this;
  on(event: 'comment', listener: (comment: Comment, rawHtml: string) => void): this;
  on(event: 'text', listener: (text: Text, rawHtml: string) => void): this;
  on(event: 'doctype', listener: (doctype: Doctype, rawHtml: string) => void): this;
  on(event: string, handler: (...args: unknown[]) => void): this;
}
