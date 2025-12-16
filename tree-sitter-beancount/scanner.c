#include "tree_sitter/parser.h"

enum TokenType {
  EOL,
  INDENT,
};

static inline void skip(TSLexer *lexer) {
  lexer->advance(lexer, true);
}

static inline bool is_whitespace(int32_t lookahead) {
  return lookahead == ' ' || lookahead == '\t' || lookahead == '\r';
}

void * tree_sitter_beancount_external_scanner_create() {
  return NULL;
}

void tree_sitter_beancount_external_scanner_destroy(
  __attribute__((unused)) void *payload
) {
  // noop
}

unsigned tree_sitter_beancount_external_scanner_serialize(
  __attribute__((unused)) void *payload,
  __attribute__((unused)) char *buffer
) {
  return 0;
}


void tree_sitter_beancount_external_scanner_deserialize(
  __attribute__((unused)) void *payload,
  __attribute__((unused)) char *buffer,
  __attribute__((unused)) unsigned length
) {
  // noop
}

bool tree_sitter_beancount_external_scanner_scan(
  __attribute__((unused)) void *payload,
  TSLexer *lexer,
  const bool *valid_symbols
) {
  if (lexer->eof(lexer)) {
    lexer->result_symbol = EOL;
    return true;
  }
  // Skip initial whitespace
  while (is_whitespace(lexer->lookahead)) {
    skip(lexer);
  }
  if (lexer->lookahead == '\n') {
    skip(lexer);
    if (!is_whitespace(lexer->lookahead) || !valid_symbols[INDENT]) {
      lexer->result_symbol = EOL;
      return true;
    }
    do {
      skip(lexer);
    } while (is_whitespace(lexer->lookahead));
    if (lexer->lookahead == '\n') {
      // The line contained just whitespace - skip and return EOL
      skip(lexer);
      lexer->result_symbol = EOL;
      return true;
    } else {
      lexer->result_symbol = INDENT;
      return true;
    }
  }

  return false;
}
