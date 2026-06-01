import {
  type CharacterToken,
  type CommentToken,
  type DoctypeToken,
  type EOFToken,
  type TagToken,
  type TokenHandler,
  type TokenizerOptions,
  Tokenizer,
} from '../../parse5/src/index.js';

export class ParserFeedbackSimulator implements TokenHandler {
  inForeignContent: boolean;
  skipNextNewLine: boolean;
  tokenizer: Tokenizer;

  constructor(options: TokenizerOptions, handler: TokenHandler);
  onNullCharacter(token: CharacterToken): void;
  onWhitespaceCharacter(token: CharacterToken): void;
  onCharacter(token: CharacterToken): void;
  onComment(token: CommentToken): void;
  onDoctype(token: DoctypeToken): void;
  onEof(token: EOFToken): void;
  onStartTag(token: TagToken): void;
  onEndTag(token: TagToken): void;
}
