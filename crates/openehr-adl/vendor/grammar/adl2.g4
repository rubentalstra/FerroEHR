//
//  description: Antlr4 grammar for Archetype Definition Language (ADL2)
//  author:      Thomas Beale <thomas.beale@openehr.org>
//  contributors:Pieter Bos <pieter.bos@nedap.com>
//  support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
//  copyright:   Copyright (c) 2015- openEHR Foundation <http://www.openEHR.org>
//  license:     Apache 2.0 License <http://www.apache.org/licenses/LICENSE-2.0.html>
//

grammar Adl2;
import cadl2;

//
//  ============== Parser rules ==============
//

adl2Archetype: ( authored_archetype | template | template_overlay | operational_template ) EOF ;

authored_archetype:
    SYM_ARCHETYPE meta_data?
    archetypeHrid
    specialize_section?
    language_section
    description_section
    definition_section
    rules_section?
    terminology_section
    annotations_section?
    ;

template: 
    SYM_TEMPLATE meta_data? 
    archetypeHrid
    specialize_section
    language_section
    description_section
    definition_section
    rules_section?
    terminology_section
    annotations_section?
    (template_overlay)*
    ;

template_overlay: 
    SYM_TEMPLATE_OVERLAY 
    archetypeHrid
    specialize_section
    definition_section
    terminology_section
    ;

operational_template: 
    SYM_OPERATIONAL_TEMPLATE meta_data? 
    archetypeHrid
    language_section
    description_section
    definition_section
    rules_section?
    terminology_section
    annotations_section?
    component_terminologies_section?
    ;

archetypeHrid: ARCHETYPE_HRID;

specialize_section  : SYM_SPECIALIZE archetype_ref ;
language_section    : SYM_LANGUAGE odin_text ;
description_section : SYM_DESCRIPTION odin_text ;
definition_section  : SYM_DEFINITION c_complex_object ;
rules_section       : SYM_RULES statement_block ;
terminology_section : SYM_TERMINOLOGY odin_text ;
annotations_section : SYM_ANNOTATIONS odin_text ;
component_terminologies_section: SYM_COMPONENT_TERMINOLOGIES odin_text ;

meta_data: '(' meta_data_item  (';' meta_data_item )* ')' ;

meta_data_item:
      meta_data_tag_adl_version '=' VERSION_ID
    | meta_data_tag_uid '=' GUID
    | meta_data_tag_build_uid '=' GUID
    | meta_data_tag_rm_release '=' VERSION_ID
    | meta_data_tag_is_controlled
    | meta_data_tag_is_generated
    | odin_object_key ( '=' meta_data_value )?
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

