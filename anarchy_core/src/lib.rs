use lazy_static::lazy_static;
use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest::Parser;
use pest_derive::Parser;
// use std::collections::HashMap;
use rustc_hash::FxHashMap;
use std::fmt;

#[derive(Parser)]
#[grammar = "anarchy.pest"] // relative to src
struct AnarchyParser;

#[derive(Clone, Debug)]
pub enum Value {
    Number(f32),
    Tuple(Vec<Value>),
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

impl fmt::Display for LanguageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LanguageError::Type(expected_type, actual_value) => write!(
                f,
                "TypeError: Expected value of type {expected_type}, got: {actual_value}",
            ),
            LanguageError::Reference(identifier) => write!(
                f,
                "ReferenceError: Couldn't find identifier named {identifier}",
            ),
            LanguageError::Range(index, length) => write!(
                f,
                "RangeError: Index {index} out of bounds for tuple of length {length}"
            ),
        }
    }
}

impl TryFrom<Value> for f32 {
    type Error = LanguageError;
    fn try_from(value: Value) -> Result<f32, LanguageError> {
        match value {
            Value::Number(number) => Ok(number),
            value => Err(LanguageError::Type(ValueType::Number, value)),
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

impl TryFrom<Value> for Vec<Value> {
    type Error = LanguageError;
    fn try_from(value: Value) -> Result<Vec<Value>, LanguageError> {
        match value {
            Value::Tuple(tuple) => Ok(tuple),
            value => Err(LanguageError::Type(ValueType::Tuple, value)),
        }
    }
}
impl From<Vec<Value>> for Value {
    fn from(tuple: Vec<Value>) -> Value {
        Value::Tuple(tuple)
    }
}

#[derive(Debug, Clone)]
pub enum LanguageError {
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
            .op(Op::infix(Rule::mul, Assoc::Left) | Op::infix(Rule::div, Assoc::Left))
            .op(Op::prefix(Rule::neg))
            .op(Op::postfix(Rule::index))
    };
}

#[derive(Debug, Clone)]
pub struct ParsedLanguage(Vec<Statement>);

pub fn parse(code: &str) -> Result<ParsedLanguage, Box<pest::error::Error<Rule>>> {
    Ok(ParsedLanguage(parse_statement_block(
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
                context.set(variable.clone(), value);
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
        let condition = f32::try_from(self.condition.evaluate(context)?)?;
        if condition != 0.0 {
            execute_statement_block(context, &self.if_branch)?;
        } else {
            match &self.else_branch {
                ElseBranch::IfStatement(if_statement) => if_statement.execute(context)?,
                ElseBranch::ElseStatement(else_block) => {
                    execute_statement_block(context, else_block)?
                }
                ElseBranch::None => {}
            };
        }
        Ok(())
    }
}

impl Expression {
    fn evaluate(&self, context: &mut ExecutionContext) -> Result<Value, LanguageError> {
        Ok(match self {
            Expression::Reference(identifier) => context.get(identifier)?,
            Expression::NumberLiteral(number) => (*number).into(),
            Expression::TupleLiteral(expressions) => Value::Tuple(
                expressions
                    .iter()
                    .map(|expression| expression.evaluate(context))
                    .collect::<Result<Vec<Value>, _>>()?,
            ),
            Expression::Index(tuple, index) => {
                let index = f32::try_from(index.evaluate(context)?)? as usize;
                let tuple = Vec::<Value>::try_from(tuple.evaluate(context)?)?;
                tuple
                    .get(index)
                    .ok_or(LanguageError::Range(index, tuple.len()))?
                    .clone()
            }
            Expression::Add(lhs, rhs) => Value::from(
                f32::try_from(lhs.evaluate(context)?)? + f32::try_from(rhs.evaluate(context)?)?,
            ),
            Expression::Sub(lhs, rhs) => Value::from(
                f32::try_from(lhs.evaluate(context)?)? - f32::try_from(rhs.evaluate(context)?)?,
            ),
            Expression::Mul(lhs, rhs) => Value::from(
                f32::try_from(lhs.evaluate(context)?)? * f32::try_from(rhs.evaluate(context)?)?,
            ),
            Expression::Div(lhs, rhs) => Value::from(
                f32::try_from(lhs.evaluate(context)?)? / f32::try_from(rhs.evaluate(context)?)?,
            ),
            Expression::BinaryAnd(lhs, rhs) => Value::from(
                (f32::try_from(lhs.evaluate(context)?)? as u32
                    & f32::try_from(rhs.evaluate(context)?)? as u32) as f32,
            ),
            Expression::Xor(lhs, rhs) => Value::from(
                (f32::try_from(lhs.evaluate(context)?)? as u32
                    ^ f32::try_from(rhs.evaluate(context)?)? as u32) as f32,
            ),
            Expression::ShiftLeft(lhs, rhs) => Value::from(
                ((f32::try_from(lhs.evaluate(context)?)? as u32)
                    << (f32::try_from(rhs.evaluate(context)?)? as u32)) as f32,
            ),
            Expression::ShiftRight(lhs, rhs) => Value::from(
                ((f32::try_from(lhs.evaluate(context)?)? as u32)
                    >> (f32::try_from(rhs.evaluate(context)?)? as u32)) as f32,
            ),
            Expression::BinaryOr(lhs, rhs) => Value::from(
                (f32::try_from(lhs.evaluate(context)?)? as u32
                    | f32::try_from(rhs.evaluate(context)?)? as u32) as f32,
            ),
            Expression::GreaterThan(lhs, rhs) => Value::from(
                f32::try_from(lhs.evaluate(context)?)? > f32::try_from(rhs.evaluate(context)?)?,
            ),
            Expression::LessThan(lhs, rhs) => Value::from(
                f32::try_from(lhs.evaluate(context)?)? < f32::try_from(rhs.evaluate(context)?)?,
            ),
            Expression::GreaterThanOrEqual(lhs, rhs) => Value::from(
                f32::try_from(lhs.evaluate(context)?)? >= f32::try_from(rhs.evaluate(context)?)?,
            ),
            Expression::LessThanOrEqual(lhs, rhs) => Value::from(
                f32::try_from(lhs.evaluate(context)?)? <= f32::try_from(rhs.evaluate(context)?)?,
            ),
            Expression::Equal(lhs, rhs) => Value::from(
                f32::try_from(lhs.evaluate(context)?)? == f32::try_from(rhs.evaluate(context)?)?,
            ),
            Expression::NotEqual(lhs, rhs) => Value::from(
                f32::try_from(lhs.evaluate(context)?)? != f32::try_from(rhs.evaluate(context)?)?,
            ),
            Expression::Neg(value) => Value::from(-f32::try_from(value.evaluate(context)?)?),
            Expression::Invert(value) => {
                Value::from(if f32::try_from(value.evaluate(context)?)? == 0.0 {
                    1.0
                } else {
                    0.0
                })
            }
            Expression::And(lhs, rhs) => {
                Value::from(if f32::try_from(lhs.evaluate(context)?)? != 0.0 {
                    f32::try_from(rhs.evaluate(context)?)?
                } else {
                    0.0
                })
            }
            Expression::Or(lhs, rhs) => {
                let lhs = f32::try_from(lhs.evaluate(context)?)?;
                Value::from(if lhs != 0.0 {
                    lhs
                } else {
                    f32::try_from(rhs.evaluate(context)?)?
                })
            }
        })
    }
}

fn parse_statement_block(pairs: Pairs<Rule>) -> Vec<Statement> {
    pairs
        .map(|pair| parse_statement(pair.into_inner().next().unwrap()))
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    scope: FxHashMap<String, Value>,
}
impl fmt::Display for ExecutionContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut scope_iter = self.scope.iter().peekable();
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
    #[inline(always)]
    pub fn get(&self, identifier: &str) -> Result<Value, LanguageError> {
        self.scope
            .get(identifier)
            .cloned()
            .ok_or_else(|| LanguageError::Reference(identifier.to_string()))
    }
    #[inline(always)]
    pub fn set(&mut self, identifier: String, value: Value) {
        self.scope.insert(identifier, value);
    }
    #[inline(always)]
    pub fn reset(&mut self) {
        self.scope.clear();
    }
}

type Identifier = String;
#[derive(Debug, Clone)]
enum ElseBranch {
    IfStatement(Box<IfStatement>),
    ElseStatement(Vec<Statement>),
    None,
}
#[derive(Debug, Clone)]
enum Expression {
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

fn parse_expression(pairs: Pairs<Rule>) -> Expression {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::number_literal => {
                Expression::NumberLiteral(primary.as_str().parse::<f32>().unwrap())
            }
            Rule::tuple_literal => Expression::TupleLiteral(
                primary
                    .into_inner()
                    .map(|entry| parse_expression(entry.into_inner()))
                    .collect::<Vec<Expression>>(),
            ),
            Rule::identifier => Expression::Reference(primary.as_str().to_string()),
            Rule::expr => parse_expression(primary.into_inner()),
            _ => unreachable!(),
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::neg => Expression::Neg(Box::new(rhs)),
            Rule::invert => Expression::Invert(Box::new(rhs)),
            _ => unreachable!(),
        })
        .map_postfix(|lhs, op| match op.as_rule() {
            Rule::index => {
                let index: Expression = parse_expression(op.into_inner());
                Expression::Index(Box::new(lhs), Box::new(index))
            }
            // Rule::fac => (1..(lhs?.try_into()? as i32) + 1).product(),
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| {
            let lhs = Box::new(lhs);
            let rhs = Box::new(rhs);
            match op.as_rule() {
                Rule::add => Expression::Add(lhs, rhs),
                Rule::sub => Expression::Sub(lhs, rhs),
                Rule::mul => Expression::Mul(lhs, rhs),
                Rule::div => Expression::Div(lhs, rhs),
                Rule::xor => Expression::Xor(lhs, rhs),
                Rule::bor => Expression::BinaryOr(lhs, rhs),
                Rule::band => Expression::BinaryAnd(lhs, rhs),
                Rule::shift_left => Expression::ShiftLeft(lhs, rhs),
                Rule::shift_right => Expression::ShiftRight(lhs, rhs),
                Rule::eq => Expression::Equal(lhs, rhs),
                Rule::neq => Expression::NotEqual(lhs, rhs),
                Rule::lt => Expression::LessThan(lhs, rhs),
                Rule::gt => Expression::GreaterThan(lhs, rhs),
                Rule::lteq => Expression::LessThanOrEqual(lhs, rhs),
                Rule::gteq => Expression::GreaterThanOrEqual(lhs, rhs),
                Rule::and => Expression::And(lhs, rhs),
                Rule::or => Expression::Or(lhs, rhs),
                _ => unreachable!(),
            }
        })
        .parse(pairs)
}

fn parse_statement(pair: Pair<'_, Rule>) -> Statement {
    // println!("Reading a rule {:?}", pair.as_rule());
    match pair.as_rule() {
        Rule::assignment_statement => {
            let mut pairs = pair.into_inner();
            let identifier = pairs.next().unwrap().as_str();
            let expression = pairs.next().unwrap();
            let value = parse_expression(expression.into_inner());
            Statement::Assignment {
                variable: identifier.to_string(),
                value,
            }
        }
        Rule::if_statement => Statement::If(parse_if_statement(pair)),
        _ => unreachable!(),
    }
}

fn parse_if_statement(pair: Pair<'_, Rule>) -> IfStatement {
    let mut pairs = pair.into_inner();
    let mut if_statement_if = pairs.next().unwrap().into_inner();
    let condition = if_statement_if.next().unwrap().into_inner();
    let if_block = parse_statement_block(if_statement_if.next().unwrap().into_inner());
    // println!("Condition: {condition}");
    let condition = parse_expression(condition);
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
                        if_statement_else.next().unwrap(),
                    ))),
                    // plain old else
                    _ => ElseBranch::ElseStatement(parse_statement_block(
                        if_statement_else.next().unwrap().into_inner(),
                    )),
                }
            }
            None => ElseBranch::None,
        },
    }
}
