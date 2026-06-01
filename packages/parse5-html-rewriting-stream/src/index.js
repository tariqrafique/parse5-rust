import { html } from '../../parse5/src/index.js';
import { SAXParser } from '../../parse5-sax-parser/src/index.js';

function escapeText(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('\u00a0', '&nbsp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

function escapeAttribute(value) {
  return String(value).replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('\u00a0', '&nbsp;');
}

export class RewritingStream extends SAXParser {
  constructor() {
    super({ sourceCodeLocationInfo: true });
  }

  _transformChunk(chunk) {
    super._transformChunk(chunk);
    return '';
  }

  _getRawHtml(location) {
    const { droppedBufferSize, html: source } = this.tokenizer.preprocessor;
    const start = location.startOffset - droppedBufferSize;
    const end = location.endOffset - droppedBufferSize;

    return source.slice(start, end);
  }

  emitIfListenerExists(eventName, token) {
    if (!super.emitIfListenerExists(eventName, token)) {
      this.emitRaw(this._getRawHtml(token.sourceCodeLocation));
    }

    this.parserFeedbackSimulator.skipNextNewLine = false;
    return true;
  }

  _emitToken(eventName, token) {
    this.emit(eventName, token, this._getRawHtml(token.sourceCodeLocation));
  }

  emitDoctype(token) {
    let html = `<!DOCTYPE ${token.name}`;

    if (token.publicId !== null) {
      html += ` PUBLIC "${token.publicId}"`;
    } else if (token.systemId !== null) {
      html += ' SYSTEM';
    }

    if (token.systemId !== null) {
      html += ` "${token.systemId}"`;
    }

    html += '>';
    this.push(html);
  }

  emitStartTag(token) {
    let html = `<${token.tagName}`;

    for (const attr of token.attrs) {
      html += ` ${attr.name}="${escapeAttribute(attr.value)}"`;
    }

    html += token.selfClosing ? '/>' : '>';
    this.push(html);
  }

  emitEndTag(token) {
    this.push(`</${token.tagName}>`);
  }

  emitText({ text }) {
    this.push(
      !this.parserFeedbackSimulator.inForeignContent && html.hasUnescapedText(this.tokenizer.lastStartTagName, true)
        ? text
        : escapeText(text)
    );
  }

  emitComment(token) {
    this.push(`<!--${token.text}-->`);
  }

  emitRaw(html) {
    this.push(html);
  }
}
