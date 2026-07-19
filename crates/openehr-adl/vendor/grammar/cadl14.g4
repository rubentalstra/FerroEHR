//
// description: Antlr4 grammar for cADL non-primitves sub-syntax of Archetype Definition Language (ADL2)
//  author:      Thomas Beale <thomas.beale@openehr.org>
//  contributors:Pieter Bos <pieter.bos@nedap.com>
//  support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
//  copyright:   Copyright (c) 2015- openEHR Foundation <http://www.openEHR.org>
// license:     Apache 2.0 License <http://www.apache.org/licenses/LICENSE-2.0.html>
//

grammar cadl14;
import cadl14_primitives, odin, base_expressions;

//
//  ======================= Top-level Objects ========================
//

c_complex_object: rm_type_id (at_type_id)? c_occurrences? ( SYM_MATCHES '{' (c_attribute+ | '*') '}' )? ;

at_type_id: '[' ( AT_CODE ) ']';

// ======================== Components =======================

c_objects: c_non_primitive_object_ordered+ | c_primitive_object ;

c_non_primitive_object_ordered: c_non_primitive_object ;

c_non_primitive_object:
      c_complex_object
    | domain_specific_extension
    | c_archetype_root
    | c_complex_object_proxy
    | archetype_slot
    | c_ordinal
    ;

domain_specific_extension: rm_type_id '<' odin_text? '>';

c_ordinal: ordinal_term  (',' ordinal_term)* (';' assumed_ordinal_value)?;

assumed_ordinal_value: INTEGER | REAL;

ordinal_term: (integer_value | real_value) '|' c_terminology_code;

c_archetype_root: SYM_USE_ARCHETYPE rm_type_id '[' AT_CODE (',' ARCHETYPE_REF)? ']' c_occurrences? ( SYM_MATCHES '{' c_attribute+ '}' )? ;

c_complex_object_proxy: SYM_USE_NODE rm_type_id ('[' AT_CODE ']')? c_occurrences? ADL_PATH ;

archetype_slot:
      c_archetype_slot_head SYM_MATCHES '{' c_includes? c_excludes? '}'
    | c_archetype_slot_head
    ;

c_archetype_slot_head: c_archetype_slot_id c_occurrences? ;

c_archetype_slot_id: SYM_ALLOW_ARCHETYPE rm_type_id ('[' AT_CODE ']')? ;

c_attribute: rm_attribute_id c_existence? c_cardinality? ( SYM_MATCHES '{' (c_objects | '*') '}' )? ;

c_includes : SYM_INCLUDE assertion+ ;
c_excludes : SYM_EXCLUDE assertion+ ;

c_existence: SYM_EXISTENCE SYM_MATCHES '{' existence '}' ;
existence: INTEGER | INTEGER '..' INTEGER ;

c_cardinality    : SYM_CARDINALITY SYM_MATCHES '{' cardinality '}' ;
cardinality      : multiplicity ( multiplicity_mod multiplicity_mod? )? ; // max of two
ordering_mod     : ';' ( SYM_ORDERED | SYM_UNORDERED ) ;
unique_mod       : ';' SYM_UNIQUE ;
multiplicity_mod : ordering_mod | unique_mod ;

c_occurrences : SYM_OCCURRENCES SYM_MATCHES '{' multiplicity '}' ;
multiplicity  : INTEGER | '*' | INTEGER '..' ( INTEGER | '*' ) ;

//
// ---------- Lexer patterns -----------------
//

// CADL keywords
SYM_EXISTENCE   : [Ee][Xx][Ii][Ss][Tt][Ee][Nn][Cc][Ee] ;
SYM_OCCURRENCES : [Oo][Cc][Cc][Uu][Rr][Rr][Ee][Nn][Cc][Ee][Ss] ;
SYM_CARDINALITY : [Cc][Aa][Rr][Dd][Ii][Nn][Aa][Ll][Ii][Tt][Yy] ;
SYM_ORDERED     : [Oo][Rr][Dd][Ee][Rr][Ee][Dd] ;
SYM_UNORDERED   : [Uu][Nn][Oo][Rr][Dd][Ee][Rr][Ee][Dd] ;
SYM_UNIQUE      : [Uu][Nn][Ii][Qq][Uu][Ee] ;
SYM_USE_NODE    : [Uu][Ss][Ee][_][Nn][Oo][Dd][Ee] ;
SYM_USE_ARCHETYPE : [Uu][Ss][Ee][_][Aa][Rr][Cc][Hh][Ee][Tt][Yy][Pp][Ee] ;
SYM_ALLOW_ARCHETYPE : [Aa][Ll][Ll][Oo][Ww][_][Aa][Rr][Cc][Hh][Ee][Tt][Yy][Pp][Ee] ;
SYM_INCLUDE     : [Ii][Nn][Cc][Ll][Uu][Dd][Ee] ;
SYM_EXCLUDE     : [Ee][Xx][Cc][Ll][Uu][Dd][Ee] ;

