use anarchy_core::pest::error::LineColLocation;
use anarchy_core::{ExecutionContext, LanguageError, Location, ParsedLanguage, UntrackedValue};
use serde::Serialize;
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

struct ParsedLanguageBundle {
    execution_context: ExecutionContext,
    parsed_language: ParsedLanguage,
    x_identifier: usize,
    y_identifier: usize,
    time_identifier: usize,
    random_identifier: usize,
    r_identifier: usize,
    g_identifier: usize,
    b_identifier: usize,
}

thread_local! {
    static PARSED_LANGUAGE: Rc<Mutex<Option<ParsedLanguageBundle>>> = Rc::new(Mutex::new(None));
}

#[derive(Serialize, Debug, Clone)]
enum ErrorLocation {
    Pos((u32, u32)),
    Span((u32, u32), (u32, u32)),
    None,
}
#[derive(Serialize, Debug, Clone)]
enum ErrorType {
    Runtime,
    Parser,
}
#[derive(Serialize, Debug, Clone)]
struct WebError {
    location: ErrorLocation,
    message: String,
    error_type: ErrorType,
}

#[wasm_bindgen]
pub fn parse(code: String) -> Result<(), JsValue> {
    let context = Rc::new(Mutex::new(ExecutionContext::default()));
    let parsed_language = match anarchy_core::parse(context.clone(), &code) {
        Ok(parsed_language) => parsed_language,
        Err(err) => {
            return Err(serde_wasm_bindgen::to_value(&WebError {
                location: match err.line_col {
                    LineColLocation::Pos((line, col)) => {
                        ErrorLocation::Pos((line as u32, col as u32))
                    }
                    LineColLocation::Span((start_line, start_col), (end_line, end_col)) => {
                        ErrorLocation::Span(
                            (start_line as u32, start_col as u32),
                            (end_line as u32, end_col as u32),
                        )
                    }
                },
                message: err.variant.to_string(),
                error_type: ErrorType::Parser,
            })
            .unwrap());
        }
    };
    let mut context = Rc::try_unwrap(context).unwrap().into_inner().unwrap();
    PARSED_LANGUAGE.with(|language| {
        language.lock().unwrap().replace(ParsedLanguageBundle {
            x_identifier: context.register("x"),
            y_identifier: context.register("y"),
            r_identifier: context.register("r"),
            g_identifier: context.register("g"),
            b_identifier: context.register("b"),
            time_identifier: context.register("time"),
            random_identifier: context.register("random"),
            execution_context: context,
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
) -> Result<(), JsValue> {
    execute_inner(image, width, height, time, random).map_err(|err| {
        serde_wasm_bindgen::to_value(&WebError {
            location: match err.location {
                Some(Location {
                    start_line,
                    start_column,
                    end_line,
                    end_column,
                }) => ErrorLocation::Span(
                    (start_line as u32, start_column as u32),
                    (end_line as u32, end_column as u32),
                ),
                None => ErrorLocation::None,
            },
            message: err.error.to_string(),
            error_type: ErrorType::Runtime,
        })
        .unwrap()
    })
}
fn execute_inner(
    image: &mut [u8],
    width: usize,
    height: usize,
    time: u32,
    random: f32,
) -> Result<(), LanguageError> {
    PARSED_LANGUAGE.with(|language| {
        let mut parsed_language = language.lock().unwrap();
        let parsed_language = parsed_language.as_mut().unwrap();
        for y in 0..height {
            for x in 0..width {
                parsed_language
                    .execution_context
                    .set(parsed_language.x_identifier, (x as f32).into());
                parsed_language
                    .execution_context
                    .set(parsed_language.y_identifier, (y as f32).into());
                parsed_language
                    .execution_context
                    .set(parsed_language.time_identifier, (time as f32).into());
                parsed_language
                    .execution_context
                    .set(parsed_language.random_identifier, random.into());

                anarchy_core::execute(
                    &mut parsed_language.execution_context,
                    &parsed_language.parsed_language,
                )?;

                let base_position = width * y * 4 + x * 4;
                let r: f32 = UntrackedValue(
                    parsed_language
                        .execution_context
                        .unattributed_get(parsed_language.r_identifier)?,
                )
                .try_into()?;
                let g: f32 = UntrackedValue(
                    parsed_language
                        .execution_context
                        .unattributed_get(parsed_language.g_identifier)?,
                )
                .try_into()?;
                let b: f32 = UntrackedValue(
                    parsed_language
                        .execution_context
                        .unattributed_get(parsed_language.b_identifier)?,
                )
                .try_into()?;
                image[base_position] = r as u8;
                image[base_position + 1] = g as u8;
                image[base_position + 2] = b as u8;
            }
        }
        Ok(())
    })
}
