use anarchy_core::{ExecutionContext, LanguageError, ParsedLanguage};
use std::cell::UnsafeCell;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::{prelude::*, Clamped};

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
    static PARSED_LANGUAGE: Rc<Mutex<Option<ParsedLanguageJail>>> = Rc::new(Mutex::new(None));
}

struct ParsedLanguageJail {
    parsed_language: ParsedLanguage<'static>,
    code: Box<UnsafeCell<String>>,
}

impl Drop for ParsedLanguageJail {
    fn drop(&mut self) {
        unsafe { std::ptr::drop_in_place(self.code.get()) }
    }
}

#[wasm_bindgen]
pub fn parse(code: String) -> Result<(), JsError> {
    let mut code = Box::new(UnsafeCell::new(code));
    let code_string = code.get_mut();
    let code_string = Box::leak(unsafe { Box::from_raw(code_string) });

    let parsed_language = match anarchy_core::parse(code_string) {
        Ok(parsed_language) => parsed_language,
        Err(err) => {
            unsafe { std::ptr::drop_in_place(code.get()) };
            return Err(JsError::new(&format!("LanguageError: {err}")));
        }
    };
    PARSED_LANGUAGE.with(|language| {
        language.lock().unwrap().replace(ParsedLanguageJail {
            code,
            parsed_language,
        });
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
    let parsed_language = PARSED_LANGUAGE.with(|language| {
        language
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .parsed_language
            .clone()
    });
    for y in 0..height {
        for x in 0..width {
            let mut context = ExecutionContext::default();
            context.set("x".to_string(), (x as f32).into());
            context.set("y".to_string(), (y as f32).into());
            context.set("time".to_string(), (time as f32).into());
            context.set("random".to_string(), random.into());

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
