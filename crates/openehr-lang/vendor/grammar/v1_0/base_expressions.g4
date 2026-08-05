//
//  description: Antlr4 grammar for openEHR Rules core syntax.
//  author:      Thomas Beale <thomas.beale@openehr.org>
//  contributors:Pieter Bos <pieter.bos@nedap.com>
//  support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
//  copyright:   Copyright (c) 2016- openEHR Foundation <http://www.openEHR.org>
//  license:     Apache 2.0 License <http://www.apache.org/licenses/LICENSE-2.0.html>
//

grammar base_expressions;
import cadl2_primitives, odin_values;

//
//  ======================= Top-level _objects ========================
//

statement_block: statement+ ;

// ------------------------- statements ---------------------------
statement: declaration | assignment | assertion;

declaration:
      variable_declaration
    | constant_declaration
    ;

variable_declaration: local_variable ':' type_id ( SYM_ASSIGNMENT expression )? ;

constant_declaration: constant_name ':' type_id  ( SYM_EQ primitive_object )? ;

assignment:
      binding
    | local_assignment
    ;

//
// The following is the means of binding a data context path to a local variable
// TODO: remove this rule when proper external bindings are supported
binding: local_variable SYM_ASSIGNMENT bound_path ;

local_assignment: local_variable SYM_ASSIGNMENT expression ;

assertion: ( ( ALPHA_LC_ID | ALPHA_UC_ID ) ':' )? boolean_expr ;

//
// -------------------------- _expressions --------------------------
//
expression:
      boolean_expr
    | arithmetic_expr
    ;

//
// _expressions evaluating to boolean values, using standard precedence
// The equality_binop ones are not strictly necessary, but allow the use
// of boolean_leaf = true, which some people like
//
boolean_expr:
      SYM_NOT boolean_expr
    | boolean_expr SYM_AND boolean_expr
    | boolean_expr SYM_XOR boolean_expr
    | boolean_expr SYM_OR boolean_expr
    | boolean_expr SYM_IMPLIES boolean_expr
    | boolean_leaf equality_binop boolean_leaf
    | boolean_leaf
    ;

//
// Atomic Boolean-valued expression elements
// TODO: SYM_EXISTS alternative to be replaced by defined() predicate
boolean_leaf:
      boolean_literal
    | for_all_expr
    | there_exists_expr
    | SYM_EXISTS ( bound_path | sub_path_local_variable )
    | '(' boolean_expr ')'
    | relational_expr
    | equality_expr
    | constraint_expr
    | value_ref
    ;

boolean_literal:
      SYM_TRUE
    | SYM_FALSE
    ;

//
//  Universal and existential quantifier
// TODO: 'in' probably isn't needed in the long term
for_all_expr: SYM_FOR_ALL VARIABLE_ID ( ':' | 'in' ) value_ref '|'? boolean_expr ;

there_exists_expr: SYM_THERE_EXISTS VARIABLE_ID ( ':' | 'in' ) value_ref '|'? boolean_expr ;

// Constraint expressions
// This provides a way of using one operator (matches) to compare a
// value (LHS) with a value range (RHS). As per ADL, the value range
// for ordered types like Integer, Date etc may be a single value,
// a list of values, or a list of intervals, and in future, potentially
// other comparators, including functions (e.g. divisible_by_N).
//
// For non-ordered types like String and Terminology_code, the RHS
// is in other forms, e.g. regex for Strings.
//
// The matches operator can be used to generate a Boolean value that
// may be used within an expression like any other Boolean (hence it
// is a booleanLeaf).
// TODO: non-primitive objects might be supported on the RHS in future.
constraint_expr: ( arithmetic_expr | value_ref ) SYM_MATCHES  ( '{' c_inline_primitive_object '}' | CONTAINED_REGEXP );

//
// _expressions evaluating to arithmetic values, using standard precedence
//
arithmetic_expr:
      <assoc=right> arithmetic_expr '^' arithmetic_expr
    | arithmetic_expr ( '/' | '*' | '%' ) arithmetic_expr
    | arithmetic_expr ( '+' | '-' ) arithmetic_expr
    | arithmetic_leaf
    ;

arithmetic_leaf:
      integer_value
    | real_value
    | date_value
    | date_time_value
    | time_value
    | duration_value
    | value_ref
    | '(' arithmetic_expr ')'
    ;

//
// Equality expression between any arithmetic value; precedence is
// lowest, so only needed between leaves, since () will be needed for
// larger expressions anyway
//
equality_expr: arithmetic_expr equality_binop arithmetic_expr ;

equality_binop:
      SYM_EQ
    | SYM_NE
    ;

//
// Relational expressions of arithmetic operands generating Boolean values
//
relational_expr: arithmetic_expr relational_binop arithmetic_expr ;

relational_binop:
      SYM_GT
    | SYM_LT
    | SYM_LE
    | SYM_GE
    ;

//
// instances references: data references, variables, and function calls.
// TODO: Remove bound_path from this rule when external binding supported
//
value_ref:
      function_call
    | bound_path
    | sub_path_local_variable
    | local_variable
    | constant_name
    ;

local_variable: VARIABLE_ID ;

// TODO: change to [] form, e.g.     book_list [{title.contains("Quixote")}]
sub_path_local_variable: VARIABLE_WITH_PATH;

// TODO: Remove this rule when external binding supported
bound_path: ADL_PATH ;

constant_name: ALPHA_UC_ID ;

function_call: ALPHA_LC_ID '(' function_args? ')' ;

function_args: expression ( ',' expression )* ;

type_id: ALPHA_UC_ID ( '<' type_id ( ',' type_id )* '>' )? ;

