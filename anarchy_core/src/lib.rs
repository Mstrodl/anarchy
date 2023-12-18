use lazy_static::lazy_static;
pub use pest;
use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest::Parser;
use pest_derive::Parser;
// use std::collections::HashMap;
use bimap::BiHashMap;
use std::fmt;
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
pub struct ParsedLanguage(Vec<Statement>);

pub fn parse(
  execution_context: Rc<Mutex<ExecutionContext>>,
  code: &str,
) -> Result<ParsedLanguage, Box<pest::error::Error<Rule>>> {
  Ok(ParsedLanguage(parse_statement_block(
    execution_context,
    AnarchyParser::parse(Rule::program, code)
      .map_err(Box::new)?
      .next()
      .unwrap()
      .into_inner()
      .next()
      .unwrap()
      .into_inner(),
  )))
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
  ParsedLanguage(pairs): &ParsedLanguage,
) -> Result<(), LanguageError> {
  execute_statement_block(context, pairs)
}

fn execute_statement_block(
  context: &mut ExecutionContext,
  statements: &Vec<Statement>,
) -> Result<(), LanguageError> {
  for statement in statements {
    statement.execute(context)?;
  }
  Ok(())
}

impl Statement {
  fn execute(&self, context: &mut ExecutionContext) -> Result<(), LanguageError> {
    match self {
      Statement::Assignment { variable, value } => {
        let value = value.evaluate(context)?;
        context.set(*variable, value);
      }
      Statement::If(if_statement) => {
        if_statement.execute(context)?;
      }
    };
    Ok(())
  }
}

impl IfStatement {
  fn execute(&self, context: &mut ExecutionContext) -> Result<(), LanguageError> {
    let condition = f32::try_from(TrackedValue(
      self.condition.evaluate(context)?,
      &self.condition.location,
    ))?;
    if condition != 0.0 {
      execute_statement_block(context, &self.if_branch)?;
    } else {
      match &self.else_branch {
        ElseBranch::IfStatement(if_statement) => if_statement.execute(context)?,
        ElseBranch::ElseStatement(else_block) => execute_statement_block(context, else_block)?,
        ElseBranch::None => {}
      };
    }
    Ok(())
  }
}

#[derive(Debug, Clone)]
enum Function {
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
}

impl Expression {
  fn evaluate(&self, context: &mut ExecutionContext) -> Result<Value, LanguageError> {
    Ok(match &self.op {
      ExpressionOp::Reference(identifier) => context.get(*identifier, &self.location)?,
      ExpressionOp::FunctionCall(function, value) => {
        let result = match function {
          Function::Len => {
            let tracked_value = TrackedValue(value.evaluate(context)?, &value.location);
            let value: Rc<Vec<Value>> = <Rc<Vec<Value>>>::try_from(&tracked_value)?;
            value.len() as f32
          }
          function => {
            let value = f32::try_from(TrackedValue(value.evaluate(context)?, &value.location))?;
            match function {
              Function::Sin => value.sin(),
              Function::Cos => value.cos(),
              Function::Tan => value.tan(),
              Function::Asin => value.asin(),
              Function::Acos => value.acos(),
              Function::Atan => value.atan(),
              Function::Abs => value.abs(),
              Function::Sqrt => value.sqrt(),
              Function::Log => value.log(2.0),
              Function::Len => unreachable!(),
            }
          }
        };
        Value::from(result)
      }
      ExpressionOp::NumberLiteral(number) => (*number).into(),
      ExpressionOp::TupleLiteral(expressions) => Value::Tuple(Rc::new(
        expressions
          .iter()
          .map(|expression| expression.evaluate(context))
          .collect::<Result<Vec<Value>, _>>()?,
      )),
      ExpressionOp::Index(tuple, index) => {
        let index_num =
          f32::try_from(TrackedValue(index.evaluate(context)?, &index.location))? as usize;
        let tuple =
          <Rc<Vec<Value>>>::try_from(&TrackedValue(tuple.evaluate(context)?, &tuple.location))?;
        tuple
          .get(index_num)
          .ok_or_else(|| LanguageError {
            error: LanguageErrorType::Range(index_num, tuple.len()),
            location: Some(index.location.clone()),
          })?
          .clone()
      }
      ExpressionOp::Pow(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?.powf(f32::try_from(
          TrackedValue(rhs.evaluate(context)?, &rhs.location),
        )?),
      ),
      ExpressionOp::Modulo(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?
          % f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?,
      ),
      ExpressionOp::Add(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?
          + f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?,
      ),
      ExpressionOp::Sub(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?
          - f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?,
      ),
      ExpressionOp::Mul(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?
          * f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?,
      ),
      ExpressionOp::Div(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?
          / f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?,
      ),
      ExpressionOp::BinaryAnd(lhs, rhs) => Value::from(
        (f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))? as u32
          & f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))? as u32)
          as f32,
      ),
      ExpressionOp::Xor(lhs, rhs) => Value::from(
        (f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))? as u32
          ^ f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))? as u32)
          as f32,
      ),
      ExpressionOp::ShiftLeft(lhs, rhs) => Value::from(
        ((f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))? as u32)
          << (f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))? as u32))
          as f32,
      ),
      ExpressionOp::ShiftRight(lhs, rhs) => Value::from(
        ((f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))? as u32)
          >> (f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))? as u32))
          as f32,
      ),
      ExpressionOp::BinaryOr(lhs, rhs) => Value::from(
        (f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))? as u32
          | f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))? as u32)
          as f32,
      ),
      ExpressionOp::GreaterThan(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?
          > f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?,
      ),
      ExpressionOp::LessThan(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?
          < f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?,
      ),
      ExpressionOp::GreaterThanOrEqual(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?
          >= f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?,
      ),
      ExpressionOp::LessThanOrEqual(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?
          <= f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?,
      ),
      ExpressionOp::Equal(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?
          == f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?,
      ),
      ExpressionOp::NotEqual(lhs, rhs) => Value::from(
        f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?
          != f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?,
      ),
      ExpressionOp::Neg(value) => Value::from(-f32::try_from(TrackedValue(
        value.evaluate(context)?,
        &value.location,
      ))?),
      ExpressionOp::Invert(value) => Value::from(
        if f32::try_from(TrackedValue(value.evaluate(context)?, &value.location))? == 0.0 {
          1.0
        } else {
          0.0
        },
      ),
      ExpressionOp::And(lhs, rhs) => Value::from(
        if f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))? != 0.0 {
          f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?
        } else {
          0.0
        },
      ),
      ExpressionOp::Or(lhs, rhs) => {
        let lhs = f32::try_from(TrackedValue(lhs.evaluate(context)?, &lhs.location))?;
        Value::from(if lhs != 0.0 {
          lhs
        } else {
          f32::try_from(TrackedValue(rhs.evaluate(context)?, &rhs.location))?
        })
      }
    })
  }
}

fn parse_statement_block(
  execution_context: Rc<Mutex<ExecutionContext>>,
  pairs: Pairs<Rule>,
) -> Vec<Statement> {
  pairs
    .filter(|pair| pair.as_rule() == Rule::statement)
    .map(|pair| parse_statement(execution_context.clone(), pair.into_inner().next().unwrap()))
    .collect()
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionContextLUT {
  scope_locations: BiHashMap<String, usize>,
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
  pub fn register(&mut self, name: &str) -> Identifier {
    match self.scope_locations.scope_locations.get_by_left(name) {
      Some(index) => *index,
      None => {
        let index = self.scope.len();
        self.scope.push(None);
        self
          .scope_locations
          .scope_locations
          .insert(name.to_string(), index);
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
    let index = self.register(identifier);
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
  FunctionCall(Function, Box<Expression>),
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
}

fn parse_expression(
  execution_context: Rc<Mutex<ExecutionContext>>,
  pairs: Pairs<Rule>,
) -> Expression {
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
            .map(|entry| parse_expression(execution_context.clone(), entry.into_inner()))
            .collect::<Vec<Expression>>(),
        ),
        Rule::identifier => {
          ExpressionOp::Reference(execution_context.lock().unwrap().register(primary.as_str()))
        }
        Rule::expr => parse_expression(execution_context, primary.into_inner()).op,
        Rule::function_call => {
          let mut pairs = primary.into_inner();
          let op = match pairs.next().unwrap().as_str() {
            "sin" => Function::Sin,
            "cos" => Function::Cos,
            "tan" => Function::Tan,
            "asin" => Function::Asin,
            "acos" => Function::Acos,
            "atan" => Function::Atan,
            "abs" => Function::Abs,
            "sqrt" => Function::Sqrt,
            "log" => Function::Log,
            "len" => Function::Len,
            _ => unreachable!(),
          };
          ExpressionOp::FunctionCall(
            op,
            Box::new(parse_expression(
              execution_context,
              pairs.next().unwrap().into_inner(),
            )),
          )
        }
        _ => unreachable!(),
      };
      Expression { op, location }
    })
    .map_prefix(|op, rhs| {
      let location = Location::from(&op);

      let op = match op.as_rule() {
        Rule::neg => ExpressionOp::Neg(Box::new(rhs)),
        Rule::invert => ExpressionOp::Invert(Box::new(rhs)),
        _ => unreachable!(),
      };
      Expression { op, location }
    })
    .map_postfix(|lhs, op| {
      let location = Location::from(&op);
      let op = match op.as_rule() {
        Rule::index => {
          let index: Expression = parse_expression(execution_context.clone(), op.into_inner());
          ExpressionOp::Index(Box::new(lhs), Box::new(index))
        }
        // Rule::fac => (1..(lhs?.try_into()? as i32) + 1).product(),
        _ => unreachable!(),
      };
      Expression { op, location }
    })
    .map_infix(|lhs, op, rhs| {
      let lhs = Box::new(lhs);
      let rhs = Box::new(rhs);
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
      Expression { location, op }
    })
    .parse(pairs)
}

fn parse_statement(
  execution_context: Rc<Mutex<ExecutionContext>>,
  pair: Pair<'_, Rule>,
) -> Statement {
  // println!("Reading a rule {:?}", pair.as_rule());
  match pair.as_rule() {
    Rule::assignment_statement => {
      let mut pairs = pair.into_inner();
      let identifier = execution_context
        .lock()
        .unwrap()
        .register(pairs.next().unwrap().as_str());
      let expression = pairs.next().unwrap();
      let value = parse_expression(execution_context, expression.into_inner());
      Statement::Assignment {
        variable: identifier,
        value,
      }
    }
    Rule::if_statement => Statement::If(parse_if_statement(execution_context, pair)),
    _ => unreachable!(),
  }
}

fn parse_if_statement(
  execution_context: Rc<Mutex<ExecutionContext>>,
  pair: Pair<'_, Rule>,
) -> IfStatement {
  let mut pairs = pair.into_inner();
  let mut if_statement_if = pairs.next().unwrap().into_inner();
  let condition = if_statement_if.next().unwrap().into_inner();
  let if_block = parse_statement_block(
    execution_context.clone(),
    if_statement_if.next().unwrap().into_inner(),
  );
  // println!("Condition: {condition}");
  let condition = parse_expression(execution_context.clone(), condition);
  IfStatement {
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
            if_statement_else.next().unwrap(),
          ))),
          // plain old else
          _ => ElseBranch::ElseStatement(parse_statement_block(
            execution_context,
            if_statement_else.next().unwrap().into_inner(),
          )),
        }
      }
      None => ElseBranch::None,
    },
  }
}
