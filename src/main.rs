use lazy_static::lazy_static;
use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest::Parser;
use pest_derive::Parser;
use std::collections::{HashMap, VecDeque};
use std::fmt;

#[derive(Parser)]
#[grammar = "anarchy.pest"] // relative to src
struct AnarchyParser;

#[derive(Clone, Debug)]
enum Value {
    Number(f32),
    Tuple(Vec<Value>),
}

#[derive(Clone, Debug)]
enum ValueType {
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
            LanguageError::TypeError(expected_type, actual_value) => write!(
                f,
                "TypeError: Expected value of type {expected_type}, got: {actual_value}",
            ),
            LanguageError::ReferenceError(identifier) => write!(
                f,
                "ReferenceError: Couldn't find identifier named {identifier}",
            ),
        }
    }
}

impl TryFrom<Value> for f32 {
    type Error = LanguageError;
    fn try_from(value: Value) -> Result<f32, LanguageError> {
        match value {
            Value::Number(number) => Ok(number),
            value => Err(LanguageError::TypeError(ValueType::Number, value)),
        }
    }
}
impl From<f32> for Value {
    fn from(number: f32) -> Value {
        Value::Number(number)
    }
}

impl TryFrom<Value> for Vec<Value> {
    type Error = LanguageError;
    fn try_from(value: Value) -> Result<Vec<Value>, LanguageError> {
        match value {
            Value::Tuple(tuple) => Ok(tuple),
            value => Err(LanguageError::TypeError(ValueType::Tuple, value)),
        }
    }
}
impl From<Vec<Value>> for Value {
    fn from(tuple: Vec<Value>) -> Value {
        Value::Tuple(tuple)
    }
}

#[derive(Debug, Clone)]
enum LanguageError {
    TypeError(ValueType, Value),
    ReferenceError(String),
}

lazy_static! {
    pub static ref PRATT_PARSER: PrattParser<Rule> = {
        PrattParser::new()
            .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
            .op(Op::infix(Rule::mul, Assoc::Left) | Op::infix(Rule::div, Assoc::Left))
            .op(Op::infix(Rule::xor, Assoc::Left)
                | Op::infix(Rule::band, Assoc::Left)
                | Op::infix(Rule::shift_left, Assoc::Left)
                | Op::infix(Rule::shift_right, Assoc::Left)
                | Op::infix(Rule::bor, Assoc::Left))
            .op(Op::infix(Rule::eq, Assoc::Left)
                | Op::infix(Rule::lt, Assoc::Left)
                | Op::infix(Rule::gt, Assoc::Left)
                | Op::infix(Rule::gteq, Assoc::Left)
                | Op::infix(Rule::lteq, Assoc::Left)
                | Op::infix(Rule::neq, Assoc::Left))
            .op(Op::prefix(Rule::neg))
            .op(Op::postfix(Rule::index))
    };
}

fn main() {
    let code = std::fs::read("./input.anarchy").unwrap();
    let code = String::from_utf8_lossy(&code);
    let pairs = AnarchyParser::parse(Rule::program, &code)
        .unwrap()
        .next()
        .unwrap()
        .into_inner()
        .next()
        .unwrap()
        .into_inner();
    let mut context = ExecutionContext::default();
    execute_statement_block(&mut context, pairs).unwrap();
}

fn execute_statement_block(
    context: &mut ExecutionContext,
    pairs: Pairs<Rule>,
) -> Result<(), LanguageError> {
    for pair in pairs {
        let pair = pair.into_inner().next().unwrap();
        println!("Found a pair: {pair}");
        execute_statement(context, pair).unwrap();
        println!("After execution: {context:?}");
        println!();
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
struct ExecutionContext {
    scope: HashMap<String, Value>,
}
impl ExecutionContext {
    fn get(&self, identifier: &str) -> Result<Value, LanguageError> {
        self.scope
            .get(identifier)
            .cloned()
            .ok_or_else(|| LanguageError::ReferenceError(identifier.to_string()))
    }
    fn set(&mut self, identifier: String, value: Value) {
        self.scope.insert(identifier, value);
    }
}

fn evaluate_expression(
    context: &ExecutionContext,
    pairs: Pairs<Rule>,
) -> Result<Value, LanguageError> {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::number_literal => Ok(primary.as_str().parse::<f32>().unwrap().into()),
            Rule::tuple_literal => Ok(Value::Tuple(
                primary
                    .into_inner()
                    .map(|entry| evaluate_expression(context, entry.into_inner()))
                    .collect::<Result<Vec<Value>, LanguageError>>()?,
            )),
            Rule::identifier => context.get(primary.as_str()),
            Rule::expr => evaluate_expression(context, primary.into_inner()),
            _ => unreachable!(),
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::neg => {
                let number: f32 = (rhs?).try_into()?;
                Ok(Value::Number(-number))
            }
            _ => unreachable!(),
        })
        .map_postfix(|lhs, op| match op.as_rule() {
            Rule::index => {
                let index: f32 = evaluate_expression(context, op.into_inner())?.try_into()?;
                let index = index.floor();
                let tuple: Vec<Value> = lhs?.try_into()?;
                Ok(tuple[index as usize].clone())
            }
            // Rule::fac => (1..(lhs?.try_into()? as i32) + 1).product(),
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| {
            let lhs: f32 = lhs?.try_into()?;
            let rhs: f32 = rhs?.try_into()?;
            Ok(match op.as_rule() {
                Rule::add => lhs + rhs,
                Rule::sub => lhs - rhs,
                Rule::mul => lhs * rhs,
                Rule::div => lhs / rhs,
                Rule::xor => ((lhs as i32) ^ (rhs as i32)) as f32,
                Rule::bor => ((lhs as i32) | (rhs as i32)) as f32,
                Rule::band => ((lhs as i32) & (rhs as i32)) as f32,
                Rule::shift_left => ((lhs as i32) << (rhs as i32)) as f32,
                Rule::shift_right => ((lhs as i32) >> (rhs as i32)) as f32,
                Rule::eq | Rule::lt | Rule::gt | Rule::gteq | Rule::lteq => {
                    let boolean = match op.as_rule() {
                        Rule::eq => lhs == rhs,
                        Rule::neq => lhs != rhs,
                        Rule::lt => lhs > rhs,
                        Rule::gt => lhs < rhs,
                        Rule::lteq => lhs >= rhs,
                        Rule::gteq => lhs <= rhs,
                        _ => unreachable!(),
                    };
                    if boolean {
                        1.0
                    } else {
                        0.0
                    }
                }
                _ => unreachable!(),
            }
            .into())
        })
        .parse(pairs)
}

fn execute_statement(
    context: &mut ExecutionContext,
    pair: Pair<'_, Rule>,
) -> Result<(), LanguageError> {
    println!("Reading a rule {:?}", pair.as_rule());
    match pair.as_rule() {
        Rule::assignment_statement => {
            let mut pairs = pair.into_inner();
            let identifier = pairs.next().unwrap().as_str();
            let expression = pairs.next().unwrap();
            let value = evaluate_expression(context, expression.into_inner())?;
            println!("Assignment {pairs} ({identifier}={value})");
            context.set(identifier.to_string(), value);
        }
        Rule::if_statement => {
            let mut pairs = pair.into_inner();
            let mut if_statement_if = pairs.next().unwrap().into_inner();
            let condition = if_statement_if.next().unwrap().into_inner();
            let if_block = if_statement_if.next().unwrap().into_inner();
            println!("Condition: {condition}");
            let condition_value = evaluate_expression(context, condition)?;
            let condition_value: f32 = condition_value.try_into()?;
            if condition_value != 0.0 {
                execute_statement_block(context, if_block)?;
            } else if let Some(if_statement_else) = pairs.next() {
                let mut if_statement_else = if_statement_else.into_inner();
                let next_pair = if_statement_else.peek().unwrap();
                match next_pair.as_rule() {
                    // else if ...
                    Rule::if_statement => {
                        execute_statement(context, if_statement_else.next().unwrap())?
                    }
                    // plain old else
                    _ => {
                        execute_statement_block(
                            context,
                            if_statement_else.next().unwrap().into_inner(),
                        )?;
                    }
                }
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
