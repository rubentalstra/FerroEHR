//
// description: Antlr4 grammar for cADL non-primitves sub-syntax of Archetype Definition Language (ADL2)
// author:      Thomas Beale <thomas.beale@openehr.org>
// contributors:Pieter Bos <pieter.bos@nedap.com>
// support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
// copyright:   Copyright (c) 2015 openEHR Foundation <http://www.openEHR.org>
// license:     Apache 2.0 License <http://www.apache.org/licenses/LICENSE-2.0.html>
//

parser grammar Cadl2Parser;
options { tokenVocab=Cadl2Lexer; }
import Cadl2PrimitiveConstraintsParser, AdlPathParser;

//
//  ======================= Top-level Objects ========================
//

cComplexObject: rmTypeId nodeId cOccurrences? ( SYM_MATCHES '{' cComplexObjectDef '}' )? ;

cComplexObjectDef: ( defaultValue | cAttributes defaultValue? ) ;

nodeId: '[' nodeIdCode ']' ;

nodeIdCode: ROOT_ID_CODE | ID_CODE ;

// ------------------- Complex constraint types ----------------------

cAttributes: ( cAttribute | cAttributeTuple )+ ;

cAttribute: ( adlPath | rmAttributeId ) cExistence? cCardinality? ( SYM_MATCHES '{' ( cAttributeDef | cInlinePrimitiveObject ) '}' )? ;

cAttributeDef: cRegularObjectOrdered+ ;

cRegularObjectOrdered: siblingOrder? cRegularObject ;

siblingOrder: ( SYM_AFTER | SYM_BEFORE ) nodeId ;

cRegularObject:
      cComplexObject
    | cArchetypeRoot
    | cComplexObjectProxy
    | archetypeSlot
    | cRegularPrimitiveObject
    ;

cArchetypeRoot: SYM_USE_ARCHETYPE rmTypeId '[' ID_CODE ',' ( FULLY_QUALIFIED_RM_ENTITY | ARCHETYPE_REF ) ']' cOccurrences? ;

cComplexObjectProxy: SYM_USE_NODE rmTypeId nodeId cOccurrences? adlPath ;

cRegularPrimitiveObject: rmTypeId nodeId cOccurrences? ( SYM_MATCHES '{' cInlinePrimitiveObject '}' )? ;

// Slot includes are modelled to support only the simple form of
// path matches {regex}, but this is probably safe. If not, the
// 'assertion' rule from EL is required, which causes the whole EL
// to be sucked in to CADL.
archetypeSlot: SYM_ALLOW_ARCHETYPE rmTypeId nodeId (( cOccurrences? ( SYM_MATCHES '{' cIncludes? cExcludes? '}' )? ) | SYM_CLOSED ) ;
cIncludes : SYM_INCLUDE archetypeIdConstraint+ ;
cExcludes : SYM_EXCLUDE archetypeIdConstraint+ ;
archetypeIdConstraint: archetypeIdPath SYM_MATCHES '{' DELIMITED_REGEX '}' ;

// have to allow for relative paths. Note the path here is not an adlPath
// (which is a path in the Cadl definition part); it is a path in the
// ARCHETYPE runtime instance
// TODO: future ADL should probably change this
archetypeIdPath : '/'? LC_ID adlPath* ;

// Tuple constraints
cAttributeTuple : '[' cAttributeTupleAttrs ']' SYM_MATCHES '{' cPrimitiveTuples '}' ;
cAttributeTupleAttrs : rmAttributeId ( ',' rmAttributeId )* ;
cPrimitiveTuples: cPrimitiveTuple ( ',' cPrimitiveTuple )* ;
cPrimitiveTuple : '[' '{' cInlinePrimitiveObject '}' ( ',' '{' cInlinePrimitiveObject '}' )* ']' ;

// ------------------- existence and cardinality -----------------------
cExistence   : SYM_EXISTENCE SYM_MATCHES '{' existence '}' ;
existence    : INTEGER | INTEGER '..' INTEGER ;

cCardinality : SYM_CARDINALITY SYM_MATCHES '{' cardinality '}' ;
cardinality  : multiplicity ( multiplicityMod multiplicityMod? )? ; // max of two

cOccurrences : SYM_OCCURRENCES SYM_MATCHES '{' multiplicity '}' ;

multiplicity : INTEGER | '*' | INTEGER '..' ( INTEGER | '*' ) ;
multiplicityMod : orderingMod | uniqueMod ;
orderingMod  : ';' ( SYM_ORDERED | SYM_UNORDERED ) ;
uniqueMod    : ';' SYM_UNIQUE ;

// ------------------------- default values ---------------------------
// These are somewhat tricky, because inline ODIN and other serial forms
// have to be supported. They are handled using modal lexing, with the
// internal text lines being captured for passing to dedicated parsers
// for the appropriate serial form (i.e. ODIN, JSON, YAML etc)
defaultValue: DEFAULT_BLOCK_START serialBlock ;

serialBlock:
      odinBlock
    | otherSerialBlock 
    ; 

odinBlock: ODIN_BLOCK_LINE+ ;

otherSerialBlock: SERIAL_BLOCK_START SERIAL_BLOCK_LINE+? SERIAL_BLOCK_END ;

// ------------------------- model references -------------------------
rmTypeId      : UC_ID ( '<' rmTypeId ( ',' rmTypeId )* '>' )? ;
rmAttributeId : LC_ID ;

//
//  ======================= Query matcher constraint type ========================
//

//
// this will match a flattened ADL-based structure, normally occurring
// within {} provided by the source context, e.g. an AQL query
//
cObjectMatcher: cComplexObjectMatcher | cInlinePrimitiveObject ;

cComplexObjectMatcher: rmTypeId nodeId? ( SYM_MATCHES '{' cComplexObjectMatcherDef '}' )? ;

cComplexObjectMatcherDef: ( defaultValue | cAttributesMatcher defaultValue? ) ;

cAttributesMatcher: ( cAttributeMatcher | cAttributeTuple )+ ;

cAttributeMatcher: ( adlPath | rmAttributeId ) ( SYM_MATCHES '{' ( cAttributeMatcherDef | cInlinePrimitiveObject ) '}' )? ;

cAttributeMatcherDef: cRegularObjectMatcher+ ;

cRegularObjectMatcher:
      cComplexObjectMatcher
    | cRegularPrimitiveObject
    ;