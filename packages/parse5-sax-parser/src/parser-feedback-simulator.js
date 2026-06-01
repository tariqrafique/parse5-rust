import { Token, Tokenizer, TokenizerMode, foreignContent, html } from '../../parse5/src/index.js';

const $ = html.TAG_ID;
const REPLACEMENT_CHARACTER = '\uFFFD';
const LINE_FEED_CODE_POINT = 0x0a;

export class ParserFeedbackSimulator {
  namespaceStack = [];
  inForeignContent = false;
  skipNextNewLine = false;

  constructor(options, handler) {
    this.handler = handler;
    this.tokenizer = new Tokenizer(options, this);
    this.#enterNamespace(html.NS.HTML);
  }

  onNullCharacter(token) {
    this.skipNextNewLine = false;

    if (this.inForeignContent) {
      this.handler.onCharacter({
        type: Token.TokenType.CHARACTER,
        chars: REPLACEMENT_CHARACTER,
        location: token.location,
      });
    } else {
      this.handler.onNullCharacter(token);
    }
  }

  onWhitespaceCharacter(token) {
    if (this.skipNextNewLine && token.chars.charCodeAt(0) === LINE_FEED_CODE_POINT) {
      this.skipNextNewLine = false;

      if (token.chars.length === 1) {
        return;
      }

      token.chars = token.chars.substring(1);
    }

    this.handler.onWhitespaceCharacter(token);
  }

  onCharacter(token) {
    this.skipNextNewLine = false;
    this.handler.onCharacter(token);
  }

  onComment(token) {
    this.skipNextNewLine = false;
    this.handler.onComment(token);
  }

  onDoctype(token) {
    this.skipNextNewLine = false;
    this.handler.onDoctype(token);
  }

  onEof(token) {
    this.skipNextNewLine = false;
    this.handler.onEof(token);
  }

  #enterNamespace(namespace) {
    this.namespaceStack.unshift(namespace);
    this.inForeignContent = namespace !== html.NS.HTML;
    this.tokenizer.inForeignNode = this.inForeignContent;
  }

  #leaveCurrentNamespace() {
    this.namespaceStack.shift();
    this.inForeignContent = this.namespaceStack[0] !== html.NS.HTML;
    this.tokenizer.inForeignNode = this.inForeignContent;
  }

  #ensureTokenizerMode(tagID) {
    switch (tagID) {
      case $.TEXTAREA:
      case $.TITLE:
        this.tokenizer.state = TokenizerMode.RCDATA;
        break;
      case $.PLAINTEXT:
        this.tokenizer.state = TokenizerMode.PLAINTEXT;
        break;
      case $.SCRIPT:
        this.tokenizer.state = TokenizerMode.SCRIPT_DATA;
        break;
      case $.STYLE:
      case $.IFRAME:
      case $.XMP:
      case $.NOEMBED:
      case $.NOFRAMES:
      case $.NOSCRIPT:
        this.tokenizer.state = TokenizerMode.RAWTEXT;
        break;
    }
  }

  onStartTag(token) {
    let tagID = token.tagID;

    switch (tagID) {
      case $.SVG:
        this.#enterNamespace(html.NS.SVG);
        break;
      case $.MATH:
        this.#enterNamespace(html.NS.MATHML);
        break;
    }

    if (this.inForeignContent) {
      if (foreignContent.causesExit(token)) {
        this.#leaveCurrentNamespace();
      } else {
        const currentNamespace = this.namespaceStack[0];

        if (currentNamespace === html.NS.MATHML) {
          foreignContent.adjustTokenMathMLAttrs(token);
        } else if (currentNamespace === html.NS.SVG) {
          foreignContent.adjustTokenSVGTagName(token);
          foreignContent.adjustTokenSVGAttrs(token);
        }

        foreignContent.adjustTokenXMLAttrs(token);
        tagID = token.tagID;

        if (!token.selfClosing && foreignContent.isIntegrationPoint(tagID, currentNamespace, token.attrs)) {
          this.#enterNamespace(html.NS.HTML);
        }
      }
    } else {
      switch (tagID) {
        case $.PRE:
        case $.TEXTAREA:
        case $.LISTING:
          this.skipNextNewLine = true;
          break;
        case $.IMAGE:
          token.tagName = html.TAG_NAMES.IMG;
          token.tagID = $.IMG;
          break;
      }

      this.#ensureTokenizerMode(tagID);
    }

    this.handler.onStartTag(token);
  }

  onEndTag(token) {
    let tagID = token.tagID;

    if (!this.inForeignContent) {
      const previousNamespace = this.namespaceStack[1];

      if (previousNamespace === html.NS.SVG) {
        const adjustedTagName = foreignContent.SVG_TAG_NAMES_ADJUSTMENT_MAP.get(token.tagName);

        if (adjustedTagName) {
          tagID = html.getTagID(adjustedTagName);
        }
      }

      if (foreignContent.isIntegrationPoint(tagID, previousNamespace, token.attrs)) {
        this.#leaveCurrentNamespace();
      }
    } else if (
      (tagID === $.SVG && this.namespaceStack[0] === html.NS.SVG) ||
      (tagID === $.MATH && this.namespaceStack[0] === html.NS.MATHML)
    ) {
      this.#leaveCurrentNamespace();
    }

    if (this.namespaceStack[0] === html.NS.SVG) {
      foreignContent.adjustTokenSVGTagName(token);
    }

    this.handler.onEndTag(token);
  }
}
