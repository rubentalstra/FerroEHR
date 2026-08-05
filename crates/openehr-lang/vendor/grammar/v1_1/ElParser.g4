//
//  description: Antlr4 grammar for openEHR Expression Language baed on BMM meta-model.
//  author:      Thomas Beale <thomas.beale@openehr.org>
//  contributors:Pieter Bos <pieter.bos@nedap.com>
//  support:     openEHR Specifications PR tracker <https://openehr.atlassian.net/projects/SPECPR/issues>
//  copyright:   Copyright (c) 2016- openEHR Foundation <http://www.openEHR.org>
//  license:     Apache 2.0 License <http://www.apache.org/licenses/LICENSE-2.0.html>
//

parser grammar ElParser;
options { tokenVocab=ElLexer; }
import Cadl2Parser;



// ========================== EL Statements ==========================

statementBlock: statement+ EOF? ;

statement: ( declaration | assignment | assertion ) ';' ;

declaration:
      variableDecl
    | constantDecl
    ;

variableDecl: elInstantiableRef ':' typeId ( SYM_ASSIGNMENT elExpression )? ;

constantDecl: UC_ID ':' typeId SYM_EQ elExpression ;

assignment: elValueGenerator SYM_ASSIGNMENT elExpression ;

assertion: LC_ID ':' elBooleanExpr ;

// ========================== Type names ==========================

typeId: UC_ID ( '<' typeId ( ',' typeId )* '>' )? ;

// ========================== EL Expressions ==========================

//
// Expressions are either value-generators, or operator expressions (containing value-generators)
//
elExpression:
      elTerminal
    | elOperatorExpression
    | elTuple
    ;

elOperatorExpression:
      elBooleanExpr
    | elArithmeticExpr
    ;

// ------------------- Boolean-returning operator expressions --------------------

//
// Expressions evaluating to boolean values, using standard precedence;
// These map to ordinary 1- and 2-argument function calls on Boolean instances
//
elBooleanExpr:
      SYM_NOT elBooleanExpr
    | elBooleanExpr SYM_AND elBooleanExpr
    | elBooleanExpr SYM_XOR elBooleanExpr
    | elBooleanExpr SYM_OR elBooleanExpr
    | elBooleanExpr SYM_IMPLIES elBooleanExpr
    | elBooleanExpr ( SYM_IFF | SYM_EQ ) elBooleanExpr
    | elBooleanLeaf
    ;

//
// Atomic Boolean-valued expression elements
//
elBooleanLeaf:
      booleanValue
    | elForAllExpr
    | elThereExistsExpr
    | elArithmeticConstraintExpr
    | elGeneralConstraintExpr
    | '(' elBooleanExpr ')'
    | SYM_EXISTS elValueGenerator
    | elArithmeticComparisonExpr
    | elObjectComparisonExpr
    | elValueGenerator
    ;

//
//  Universal and existential quantifier
//
elForAllExpr: SYM_FOR_ALL elLocalVariableId ':' elValueGenerator '¦' elBooleanExpr ;

elThereExistsExpr: SYM_THERE_EXISTS elLocalVariableId ':' elValueGenerator '¦' elBooleanExpr ;

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
elArithmeticConstraintExpr: elArithmeticLeaf SYM_MATCHES '{' cInlineOrderedObject '}' ;

elGeneralConstraintExpr: elSimpleTerminal SYM_MATCHES '{' cObjectMatcher '}' ;

// --------------------------- Arithmetic operator expressions --------------------------

//
// Comparison expressions of arithmetic operands generating Boolean results
//
elArithmeticComparisonExpr: elArithmeticExpr elComparisonBinop elArithmeticExpr ;

elComparisonBinop:
      SYM_EQ
    | SYM_NE
    | SYM_GT
    | SYM_LT
    | SYM_LE
    | SYM_GE
    ;

//
// Expressions evaluating to values of arithmetic types, using standard precedence
//
elArithmeticExpr:
      <assoc=right> elArithmeticExpr '^' elArithmeticExpr
    | elArithmeticExpr ( '/' | SYM_ASTERISK | '%' ) elArithmeticExpr
    | elArithmeticExpr ( '+' | '-' ) elArithmeticExpr
    | elArithmeticLeaf
    ;

// TODO: need to be able to plug in terminal to allow decision tables in expressions
elArithmeticLeaf:
      elArithmeticValue
    | '(' elArithmeticExpr ')'
    | elValueGenerator
    | dlSimpleCaseTable
    ;

elArithmeticValue:
      integerValue
    | realValue
    | dateValue
    | dateTimeValue
    | timeValue
    | durationValue
    ;

// -------------------- Equality operator expressions for other types ------------------------

//
// Compare any kind of objects
//
elObjectComparisonExpr: elSimpleTerminal elEqualityBinop elSimpleTerminal ;

elEqualityBinop:
    SYM_EQ
  | SYM_NE
  ;

//
// -------------------------- tuples -----------------------------
//

elTuple: '[' elExpression ( ',' elExpression )+ ']';

//
// -------------------------- value-generating expressions -----------------------------
//

elTerminal:
      elSimpleTerminal
    | dlDecisionTable
    ;

elSimpleTerminal:
      primitiveObject
    | elValueGenerator
    ;

//
// TODO: Can't syntactically distinguish between a local variable
//       and a property or constant reference.
//
elValueGenerator:
      SYM_SELF
    | elBareRef
    | elScopedFeatureRef
    ;

//
// An unscoped reference of some kind
// Will map to EL_WRITABLE_VARIABLE or EL_PROPERTY_REF (unscoped)
//
elBareRef:
      elInstantiableRef
    | elFunctionCall
    | elConstantId
    ;

//
// Instantiable feature refs; may be the target of an assignment
//
elInstantiableRef:
      SYM_RESULT
    | elBoundVariableId
    | elLocalVariableId
    ;


//
// Scoped feature references. Has to have at least one scoping element, either a
// class name in {} or else a baseRef name; then any number of bareRefs.
// Will map to any EL_FEATURE_REF (scoped)
//
elScopedFeatureRef: elScoper elBareRef ;

elScoper: ( '{' typeId '}' | elBareRef ) '.'  ( elBareRef '.' )* ;

//
// A variable bound to a data source, lexical form '$xxxx'
// TODO: analyse how a boundVariableId can be created as a built-in feature
//
elBoundVariableId: BOUND_VARIABLE_ID ;

elLocalVariableId: LC_ID ;

elConstantId: UC_ID ;

//
// Function calls
//
elFunctionCall: LC_ID ( '(' elExprList ')' )? ;

elExprList: elExpression ( ',' elExpression )* ;

//
// -------------------------- decision tables -----------------------------
//

dlDecisionTable:
      dlBinaryChoice
    | dlCaseTable
    | dlConditionTable
    ;

dlCaseTable:
    | dlSimpleCaseTable
    | dlGeneralCaseTable
    ;

//
// condition chains (if/then statement equivalent)
// choice in
//   =========================================================
//   er_positive and
//   her2_negative and
//   not ki67.in_range (#high):    #luminal_A,
//   ---------------------------------------------------------
//   er_positive and
//   her2_negative and
//   ki67.in_range (#high):        #luminal_B_HER2_negative,
//   ---------------------------------------------------------
//   *:                            #none
//   =========================================================
//
dlConditionTable: SYM_CHOICE SYM_IN BLOCK_DELIM ( dlConditionBranch ',' )+ ( dlConditionBranch | dlConditionDefaultBranch ) BLOCK_DELIM ;

dlConditionBranch: elBooleanExpr ':' elExpression ;

dlConditionDefaultBranch: SYM_ASTERISK ':' elExpression ;

//
// Binary-choice version of condition table, using old-school
// C/Java syntax:
// booleanExpr ? x : y ;
//
dlBinaryChoice:  elBooleanExpr '?' elSimpleTerminal ':' elSimpleTerminal ;

//
// Case tables, e.g.:
//     Result := case qCSI_score in
//        ============================
//        0:          expr0,
//        ----------------------------
//        |1..2|:     expr1,
//        ----------------------------
//        |3..5|:     expr2,
//        ----------------------------
//        |6..8|:     expr3,
//        ----------------------------
//        |≥ 9|:      expr4
//        ============================
//     ;
//
dlGeneralCaseTable: SYM_CASE elExpression SYM_IN BLOCK_DELIM ( dlGeneralCaseBranch ',' )+ ( dlGeneralCaseBranch | dlGeneralCaseDefaultBranch ) BLOCK_DELIM ;

dlGeneralCaseBranch: primitiveObject ':' elExpression ;

dlGeneralCaseDefaultBranch: SYM_ASTERISK ':' elExpression ;

//
// Simple value-based (typed) Case tables, e.g.:
// case gfr_range in
//   =================
//   |>20|:      1,
//   |10..20|:   0.75,
//   |<10|:      0.5
//   =================
//   ;
//
dlSimpleCaseTable: SYM_CASE elSimpleTerminal SYM_IN BLOCK_DELIM ( dlSimpleCaseBranch ',' )+ ( dlSimpleCaseBranch | dlSimpleCaseDefaultBranch ) BLOCK_DELIM ;

dlSimpleCaseBranch: primitiveObject ':' elSimpleTerminal ;

dlSimpleCaseDefaultBranch: SYM_ASTERISK ':' elSimpleTerminal ;
