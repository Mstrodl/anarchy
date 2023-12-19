use bimap::BiHashMap;
use lazy_static::lazy_static;
pub use pest;
use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use std::fmt;
use std::iter::zip;
use std::rc::Rc;
use std::sync::Mutex;

#[derive(Parser)]
#[grammar = "anarchy.pest"] // relative to src
struct AnarchyParser;

#[derive(Clone, Debug)]
pub enum Value {
  Number(f32),
  Tuple(Rc<Vec<Value>>),
}

#[derive(Clone, Debug)]
pub enum ValueType {
  Number,
  Tuple,
}

impl fmt::Display for ValueType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{self:?}")
  }
}

impl fmt::Display for Value {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Value::Number(number) => write!(f, "Number({number})"),
      Value::Tuple(tuple) => write!(
        f,
        "Tuple({})",
        tuple
          .iter()
          .map(|value| value.to_string())
          .collect::<Vec<String>>()
          .join(", ")
      ),
    }
  }
}

impl fmt::Display for Location {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}:{} to {}:{}",
      self.start_line, self.start_column, self.end_line, self.end_column
    )
  }
}

impl fmt::Display for LanguageError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self.location {
      Some(location) => write!(f, "LanguageError @ {}: {}", location, self.error),
      None => write!(f, "LanguageError: {}", self.error),
    }
  }
}

impl fmt::Display for LanguageErrorType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      LanguageErrorType::Type(expected_type, actual_value) => write!(
        f,
        "TypeError: Expected value of type {expected_type}, got: {actual_value}",
      ),
      LanguageErrorType::Reference(identifier) => write!(
        f,
        "ReferenceError: Couldn't find identifier named {identifier}",
      ),
      LanguageErrorType::Range(index, length) => write!(
        f,
        "RangeError: Index {index} out of bounds for tuple of length {length}"
      ),
      LanguageErrorType::ArgumentCountMismatch(found, expected) => write!(
        f,
        "ArgumentCountMismatch: Function takes {expected} arguments, but you used: {found}"
      ),
    }
  }
}

struct TrackedValue<'a>(Value, &'a Location);
pub struct UntrackedValue(pub Value);

impl TryFrom<UntrackedValue> for f32 {
  type Error = LanguageError;
  fn try_from(UntrackedValue(value): UntrackedValue) -> Result<f32, LanguageError> {
    match value {
      Value::Number(number) => Ok(number),
      value => Err(LanguageError {
        error: LanguageErrorType::Type(ValueType::Number, value),
        location: None,
      }),
    }
  }
}

impl<'a> TryFrom<TrackedValue<'a>> for f32 {
  type Error = LanguageError;
  fn try_from(TrackedValue(value, location): TrackedValue<'a>) -> Result<f32, LanguageError> {
    match value {
      Value::Number(number) => Ok(number),
      value => Err(LanguageError {
        error: LanguageErrorType::Type(ValueType::Number, value),
        location: Some(location.clone()),
      }),
    }
  }
}
impl From<f32> for Value {
  fn from(number: f32) -> Value {
    Value::Number(number)
  }
}
impl From<bool> for Value {
  fn from(boolean: bool) -> Value {
    Value::Number(if boolean { 1.0 } else { 0.0 })
  }
}

impl<'a, 'b> TryFrom<&'b TrackedValue<'a>> for Rc<Vec<Value>> {
  type Error = LanguageError;
  fn try_from(
    TrackedValue(value, location): &'b TrackedValue<'a>,
  ) -> Result<Rc<Vec<Value>>, LanguageError> {
    match value {
      Value::Tuple(tuple) => Ok(Rc::clone(tuple)),
      value => Err(LanguageError {
        error: LanguageErrorType::Type(ValueType::Tuple, value.clone()),
        location: Some((*location).clone()),
      }),
    }
  }
}

impl From<Rc<Vec<Value>>> for Value {
  fn from(tuple: Rc<Vec<Value>>) -> Value {
    Value::Tuple(tuple)
  }
}

#[derive(Debug, Clone)]
pub struct LanguageError {
  pub location: Option<Location>,
  pub error: LanguageErrorType,
}

#[derive(Debug, Clone)]
pub enum LanguageErrorType {
  Type(ValueType, Value),
  Reference(String),
  Range(usize, usize),
  ArgumentCountMismatch(usize, usize),
}

lazy_static! {
    pub static ref PRATT_PARSER: PrattParser<Rule> = {
        // Lower = higher priority
        PrattParser::new()
            .op(Op::infix(Rule::and, Assoc::Left) | Op::infix(Rule::or, Assoc::Left))
            .op(Op::infix(Rule::eq, Assoc::Left)
                | Op::infix(Rule::lt, Assoc::Left)
                | Op::infix(Rule::gt, Assoc::Left)
                | Op::infix(Rule::gteq, Assoc::Left)
                | Op::infix(Rule::lteq, Assoc::Left)
                | Op::infix(Rule::neq, Assoc::Left))
            .op(Op::infix(Rule::xor, Assoc::Left)
                | Op::infix(Rule::band, Assoc::Left)
                | Op::infix(Rule::shift_left, Assoc::Left)
                | Op::infix(Rule::shift_right, Assoc::Left)
                | Op::infix(Rule::bor, Assoc::Left))
            .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
            .op(Op::infix(Rule::mul, Assoc::Left) | Op::infix(Rule::div, Assoc::Left) | Op::infix(Rule::modulo, Assoc::Left))
            .op(Op::infix(Rule::pow, Assoc::Left))
            .op(Op::prefix(Rule::invert))
            .op(Op::prefix(Rule::neg))
            .op(Op::postfix(Rule::index))
    };
}

#[derive(Debug, Clone)]
struct Function {
  // name: String,
  arguments: Vec<Identifier>,
  contents: Vec<Statement>,
}

#[derive(Debug, Clone)]
struct FunctionPrototype {
  identifier: Identifier,
  argument_count: usize,
}

#[derive(Debug, Clone)]
pub struct ParsedLanguage {
  top_level: Vec<Statement>,
  functions: Vec<Function>,
}

impl From<LanguageError> for ParseError {
  fn from(error: LanguageError) -> Self {
    Self::LanguageError(error)
  }
}

pub fn parse(
  execution_context: Rc<Mutex<ExecutionContext>>,
  code: &str,
) -> Result<ParsedLanguage, ParseError> {
  let mut program = AnarchyParser::parse(Rule::program, code)
    .map_err(|err| ParseError::PestError(Box::new(err)))?
    .next()
    .unwrap()
    .into_inner();
  let function_definitions = program.next().unwrap().into_inner();
  let mut functions: Vec<Function> = Vec::new();
  let mut functions_map = HashMap::new();
  for function_definition in function_definitions {
    println!("Function Definition: {function_definition:?}");
    let mut function_definition = function_definition.into_inner();
    let function_name = function_definition.next().unwrap().as_str().to_string();
    let arguments = function_definition
      .next()
      .unwrap()
      .into_inner()
      .map(|arg| {
        execution_context.lock().unwrap().register(VariableKey {
          name: arg.as_str().to_string(),
          scope: function_name.to_string(),
        })
      })
      .collect::<Vec<Identifier>>();
    let statement_block = function_definition.next().unwrap();
    let contents = parse_statement_block(
      execution_context.clone(),
      function_name.clone(),
      statement_block.into_inner(),
      &functions_map,
    )?;
    functions_map.insert(
      function_name.clone(),
      FunctionPrototype {
        identifier: functions.len(),
        argument_count: arguments.len(),
      },
    );
    functions.push(Function {
      // name: function_name,
      arguments,
      contents,
    });
  }
  let statement_block = program.next().unwrap();

  Ok(ParsedLanguage {
    top_level: parse_statement_block(
      execution_context,
      "".to_string(),
      statement_block.into_inner(),
      &functions_map,
    )?,
    functions,
  })
}

// pub fn execute(
//     context: &mut ExecutionContext,
//     pairs: ParsedLanguage<'_>,
// ) -> Result<(), LanguageError> {
//     //let mut context = ExecutionContext::default();
//     execute_statement_block(context, pairs.0)
// }

pub fn execute(
  context: &mut ExecutionContext,
  ParsedLanguage {
    top_level: pairs,
    functions,
  }: &ParsedLanguage,
) -> Result<Option<Value>, LanguageError> {
  execute_statement_block(context, pairs, functions)
}

fn execute_statement_block(
  context: &mut ExecutionContext,
  statements: &Vec<Statement>,
  functions: &Vec<Function>,
) -> Result<Option<Value>, LanguageError> {
  for statement in statements {
    if let Some(value) = statement.execute(context, functions)? {
      // Bail early (e.g. return function!)
      return Ok(Some(value));
    }
  }
  Ok(None)
}

impl Statement {
  fn execute(
    &self,
    context: &mut ExecutionContext,
    functions: &Vec<Function>,
  ) -> Result<Option<Value>, LanguageError> {
    match self {
      Statement::Assignment { variable, value } => {
        let value = value.evaluate(context, functions)?;
        context.set(*variable, value);
      }
      Statement::If(if_statement) => {
        if_statement.execute(context, functions)?;
      }
      Statement::Return(expression) => {
        return Ok(Some(expression.evaluate(context, functions)?));
      }
    };
    Ok(None)
  }
}

impl IfStatement {
  fn execute(
    &self,
    context: &mut ExecutionContext,
    functions: &Vec<Function>,
  ) -> Result<Option<Value>, LanguageError> {
    let condition = f32::try_from(TrackedValue(
      self.condition.evaluate(context, functions)?,
      &self.condition.location,
    ))?;
    if condition != 0.0 {
      execute_statement_block(context, &self.if_branch, functions)
    } else {
      match &self.else_branch {
        ElseBranch::IfStatement(if_statement) => if_statement.execute(context, functions),
        ElseBranch::ElseStatement(else_block) => {
          execute_statement_block(context, else_block, functions)
        }
        ElseBranch::None => Ok(None),
      }
    }
  }
}

#[derive(Debug, Clone)]
enum FunctionIdentifier {
  Sin,
  Cos,
  Tan,
  Abs,
  Sqrt,
  Log,
  Acos,
  Asin,
  Atan,
  Len,
  UserDefined(Identifier),
}

impl Expression {
  fn evaluate(
    &self,
    context: &mut ExecutionContext,
    functions: &Vec<Function>,
  ) -> Result<Value, LanguageError> {
    Ok(match &self.op {
      ExpressionOp::Reference(identifier) => context.get(*identifier, &self.location)?,
      ExpressionOp::FunctionCall(function, arguments) => match function {
        FunctionIdentifier::Len => {
          let tracked_value = TrackedValue(
            arguments[0].evaluate(context, functions)?,
            &arguments[0].location,
          );
          let value: Rc<Vec<Value>> = <Rc<Vec<Value>>>::try_from(&tracked_value)?;
          Value::from(value.len() as f32)
        }
        FunctionIdentifier::UserDefined(identifier) => {
          let function = &functions[*identifier];
          for (argument_id, arg_expression) in zip(function.arguments.iter(), arguments.iter()) {
            let arg_value = arg_expression.evaluate(context, functions)?;
            context.set(*argument_id, arg_value);
          }
          execute_statement_block(context, &function.contents, functions)?
            .unwrap_or(Value::Number(0.0_f32))
        }
        function => {
          let value = f32::try_from(TrackedValue(
            arguments[0].evaluate(context, functions)?,
            &arguments[0].location,
          ))?;
          Value::from(match function {
            FunctionIdentifier::Sin => value.sin(),
            FunctionIdentifier::Cos => value.cos(),
            FunctionIdentifier::Tan => value.tan(),
            FunctionIdentifier::Asin => value.asin(),
            FunctionIdentifier::Acos => value.acos(),
            FunctionIdentifier::Atan => value.atan(),
            FunctionIdentifier::Abs => value.abs(),
            FunctionIdentifier::Sqrt => value.sqrt(),
            FunctionIdentifier::Log => value.log(2.0),
            FunctionIdentifier::Len => unreachable!(),
            FunctionIdentifier::UserDefined(_) => unreachable!(),
          })
        }
      },
      ExpressionOp::NumberLiteral(number) => (*number).into(),
      ExpressionOp::TupleLiteral(expressions) => Value::Tuple(Rc::new(
        expressions
          .iter()
          .map(|expression| expression.evaluate(context, functions))
          .collect::<Result<Vec<Value>, _>>()?,
      )),
      ExpressionOp::Index(tuple, index) => {
        let index_num = f32::try_from(TrackedValue(
          index.evaluate(context, functions)?,
          &index.location,
        ))? as usize;
        let tuple = <Rc<Vec<Value>>>::try_from(&TrackedValue(
          tuple.evaluate(context, functions)?,
          &tuple.location,
        ))?;
        tuple
          .get(index_num)
          .ok_or_else(|| LanguageError {
            error: LanguageErrorType::Range(index_num, tuple.len()),
            location: Some(index.location.clone()),
          })?
          .clone()
      }
      ExpressionOp::Pow(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
        .powf(f32::try_from(TrackedValue(
          rhs.evaluate(context, functions)?,
          &rhs.location,
        ))?),
      ),
      ExpressionOp::Modulo(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          % f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?,
      ),
      ExpressionOp::Add(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          + f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?,
      ),
      ExpressionOp::Sub(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          - f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?,
      ),
      ExpressionOp::Mul(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          * f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?,
      ),
      ExpressionOp::Div(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          / f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?,
      ),
      ExpressionOp::BinaryAnd(lhs, rhs) => Value::from(
        (f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))? as u32
          & f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))? as u32) as f32,
      ),
      ExpressionOp::Xor(lhs, rhs) => Value::from(
        (f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))? as u32
          ^ f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))? as u32) as f32,
      ),
      ExpressionOp::ShiftLeft(lhs, rhs) => Value::from(
        ((f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))? as u32)
          << (f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))? as u32)) as f32,
      ),
      ExpressionOp::ShiftRight(lhs, rhs) => Value::from(
        ((f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))? as u32)
          >> (f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))? as u32)) as f32,
      ),
      ExpressionOp::BinaryOr(lhs, rhs) => Value::from(
        (f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))? as u32
          | f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))? as u32) as f32,
      ),
      ExpressionOp::GreaterThan(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          > f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?,
      ),
      ExpressionOp::LessThan(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          < f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?,
      ),
      ExpressionOp::GreaterThanOrEqual(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          >= f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?,
      ),
      ExpressionOp::LessThanOrEqual(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          <= f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?,
      ),
      ExpressionOp::Equal(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          == f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?,
      ),
      ExpressionOp::NotEqual(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          != f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?,
      ),
      ExpressionOp::Neg(value) => Value::from(-f32::try_from(TrackedValue(
        value.evaluate(context, functions)?,
        &value.location,
      ))?),
      ExpressionOp::Invert(value) => Value::from(
        if f32::try_from(TrackedValue(
          value.evaluate(context, functions)?,
          &value.location,
        ))?
          == 0.0
        {
          1.0
        } else {
          0.0
        },
      ),
      ExpressionOp::And(lhs, rhs) => Value::from(
        if f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?
          != 0.0
        {
          f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?
        } else {
          0.0
        },
      ),
      ExpressionOp::Or(lhs, rhs) => {
        let lhs = f32::try_from(TrackedValue(
          lhs.evaluate(context, functions)?,
          &lhs.location,
        ))?;
        Value::from(if lhs != 0.0 {
          lhs
        } else {
          f32::try_from(TrackedValue(
            rhs.evaluate(context, functions)?,
            &rhs.location,
          ))?
        })
      }
    })
  }
}

fn parse_statement_block(
  execution_context: Rc<Mutex<ExecutionContext>>,
  scope: String,
  pairs: Pairs<Rule>,
  functions: &HashMap<String, FunctionPrototype>,
) -> Result<Vec<Statement>, LanguageError> {
  pairs
    .filter(|pair| pair.as_rule() == Rule::statement)
    .map(|pair| {
      parse_statement(
        execution_context.clone(),
        scope.clone(),
        pair.into_inner().next().unwrap(),
        functions,
      )
    })
    .collect::<Result<Vec<Statement>, LanguageError>>()
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct VariableKey {
  // variable name
  pub name: String,
  // e.g. function this belongs to
  pub scope: String,
}

impl fmt::Display for VariableKey {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}::{}", self.scope, self.name)
  }
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionContextLUT {
  scope_locations: BiHashMap<VariableKey, usize>,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
  scope_locations: ExecutionContextLUT,
  scope: Vec<Option<Value>>,
}
impl fmt::Display for ExecutionContext {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{{")?;
    let mut scope_iter = self
      .scope_locations
      .scope_locations
      .iter()
      .filter_map(|(key, index)| Some((key, self.scope[*index].clone()?)))
      .peekable();
    while let Some((key, value)) = scope_iter.next() {
      write!(f, "{key} = {value}")?;
      if scope_iter.peek().is_some() {
        write!(f, ", ")?;
      }
    }
    write!(f, "}}")
  }
}
impl ExecutionContext {
  pub fn new_with_scope_locations(scope_locations: ExecutionContextLUT) -> Self {
    let length = scope_locations.scope_locations.len();
    let mut scope = Vec::with_capacity(length);
    scope.resize(length, None);
    Self {
      scope_locations,
      scope,
    }
  }
  pub fn export_scope_locations(&self) -> ExecutionContextLUT {
    self.scope_locations.clone()
  }
  pub fn register(&mut self, key: VariableKey) -> Identifier {
    match self.scope_locations.scope_locations.get_by_left(&key) {
      Some(index) => *index,
      None => {
        let index = self.scope.len();
        self.scope.push(None);
        self.scope_locations.scope_locations.insert(key, index);
        index
      }
    }
  }
  #[inline(always)]
  fn inner_get(
    &self,
    identifier: Identifier,
    location: Option<&Location>,
  ) -> Result<Value, LanguageError> {
    self.scope[identifier].clone().ok_or_else(|| LanguageError {
      error: LanguageErrorType::Reference(
        self
          .scope_locations
          .scope_locations
          .get_by_right(&identifier)
          .unwrap()
          .to_string(),
      ),
      location: location.cloned(),
    })
  }
  #[inline(always)]
  fn get(&self, identifier: Identifier, location: &Location) -> Result<Value, LanguageError> {
    self.inner_get(identifier, Some(location))
  }
  pub fn unattributed_get(&mut self, identifier: Identifier) -> Result<Value, LanguageError> {
    self.inner_get(identifier, None)
  }
  #[inline(always)]
  pub fn set(&mut self, identifier: Identifier, value: Value) {
    self.scope[identifier] = Some(value);
  }
  #[inline(always)]
  pub fn set_runtime(&mut self, identifier: &str, value: Value) {
    let index = self.register(VariableKey {
      name: identifier.to_string(),
      scope: "".to_string(),
    });
    self.set(index, value);
  }
  #[inline(always)]
  pub fn reset(&mut self) {
    // Reset all values to None
    self.scope.fill(None);
  }
}

type Identifier = usize;
#[derive(Debug, Clone)]
enum ElseBranch {
  IfStatement(Box<IfStatement>),
  ElseStatement(Vec<Statement>),
  None,
}

impl<'a> From<&Pair<'a, Rule>> for Location {
  fn from(pair: &Pair<'a, Rule>) -> Location {
    let (start_line, start_column) = pair.line_col();
    let (end_line, end_column) = pair.as_span().end_pos().line_col();
    Location {
      start_line,
      start_column,
      end_line,
      end_column,
    }
  }
}

#[derive(Debug, Clone)]
pub struct Location {
  pub start_line: usize,
  pub start_column: usize,
  pub end_line: usize,
  pub end_column: usize,
}
#[derive(Debug, Clone)]
struct Expression {
  location: Location,
  op: ExpressionOp,
}
#[derive(Debug, Clone)]
enum ExpressionOp {
  Add(Box<Expression>, Box<Expression>),
  Mul(Box<Expression>, Box<Expression>),
  Sub(Box<Expression>, Box<Expression>),
  Div(Box<Expression>, Box<Expression>),
  BinaryAnd(Box<Expression>, Box<Expression>),
  Xor(Box<Expression>, Box<Expression>),
  ShiftLeft(Box<Expression>, Box<Expression>),
  ShiftRight(Box<Expression>, Box<Expression>),
  BinaryOr(Box<Expression>, Box<Expression>),
  GreaterThan(Box<Expression>, Box<Expression>),
  LessThan(Box<Expression>, Box<Expression>),
  LessThanOrEqual(Box<Expression>, Box<Expression>),
  GreaterThanOrEqual(Box<Expression>, Box<Expression>),
  Equal(Box<Expression>, Box<Expression>),
  NotEqual(Box<Expression>, Box<Expression>),
  NumberLiteral(f32),
  TupleLiteral(Vec<Expression>),
  Reference(Identifier),
  Index(Box<Expression>, Box<Expression>),
  Neg(Box<Expression>),
  Invert(Box<Expression>),
  Or(Box<Expression>, Box<Expression>),
  And(Box<Expression>, Box<Expression>),
  FunctionCall(FunctionIdentifier, Vec<Expression>),
  Modulo(Box<Expression>, Box<Expression>),
  Pow(Box<Expression>, Box<Expression>),
}
#[derive(Debug, Clone)]
struct IfStatement {
  condition: Expression,
  if_branch: Vec<Statement>,
  else_branch: ElseBranch,
}
#[derive(Debug, Clone)]
enum Statement {
  Assignment {
    variable: Identifier,
    value: Expression,
  },
  If(IfStatement),
  Return(Expression),
}

pub type PestError = pest::error::Error<Rule>;

#[derive(Debug, Clone)]
pub enum ParseError {
  PestError(Box<PestError>),
  LanguageError(LanguageError),
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::PestError(error) => write!(f, "PestError: {error}"),
      Self::LanguageError(error) => write!(f, "LanguageError: {error}"),
    }
  }
}

fn parse_expression(
  execution_context: Rc<Mutex<ExecutionContext>>,
  scope: String,
  pairs: Pairs<Rule>,
  functions: &HashMap<String, FunctionPrototype>,
) -> Result<Expression, LanguageError> {
  let execution_context = &execution_context;
  PRATT_PARSER
    .map_primary(|primary| {
      let execution_context = execution_context.clone();
      let location = Location::from(&primary);
      let op = match primary.as_rule() {
        Rule::number_literal => {
          ExpressionOp::NumberLiteral(primary.as_str().parse::<f32>().unwrap())
        }
        Rule::tuple_literal => ExpressionOp::TupleLiteral(
          primary
            .into_inner()
            .map(|entry| {
              parse_expression(
                execution_context.clone(),
                scope.clone(),
                entry.into_inner(),
                functions,
              )
            })
            .collect::<Result<Vec<Expression>, LanguageError>>()?,
        ),
        Rule::identifier => {
          ExpressionOp::Reference(execution_context.lock().unwrap().register(VariableKey {
            name: primary.as_str().to_string(),
            scope: scope.clone(),
          }))
        }
        Rule::expr => {
          parse_expression(
            execution_context,
            scope.clone(),
            primary.into_inner(),
            functions,
          )?
          .op
        }
        Rule::function_call => {
          let mut pairs = primary.into_inner();
          let op_identifier = pairs.next().unwrap();
          let arguments_pairs = pairs.next().unwrap();
          let argument_pairs_location = Location::from(&arguments_pairs);
          let arguments = arguments_pairs
            .into_inner()
            .map(|expression| {
              parse_expression(
                execution_context.clone(),
                scope.clone(),
                expression.into_inner(),
                functions,
              )
            })
            .collect::<Result<Vec<Expression>, LanguageError>>()?;
          let op = match op_identifier.as_str() {
            "sin" => FunctionIdentifier::Sin,
            "cos" => FunctionIdentifier::Cos,
            "tan" => FunctionIdentifier::Tan,
            "asin" => FunctionIdentifier::Asin,
            "acos" => FunctionIdentifier::Acos,
            "atan" => FunctionIdentifier::Atan,
            "abs" => FunctionIdentifier::Abs,
            "sqrt" => FunctionIdentifier::Sqrt,
            "log" => FunctionIdentifier::Log,
            "len" => FunctionIdentifier::Len,
            name => {
              let function = functions.get(name).ok_or_else(|| LanguageError {
                location: Some(Location::from(&op_identifier)),
                error: LanguageErrorType::Reference(name.to_string()),
              })?;
              if function.argument_count != arguments.len() {
                return Err(LanguageError {
                  location: Some(argument_pairs_location),
                  error: LanguageErrorType::ArgumentCountMismatch(
                    arguments.len(),
                    function.argument_count,
                  ),
                });
              }
              FunctionIdentifier::UserDefined(function.identifier)
            }
          };
          ExpressionOp::FunctionCall(op, arguments)
        }
        _ => unreachable!(),
      };
      Ok(Expression { op, location }) as Result<_, LanguageError>
    })
    .map_prefix(|op, rhs| {
      let location = Location::from(&op);

      let op = match op.as_rule() {
        Rule::neg => ExpressionOp::Neg(Box::new(rhs?)),
        Rule::invert => ExpressionOp::Invert(Box::new(rhs?)),
        _ => unreachable!(),
      };
      Ok(Expression { op, location })
    })
    .map_postfix(|lhs, op| {
      let location = Location::from(&op);
      let op = match op.as_rule() {
        Rule::index => {
          let index: Expression = parse_expression(
            execution_context.clone(),
            scope.clone(),
            op.into_inner(),
            functions,
          )?;
          ExpressionOp::Index(Box::new(lhs?), Box::new(index))
        }
        // Rule::fac => (1..(lhs?.try_into()? as i32) + 1).product(),
        _ => unreachable!(),
      };
      Ok(Expression { op, location })
    })
    .map_infix(|lhs, op, rhs| {
      let lhs = Box::new(lhs?);
      let rhs = Box::new(rhs?);
      let location = Location::from(&op);
      let op = match op.as_rule() {
        Rule::add => ExpressionOp::Add(lhs, rhs),
        Rule::sub => ExpressionOp::Sub(lhs, rhs),
        Rule::mul => ExpressionOp::Mul(lhs, rhs),
        Rule::div => ExpressionOp::Div(lhs, rhs),
        Rule::xor => ExpressionOp::Xor(lhs, rhs),
        Rule::bor => ExpressionOp::BinaryOr(lhs, rhs),
        Rule::band => ExpressionOp::BinaryAnd(lhs, rhs),
        Rule::shift_left => ExpressionOp::ShiftLeft(lhs, rhs),
        Rule::shift_right => ExpressionOp::ShiftRight(lhs, rhs),
        Rule::eq => ExpressionOp::Equal(lhs, rhs),
        Rule::neq => ExpressionOp::NotEqual(lhs, rhs),
        Rule::lt => ExpressionOp::LessThan(lhs, rhs),
        Rule::gt => ExpressionOp::GreaterThan(lhs, rhs),
        Rule::lteq => ExpressionOp::LessThanOrEqual(lhs, rhs),
        Rule::gteq => ExpressionOp::GreaterThanOrEqual(lhs, rhs),
        Rule::and => ExpressionOp::And(lhs, rhs),
        Rule::or => ExpressionOp::Or(lhs, rhs),
        Rule::modulo => ExpressionOp::Modulo(lhs, rhs),
        Rule::pow => ExpressionOp::Pow(lhs, rhs),
        _ => unreachable!(),
      };
      Ok(Expression { location, op })
    })
    .parse(pairs)
}

fn parse_statement(
  execution_context: Rc<Mutex<ExecutionContext>>,
  scope: String,
  pair: Pair<'_, Rule>,
  functions: &HashMap<String, FunctionPrototype>,
) -> Result<Statement, LanguageError> {
  // println!("Reading a rule {:?}", pair.as_rule());
  Ok(match pair.as_rule() {
    Rule::assignment_statement => {
      let mut pairs = pair.into_inner();
      let identifier = execution_context.lock().unwrap().register(VariableKey {
        name: pairs.next().unwrap().as_str().to_string(),
        scope: scope.clone(),
      });
      let expression = pairs.next().unwrap();
      let value = parse_expression(execution_context, scope, expression.into_inner(), functions)?;
      Statement::Assignment {
        variable: identifier,
        value,
      }
    }
    Rule::if_statement => Statement::If(parse_if_statement(
      execution_context,
      scope,
      pair,
      functions,
    )?),
    Rule::return_statement => {
      let mut pairs = pair.into_inner();
      let expression = pairs.next().unwrap();
      Statement::Return(parse_expression(
        execution_context,
        scope,
        expression.into_inner(),
        functions,
      )?)
    }
    _ => unreachable!(),
  })
}

fn parse_if_statement(
  execution_context: Rc<Mutex<ExecutionContext>>,
  scope: String,
  pair: Pair<'_, Rule>,
  functions: &HashMap<String, FunctionPrototype>,
) -> Result<IfStatement, LanguageError> {
  let mut pairs = pair.into_inner();
  let mut if_statement_if = pairs.next().unwrap().into_inner();
  let condition = if_statement_if.next().unwrap().into_inner();
  let if_block = parse_statement_block(
    execution_context.clone(),
    scope.clone(),
    if_statement_if.next().unwrap().into_inner(),
    functions,
  )?;
  // println!("Condition: {condition}");
  let condition = parse_expression(
    execution_context.clone(),
    scope.clone(),
    condition,
    functions,
  )?;
  Ok(IfStatement {
    condition,
    if_branch: if_block,
    else_branch: match pairs.next() {
      Some(if_statement_else) => {
        let mut if_statement_else = if_statement_else.into_inner();
        let next_pair = if_statement_else.peek().unwrap();
        match next_pair.as_rule() {
          // else if ...
          Rule::if_statement => ElseBranch::IfStatement(Box::new(parse_if_statement(
            execution_context.clone(),
            scope,
            if_statement_else.next().unwrap(),
            functions,
          )?)),
          // plain old else
          _ => ElseBranch::ElseStatement(parse_statement_block(
            execution_context,
            scope,
            if_statement_else.next().unwrap().into_inner(),
            functions,
          )?),
        }
      }
      None => ElseBranch::None,
    },
  })
}
