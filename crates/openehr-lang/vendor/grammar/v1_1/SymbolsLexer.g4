
//
// Grammar defining compact terminal value types, including atoms, lists and intervals that
// may be used in ODIN but also other formalisms such as JSON (schema driven) and YAML.
// author:      Pieter Bos <pieter.bos@nedap.com>, Thomas Beale <thomas.beale@openEHR.org>
// support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
// copyright:   Copyright (c) 2018- openEHR Foundation <http://www.openEHR.org>
//

lexer grammar SymbolsLexer;

// -------------------- special symbols ------------------------
SYM_NAMESPACE_SEP: '::' ;
SYM_DOUBLE_DOT   : '..' ;

// ------------------ logic & math symbols ---------------------
SYM_LE : '<=' | '≤' ;
SYM_GE : '>=' | '≥' ;
SYM_GT : '>' ;
SYM_LT : '<' ;
SYM_NE : '/=' | '!=' | '≠' ;
SYM_EQ : '=' ;

SYM_PLUS_OR_MINUS : '+/-' | '±' ;
SYM_PLUS    : '+' ;
SYM_MINUS   : '-' ;
SYM_PERCENT : '%' ;
SYM_CARET   : '^' ;

// -------------------- general symbols ------------------------
SYM_VERTICAL_BAR : '|' ;
SYM_DOT   : '.' ;

SYM_COMMA : ',' ;
SYM_SLASH : '/' ;
SYM_COLON : ':' ;
SYM_SEMI_COLON : ';' ;

SYM_ASTERISK  : '*' ;

SYM_LPAREN   : '(';
SYM_RPAREN   : ')';
SYM_LBRACKET : '[';
SYM_RBRACKET : ']';
SYM_LCURLY   : '{' ;
SYM_RCURLY   : '}' ;
