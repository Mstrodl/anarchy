use anarchy_core::{ExecutionContext, LanguageError, ParsedLanguage};
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    // The `console.log` is quite polymorphic, so we can bind it with multiple
    // signatures. Note that we need to use `js_name` to ensure we always call
    // `log` in JS.
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    // Multiple arguments too!
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_many(a: &str, b: &str);
}

#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();
}

thread_local! {
    static PARSED_LANGUAGE: Rc<Mutex<Option<ParsedLanguage>>> = Rc::new(Mutex::new(None));
}

#[wasm_bindgen]
pub fn parse(code: String) -> Result<(), JsError> {
    let parsed_language = match anarchy_core::parse(&code) {
        Ok(parsed_language) => parsed_language,
        Err(err) => {
            return Err(JsError::new(&format!("LanguageError: {err}")));
        }
    };
    PARSED_LANGUAGE.with(|language| {
        language.lock().unwrap().replace(parsed_language);
    });

    Ok(())
}

#[wasm_bindgen]
pub fn execute(
    image: &mut [u8],
    width: usize,
    height: usize,
    time: u32,
    random: f32,
) -> Result<(), JsError> {
    execute_inner(image, width, height, time, random)
        .map_err(|err| JsError::new(&format!("LanguageError: {err}")))
}
fn execute_inner(
    image: &mut [u8],
    width: usize,
    height: usize,
    time: u32,
    random: f32,
) -> Result<(), LanguageError> {
    PARSED_LANGUAGE.with(|language| {
        let parsed_language = language.lock().unwrap();
        let parsed_language = parsed_language.as_ref().unwrap();
        for y in 0..height {
            for x in 0..width {
                let mut context = ExecutionContext::default();
                context.set("x".to_string(), (x as f32).into());
                context.set("y".to_string(), (y as f32).into());
                context.set("time".to_string(), (time as f32).into());
                context.set("random".to_string(), random.into());

                anarchy_core::execute(&mut context, parsed_language)?;

                let base_position = width * y * 4 + x * 4;
                let r: f32 = context.get("r")?.try_into()?;
                let g: f32 = context.get("g")?.try_into()?;
                let b: f32 = context.get("b")?.try_into()?;
                image[base_position] = r as u8;
                image[base_position + 1] = g as u8;
                image[base_position + 2] = b as u8;
            }
        }
        Ok(())
    })
}
