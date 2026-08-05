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
	;

attr_vals : ( attr_val ';'? )+ ;

attr_val : odin_object_key '=' object_block ;

odin_object_key : ALPHA_UC_ID | ALPHA_UNDERSCORE_ID | rm_attribute_id ;

object_block :
      object_value_block
    | object_reference_block
    ;

object_value_block : ( '(' rm_type_id ')' )? '<' ( primitive_object | attr_vals? | keyed_object* ) '>' | EMBEDDED_URI;

keyed_object : '[' key_id ']' '=' object_block ;

key_id :
      string_value
    | integer_value
    | date_value
    | time_value
    | date_time_value
    ;

object_reference_block : '<' odin_path_list '>' ;

odin_path_list  : odin_path ( ( ',' odin_path )+ | SYM_LIST_CONTINUE )? ;
odin_path       : '/' | ADL_PATH ;

rm_type_id      : ALPHA_UC_ID ( '<' rm_type_id ( ',' rm_type_id )* '>' )? ;
rm_attribute_id : ALPHA_LC_ID ;
