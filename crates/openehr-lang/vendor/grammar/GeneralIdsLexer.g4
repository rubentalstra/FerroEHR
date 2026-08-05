//
//  General purpose patterns used in all openEHR parser and lexer tools
//  author:      Thomas Beale <thomas.beale@openEHR.org>
//  support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
//  copyright:   Copyright (c) 2021- openEHR Foundation <http://www.openEHR.org>
//

lexer grammar GeneralIdsLexer;
import PrimitiveTypesLexer;

// ------------------- human-readable identifiers ----------------------

UC_ID :  ALPHA_UCHAR ALPHANUM_US_CHAR* ;
LC_ID :  '_'? ALPHA_LCHAR ALPHANUM_US_CHAR* ;

// unlike a programming language identifier, a 'web id' can
// have hyphens and mixed case. To match a Web id in a parser,
// Use LC_ID | WEB_ID or similar
WEB_ID: ALPHANUM_CHAR ALPHANUM_US_HYP_CHAR* ;