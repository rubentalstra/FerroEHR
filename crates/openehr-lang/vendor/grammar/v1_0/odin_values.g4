//
// grammar defining ODIN terminal value types, including atoms, lists and intervals
// author:      Pieter Bos <pieter.bos@nedap.com>
// support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
// copyright:   Copyright (c) 2018- openEHR Foundation <http://www.openEHR.org>
//

grammar odin_values;
import base_patterns;

string_value : STRING ;
string_list_value : string_value ( ( ',' string_value )+ | ',' SYM_LIST_CONTINUE ) ;

integer_value : ( '+' | '-' )? INTEGER ;
integer_list_value : integer_value ( ( ',' integer_value )+ | ',' SYM_LIST_CONTINUE ) ;
integer_interval_value :
      '|' SYM_GT? integer_value SYM_INTERVAL_SEP SYM_LT? integer_value '|'
    | '|' relop? integer_value '|'
    ;
integer_interval_list_value : integer_interval_value ( ( ',' integer_interval_value )+ | ',' SYM_LIST_CONTINUE ) ;

real_value : ( '+' | '-' )? REAL ;
real_list_value : real_value ( ( ',' real_value )+ | ',' SYM_LIST_CONTINUE ) ;
real_interval_value :
      '|' SYM_GT? real_value SYM_INTERVAL_SEP SYM_LT? real_value '|'
    | '|' relop? real_value '|'
    ;
real_interval_list_value : real_interval_value ( ( ',' real_interval_value )+ | ',' SYM_LIST_CONTINUE ) ;

boolean_value : SYM_TRUE | SYM_FALSE ;
boolean_list_value : boolean_value ( ( ',' boolean_value )+ | ',' SYM_LIST_CONTINUE ) ;

character_value : CHARACTER ;
character_list_value : character_value ( ( ',' character_value )+ | ',' SYM_LIST_CONTINUE ) ;

date_value : ISO8601_DATE ;
date_list_value : date_value ( ( ',' date_value )+ | ',' SYM_LIST_CONTINUE ) ;
date_interval_value :
      '|' SYM_GT? date_value SYM_INTERVAL_SEP SYM_LT? date_value '|'
    | '|' relop? date_value '|'
    ;
date_interval_list_value : date_interval_value ( ( ',' date_interval_value )+ | ',' SYM_LIST_CONTINUE ) ;

time_value : ISO8601_TIME ;
time_list_value : time_value ( ( ',' time_value )+ | ',' SYM_LIST_CONTINUE ) ;
time_interval_value :
      '|' SYM_GT? time_value SYM_INTERVAL_SEP SYM_LT? time_value '|'
    | '|' relop? time_value '|'
    ;
time_interval_list_value : time_interval_value ( ( ',' time_interval_value )+ | ',' SYM_LIST_CONTINUE ) ;

date_time_value : ISO8601_DATE_TIME ;
date_time_list_value : date_time_value ( ( ',' date_time_value )+ | ',' SYM_LIST_CONTINUE ) ;
date_time_interval_value :
      '|' SYM_GT? date_time_value SYM_INTERVAL_SEP SYM_LT? date_time_value '|'
    | '|' relop? date_time_value '|'
    ;
date_time_interval_list_value : date_time_interval_value ( ( ',' date_time_interval_value )+ | ',' SYM_LIST_CONTINUE ) ;

duration_value : ISO8601_DURATION ;
duration_list_value : duration_value ( ( ',' duration_value )+ | ',' SYM_LIST_CONTINUE ) ;
duration_interval_value :
      '|' SYM_GT? duration_value SYM_INTERVAL_SEP SYM_LT? duration_value '|'
    | '|' relop? duration_value '|'
    ;
duration_interval_list_value : duration_interval_value ( ( ',' duration_interval_value )+ | ',' SYM_LIST_CONTINUE ) ;

term_code_value : TERM_CODE_REF ;
term_code_list_value : term_code_value ( ( ',' term_code_value )+ | ',' SYM_LIST_CONTINUE ) ;

relop : SYM_GT | SYM_LT | SYM_LE | SYM_GE ;

//
//  ======================= Lexical rules ========================
//

SYM_LIST_CONTINUE: '...' ;
SYM_INTERVAL_SEP: '..' ;
