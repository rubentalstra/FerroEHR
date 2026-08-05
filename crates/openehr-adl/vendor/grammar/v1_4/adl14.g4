//
//  description: Antlr4 grammar for Archetype Definition Language (ADL2)
//  author:      Thomas Beale <thomas.beale@openehr.org>
//  contributors:Pieter Bos <pieter.bos@nedap.com>
//  support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
//  copyright:   Copyright (c) 2015- openEHR Foundation <http://www.openEHR.org>
//  license:     Apache 2.0 License <http://www.apache.org/licenses/LICENSE-2.0.html>
//

grammar adl14;
import cadl14, odin;

//
//  ============== Parser rules ==============
//

adl14_archetype: ( archetype) EOF ;

archetype: 
    SYM_ARCHETYPE meta_data?
    ARCHETYPE_HRID
    specialization_section?
    concept_section
    language_section
    description_section
    definition_section
    invariant_section?
    ontology_section
    ;


specialization_section : SYM_SPECIALIZE ARCHETYPE_REF ;
language_section       : SYM_LANGUAGE odin_text ;
description_section    : SYM_DESCRIPTION odin_text ;
definition_section     : SYM_DEFINITION c_complex_object ;
invariant_section      : SYM_INVARIANT ( statement (';')?)+ ;
ontology_section       : SYM_ONTOLOGY odin_text ;

concept_section:
 'concept' '[' AT_CODE ']';

meta_data: '(' meta_data_item  (';' meta_data_item )* ')' ;

meta_data_item:
      meta_data_tag_adl_version '=' VERSION_ID
    | meta_data_tag_uid '=' GUID
    | meta_data_tag_build_uid '=' GUID
    | meta_data_tag_rm_release '=' VERSION_ID
    | meta_data_tag_is_controlled
    | meta_data_tag_is_generated
    | ALPHANUM_ID ( '=' meta_data_value )?
    ;

meta_data_value:
      primitive_value
    | GUID
    | VERSION_ID
    ;

meta_data_tag_adl_version   : 'adl_version' ;
meta_data_tag_uid           : 'uid' ;
meta_data_tag_build_uid     : 'build_uid' ;
meta_data_tag_rm_release    : 'rm_release' ;
meta_data_tag_is_controlled : 'controlled' ;
meta_data_tag_is_generated  : 'generated' ;


//
// ------------------ lexical patterns -----------------
//

SYM_ARCHETYPE   : [Aa][Rr][Cc][Hh][Ee][Tt][Yy][Pp][Ee] ;

SYM_SPECIALIZE  : '\n'[Ss][Pp][Ee][Cc][Ii][Aa][Ll][Ii][SsZz][Ee] ;
SYM_LANGUAGE    : '\n'[Ll][Aa][Nn][Gg][Uu][Aa][Gg][Ee] ;
SYM_DESCRIPTION : '\n'[Dd][Ee][Ss][Cc][Rr][Ii][Pp][Tt][Ii][Oo][Nn] ;
SYM_DEFINITION  : '\n'[Dd][Ee][Ff][Ii][Nn][Ii][Tt][Ii][Oo][Nn] ;
SYM_INVARIANT   : '\n'[Ii][Nn][Vv][Aa][Rr][Ii][Aa][Nn][Tt] ;
SYM_ONTOLOGY    : '\n'[Oo][Nn][Tt][Oo][Ll][Oo][Gg][Yy] ;
SYM_ANNOTATIONS : '\n'[Aa][Nn][Nn][Oo][Tt][Aa][Tt][Ii][Oo][Nn][Ss] ;

// ---------------- meta-data keywords and symbols ---------------
SYM_EQ         : '=' ;
ALPHANUM_ID : [a-zA-Z0-9][a-zA-Z0-9_]* ;

