import { Transform } from 'node:stream';

import {
  type Attribute,
  type Location,
  type TokenizerOptions,
} from '../../parse5/src/index.js';
import { ParserFeedbackSimulator } from './parser-feedback-simulator.js';

export interface SAXParserOptions {
  sourceCodeLocationInfo?: boolean;
}

export interface SaxToken {
  sourceCodeLocation?: Location | null;
}

export interface StartTag extends SaxToken {
  tagName: string;
  attrs: Attribute[];
  selfClosing: boolean;
}

export interface EndTag extends SaxToken {
  tagName: string;
}

export interface Text extends SaxToken {
  text: string;
}

export interface Comment extends SaxToken {
  text: string;
}

export interface Doctype extends SaxToken {
  name: string | null;
  publicId: string | null;
  systemId: string | null;
}

export class SAXParser extends Transform {
  protected options: SAXParserOptions;
  protected parserFeedbackSimulator: ParserFeedbackSimulator;
  protected tokenizer: ParserFeedbackSimulator['tokenizer'];

  constructor(options?: TokenizerOptions);
  stop(): void;
  protected _transformChunk(chunk: string): string;
  protected emitIfListenerExists(eventName: string, token: SaxToken): boolean;
  protected _emitToken(eventName: string, token: SaxToken): void;
  on(event: 'startTag', listener: (startTag: StartTag) => void): this;
  on(event: 'endTag', listener: (endTag: EndTag) => void): this;
  on(event: 'comment', listener: (comment: Comment) => void): this;
  on(event: 'text', listener: (text: Text) => void): this;
  on(event: 'doctype', listener: (doctype: Doctype) => void): this;
  on(event: string, handler: (...args: unknown[]) => void): this;
}
