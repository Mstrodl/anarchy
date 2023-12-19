use anarchy_core::{
  parse, ExecutionContext, LanguageError, ParsedLanguage, UntrackedValue, Value, VariableKey,
};
use std::rc::Rc;
use std::sync::Mutex;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() {
  let code = std::fs::read("./input.anarchy").unwrap();
  let code = String::from_utf8_lossy(&code);
  // let mut context = ExecutionContext::default();
  // execute(&mut context, pairs).unwrap();
  // println!("Executed program at ./input.anarchy Resulting state: {context}");
  //torture_test();
  // let code = include_str!("../../input.anarchy"); // r=time&255;g=time&255;b=time&255;".to_owned();
  let context = Rc::new(Mutex::new(ExecutionContext::default()));
  let parsed_language = parse(context.clone(), &code).unwrap();
  println!("Finished parsing!");
  let mut context = Rc::try_unwrap(context).unwrap().into_inner().unwrap();
  const HEIGHT: usize = 100;
  const WIDTH: usize = 100;
  let random = 0f32;
  let mut image = [0u8; WIDTH * HEIGHT * 4];

  context.set_runtime("x", Value::Number(0.0));
  context.set_runtime("y", Value::Number(0.0));
  context.set_runtime("time", Value::Number(0.0));
  context.set_runtime("random", Value::Number(0.0));
  anarchy_core::execute(&mut context, &parsed_language).unwrap();
  println!("After execution: {context}");

  let r_identifier = context.register(VariableKey {
    name: "r".to_string(),
    scope: "".to_string(),
  });
  let g_identifier = context.register(VariableKey {
    name: "g".to_string(),
    scope: "".to_string(),
  });
  let b_identifier = context.register(VariableKey {
    name: "b".to_string(),
    scope: "".to_string(),
  });
  let time_identifier = context.register(VariableKey {
    name: "time".to_string(),
    scope: "".to_string(),
  });
  let random_identifier = context.register(VariableKey {
    name: "random".to_string(),
    scope: "".to_string(),
  });
  let x_identifier = context.register(VariableKey {
    name: "x".to_string(),
    scope: "".to_string(),
  });
  let y_identifier = context.register(VariableKey {
    name: "y".to_string(),
    scope: "".to_string(),
  });

  for time in 0..500 {
    run_iteration(
      &parsed_language,
      &mut image,
      WIDTH,
      HEIGHT,
      time,
      random,
      IdentifierBundle {
        r_identifier,
        g_identifier,
        b_identifier,
        x_identifier,
        y_identifier,
        time_identifier,
        random_identifier,
      },
      &mut context,
    )
    .unwrap();
  }
}

struct IdentifierBundle {
  r_identifier: usize,
  g_identifier: usize,
  b_identifier: usize,
  x_identifier: usize,
  y_identifier: usize,
  time_identifier: usize,
  random_identifier: usize,
}

#[allow(clippy::too_many_arguments)]
fn run_iteration(
  parsed_language: &ParsedLanguage,
  image: &mut [u8],
  width: usize,
  height: usize,
  time: u32,
  random: f32,
  IdentifierBundle {
    r_identifier,
    g_identifier,
    b_identifier,
    x_identifier,
    y_identifier,
    time_identifier,
    random_identifier,
  }: IdentifierBundle,
  context: &mut ExecutionContext,
) -> Result<(), LanguageError> {
  let time_float: Value = (time as f32).into();
  let random_float: Value = random.into();
  for y in 0..height {
    let y_float: Value = (y as f32).into();
    for x in 0..width {
      context.reset();
      context.set(x_identifier, (x as f32).into());
      context.set(y_identifier, y_float.clone());
      context.set(time_identifier, time_float.clone());
      context.set(random_identifier, random_float.clone());

      anarchy_core::execute(context, parsed_language)?;

      let base_position = height * x * 4 + y * 4;
      println!("Seems legit {context}");
      let r: f32 = UntrackedValue(context.unattributed_get(r_identifier)?).try_into()?;
      let g: f32 = UntrackedValue(context.unattributed_get(g_identifier)?).try_into()?;
      let b: f32 = UntrackedValue(context.unattributed_get(b_identifier)?).try_into()?;
      image[base_position] = r as u8;
      image[base_position + 1] = g as u8;
      image[base_position + 2] = b as u8;
    }
  }
  Ok(())
}
