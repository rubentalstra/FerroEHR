//
// description: Antlr4 grammar for cADL primitives sub-syntax of Archetype Definition Language (ADL2)
// author:      Thomas Beale <thomas.beale@openehr.org>
// contributors:Pieter Bos <pieter.bos@nedap.com>
// support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
// copyright:   Copyright (c) 2015 openEHR Foundation
// license:     Apache 2.0 License <http://www.apache.org/licenses/LICENSE-2.0.html>
//

grammar cadl2_primitives;
import adl_keywords, odin_values;

//
//  ======================= Parser rules ========================
//

c_inline_primitive_object:
      c_integer
    | c_real
    | c_date
    | c_time
    | c_date_time
    | c_duration
    | c_string
    | c_terminology_code
    | c_boolean
    ;

c_integer: ( integer_value | integer_list_value | integer_interval_value | integer_interval_list_value ) assumed_integer_value? ;
assumed_integer_value: ';' integer_value ;

c_real: ( real_value | real_list_value | real_interval_value | real_interval_list_value ) assumed_real_value? ;
assumed_real_value: ';' real_value ;

c_date_time: ( DATE_TIME_CONSTRAINT_PATTERN | date_time_value | date_time_list_value | date_time_interval_value | date_time_interval_list_value ) assumed_date_time_value? ;
assumed_date_time_value: ';' date_time_value ;

c_date: ( DATE_CONSTRAINT_PATTERN | date_value | date_list_value | date_interval_value | date_interval_list_value ) assumed_date_value? ;
assumed_date_value: ';' date_value ;

c_time: ( TIME_CONSTRAINT_PATTERN | time_value | time_list_value | time_interval_value | time_interval_list_value ) assumed_time_value? ;
assumed_time_value: ';' time_value ;

c_duration: ( DURATION_CONSTRAINT_PATTERN ( '/' ( duration_interval_value | duration_value ))?
    | duration_value | duration_list_value | duration_interval_value | duration_interval_list_value ) assumed_duration_value?
    ;
assumed_duration_value: ';' duration_value ;

c_string: ( string_value | string_list_value ) assumed_string_value? ;
assumed_string_value: ';' string_value ;

// ADL2 term types: [ac3], [ac3; at5], [at5]
// NOTE: an assumed at-code (the ';' AT_CODE pattern) can only occur after an ac-code not after the single at-code
c_terminology_code: '[' ( AC_CODE ( ';' AT_CODE )? | AT_CODE ) ']' ;

c_boolean: ( boolean_value | boolean_list_value ) assumed_boolean_value? ;
assumed_boolean_value: ';' boolean_value ;


