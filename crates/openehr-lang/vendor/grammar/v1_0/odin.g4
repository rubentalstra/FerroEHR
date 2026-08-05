//
// description: Antlr4 grammar for Object Data Instance Notation (ODIN)
// author:      Thomas Beale <thomas.beale@openehr.org>
// contributors:Pieter Bos <pieter.bos@nedap.com>
// support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
// copyright:   Copyright (c) 2015- openEHR Foundation <http://www.openEHR.org>
// license:     Apache 2.0 License <http://www.apache.org/licenses/LICENSE-2.0.html>
//

grammar odin;
import odin_values;

//
// -------------------------- Parse Rules --------------------------
//

odin_text :
      attr_vals
    | object_value_block
    | keyed_object+
    | included_other_language
	;

attr_vals : ( attr_val ';'? )+ ;

attr_val : attribute_id '=' object_block ;

object_block :
      object_value_block
    | object_reference_block
    ;

object_value_block : ( '(' type_id ')' )? '<' ( primitive_object | attr_vals? | keyed_object* ) '>' | EMBEDDED_URI;

keyed_object : '[' primitive_value ']' '=' object_block ; // TODO: probably should limit to String and Integer?

included_other_language: INCLUDED_LANGUAGE_FRAGMENT;

// ------ leaf types ------

primitive_object :
      primitive_value 
    | primitive_list_value 
    | primitive_interval_value
    ;

primitive_value :
      string_value 
    | integer_value 
    | real_value 
    | boolean_value 
    | character_value 
    | term_code_value
    | date_value
    | time_value 
    | date_time_value 
    | duration_value
    ;

primitive_list_value :  primitive_value ( ( ',' primitive_value )+ | ',' SYM_LIST_CONTINUE ) ;


primitive_interval_value :
      integer_interval_value
    | real_interval_value
    | date_interval_value
    | time_interval_value
    | date_time_interval_value
    | duration_interval_value
    ;

object_reference_block : '<' odin_path_list '>' ;

odin_path_list     : odin_path ( ( ',' odin_path )+ | SYM_LIST_CONTINUE )? ;
odin_path          : '/' | ADL_PATH ;
