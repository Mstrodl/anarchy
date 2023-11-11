use anarchy_core::{execute, parse, ExecutionContext, LanguageError, ParsedLanguage, Value};

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() {
    // let code = std::fs::read("./input.anarchy").unwrap();
    // let code = String::from_utf8_lossy(&code);
    // let pairs = parse(&code).unwrap();
    // let mut context = ExecutionContext::default();
    // execute(&mut context, pairs).unwrap();
    // println!("Executed program at ./input.anarchy Resulting state: {context}");
    //torture_test();
    let code = include_str!("../../input.anarchy"); // r=time&255;g=time&255;b=time&255;".to_owned();
    let parsed_language = parse(&code).unwrap();
    const HEIGHT: usize = 100;
    const WIDTH: usize = 100;
    let random = 0f32;
    let mut image = [0u8; WIDTH * HEIGHT * 4];
    for time in 0..50 {
        run_iteration(&parsed_language, &mut image, WIDTH, HEIGHT, time, random).unwrap();
    }
}

fn run_iteration(
    parsed_language: &ParsedLanguage<'_>,
    image: &mut [u8],
    width: usize,
    height: usize,
    time: u32,
    random: f32,
) -> Result<(), LanguageError> {
    let mut context = ExecutionContext::default();
    let time_float: Value = (time as f32).into();
    let random_float: Value = random.into();
    for y in 0..height {
        let y_float: Value = (y as f32).into();
        for x in 0..width {
            context.reset();
            context.set("x".to_string(), (x as f32).into());
            context.set("y".to_string(), y_float.clone());
            context.set("time".to_string(), time_float.clone());
            context.set("random".to_string(), random_float.clone());

            anarchy_core::execute(&mut context, parsed_language.clone())?;

            let base_position = height * x * 4 + y * 4;
            let r: f32 = context.get("r")?.try_into()?;
            let g: f32 = context.get("g")?.try_into()?;
            let b: f32 = context.get("b")?.try_into()?;
            image[base_position] = r as u8;
            image[base_position + 1] = g as u8;
            image[base_position + 2] = b as u8;
        }
    }
    Ok(())
}
