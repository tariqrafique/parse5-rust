import { Transform } from 'node:stream';

import { DevNullStream } from './dev-null-stream.js';
import { ParserFeedbackSimulator } from './parser-feedback-simulator.js';

export class SAXParser extends Transform {
  #pendingText = null;
  #lastChunkWritten = false;
  #stopped = false;

  constructor(options = {}) {
    super({ encoding: 'utf8', decodeStrings: false });

    this.options = {
      sourceCodeLocationInfo: false,
      ...options,
    };
    this.parserFeedbackSimulator = new ParserFeedbackSimulator(this.options, this);
    this.tokenizer = this.parserFeedbackSimulator.tokenizer;
    this.pipe(new DevNullStream());
  }

  _transform(chunk, _encoding, callback) {
    if (typeof chunk !== 'string') {
      throw new TypeError('Parser can work only with string streams.');
    }

    callback(null, this._transformChunk(chunk));
  }

  _final(callback) {
    this.#lastChunkWritten = true;
    callback(null, this._transformChunk(''));
  }

  stop() {
    this.#stopped = true;
    this.tokenizer.pause();
  }

  _transformChunk(chunk) {
    if (!this.#stopped) {
      this.tokenizer.write(chunk, this.#lastChunkWritten);
    }

    return chunk;
  }

  onCharacter({ chars, location }) {
    if (this.#pendingText === null) {
      this.#pendingText = { text: chars, sourceCodeLocation: location };
    } else {
      this.#pendingText.text += chars;

      if (location && this.#pendingText.sourceCodeLocation) {
        const { endLine, endCol, endOffset } = location;
        this.#pendingText.sourceCodeLocation = {
          ...this.#pendingText.sourceCodeLocation,
          endLine,
          endCol,
          endOffset,
        };
      }
    }

    if (this.tokenizer.preprocessor.willDropParsedChunk()) {
      this.#emitPendingText();
    }
  }

  onWhitespaceCharacter(token) {
    this.onCharacter(token);
  }

  onNullCharacter(token) {
    this.onCharacter(token);
  }

  onEof() {
    this.#emitPendingText();
    this.#stopped = true;
  }

  onStartTag(token) {
    this.#emitPendingText();

    this.emitIfListenerExists('startTag', {
      tagName: token.tagName,
      attrs: token.attrs,
      selfClosing: token.selfClosing,
      sourceCodeLocation: token.location,
    });
  }

  onEndTag(token) {
    this.#emitPendingText();

    this.emitIfListenerExists('endTag', {
      tagName: token.tagName,
      sourceCodeLocation: token.location,
    });
  }

  onDoctype(token) {
    this.#emitPendingText();

    this.emitIfListenerExists('doctype', {
      name: token.name,
      publicId: token.publicId,
      systemId: token.systemId,
      sourceCodeLocation: token.location,
    });
  }

  onComment(token) {
    this.#emitPendingText();

    this.emitIfListenerExists('comment', {
      text: token.data,
      sourceCodeLocation: token.location,
    });
  }

  emitIfListenerExists(eventName, token) {
    if (this.listenerCount(eventName) === 0) {
      return false;
    }

    this._emitToken(eventName, token);
    return true;
  }

  _emitToken(eventName, token) {
    this.emit(eventName, token);
  }

  #emitPendingText() {
    if (this.#pendingText !== null) {
      this.emitIfListenerExists('text', this.#pendingText);
      this.#pendingText = null;
    }
  }
}
