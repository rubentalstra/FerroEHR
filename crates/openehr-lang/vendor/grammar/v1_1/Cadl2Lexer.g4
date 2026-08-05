//
// description: Antlr4 grammar for cADL non-primitves sub-syntax of Archetype Definition Language (ADL2).
//              This has to include
//              other relevant Lexer grammars in the correct order, in order to generate a
//              correct total tokens file for use by the parser grammar.
// author:      Thomas Beale <thomas.beale@openehr.org>
// contributors:Pieter Bos <pieter.bos@nedap.com>
// support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
// copyright:   Copyright (c) 2015 openEHR Foundation <http://www.openEHR.org>
// license:     Apache 2.0 License <http://www.apache.org/licenses/LICENSE-2.0.html>
//

lexer grammar Cadl2Lexer;
import Cadl2PrimitiveConstraintsLexer, GeneralIdsLexer;

channels {
    COMMENT
}

// ------------------ lines and comments ------------------
CMT_LINE : '--' .*? EOL -> channel(COMMENT) ;
EOL      : '\r'? '\n'   -> skip ;
WS       : [ \t\r]+     -> skip ;

// ----------------------- keywords -----------------------
SYM_EXISTENCE   : 'existence' ;
SYM_OCCURRENCES : 'occurrences' ;
SYM_CARDINALITY : 'cardinality' ;
SYM_ORDERED     : 'ordered' ;
SYM_UNORDERED   : 'unordered' ;
SYM_UNIQUE      : 'unique' ;
SYM_USE_NODE    : 'use_node' ;
SYM_USE_ARCHETYPE : 'use_archetype' ;
SYM_ALLOW_ARCHETYPE : 'allow_archetype' ;
SYM_INCLUDE     : 'include' ;
SYM_EXCLUDE     : 'exclude' ;
SYM_AFTER       : 'after' ;
SYM_BEFORE      : 'before' ;
SYM_CLOSED      : 'closed' ;

// --------------------- general symbols ------------------------
SYM_MATCHES  : 'matches' | 'is_in' | '∈' ;

// ----------------------- default blocks --------------------------
// ODIN flavour
//        _default = (TYPE) <
//            attribute_1 = <"aaaa">
//            attribute_3 = <
//               attribute_3_1 = <"aaaa">
//            >
//            attribute_N = <25.77>
//        >
//
// Any other flavour, e.g. json
//        _default = (json) <#
//            {
//                "_type": "TYPE",
//                "attribute_1": value_1,
//                "attribute_2": value_2,
//                ...
//                "attribute_N": value_N
//            }
//        #>

DEFAULT_BLOCK_START : '_default' WS? '=' -> mode (OBJECT_BLOCK);

// -------------------------- Modal lexers -----------------------------

// modes to grab included object blocks; these do explicit whitespace handling
// since they have to capture everything so it can be passed to other parsers
mode OBJECT_BLOCK ;
SERIAL_BLOCK_START: WS? '(' TYPE_INFO ')' WS? '<#' WS? EOL -> mode (SERIAL_BLOCK);
ODIN_BLOCK_START: WS? '(' TYPE_INFO ')' WS? '<' WS? EOL -> mode (ODIN_BLOCK), type (ODIN_BLOCK_LINE);
fragment TYPE_INFO: ALPHA_CHAR ALPHANUM_US_CHAR* ;
WS2 : WS -> skip ;

//
// The end of a Serial block is easy to find - because '#>' only occurs once
//
mode SERIAL_BLOCK ;
SERIAL_BLOCK_END: WS '#>' WS? EOL -> mode(DEFAULT_MODE) ;
SERIAL_BLOCK_LINE : NON_EOL* EOL ;
fragment NON_EOL : ~'\n' ;

//
// The end of an ODIN block is a bit tricky - because '>' occurs numerous times
// -> have to spot following '}', and return the usual token (SYM_RCURLY)
// Just in case, we rewrite the matched text to just the symbol.
//
mode ODIN_BLOCK ;
ODIN_BLOCK_POST: WS '}' WS? EOL { setText ("}"); } -> mode(DEFAULT_MODE), type(SYM_RCURLY) ;
ODIN_BLOCK_LINE : NON_EOL* EOL ;
