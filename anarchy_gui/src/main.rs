use anarchy_core::{parse, ExecutionContext, LanguageError, ParsedLanguage, UntrackedValue, Value};
use ringbuf::{HeapRb, Rb};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use winit::dpi::LogicalSize;
use winit::dpi::Size;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::window::WindowBuilder;

const HEIGHT: usize = 100;
const WIDTH: usize = 100;

struct FrameMessage {
    buffer: [u32; HEIGHT * WIDTH],
    time: Instant,
}

fn main() {
    let code = std::fs::read_to_string("./input.anarchy").unwrap();
    let event_loop: EventLoop<FrameMessage> = EventLoopBuilder::with_user_event().build().unwrap();
    let window = Rc::new(
        WindowBuilder::new()
            .with_inner_size(Size::Logical(LogicalSize::new(HEIGHT as f64, WIDTH as f64)))
            .build(&event_loop)
            .unwrap(),
    );
    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
    surface
        .resize(
            NonZeroU32::new(WIDTH as u32).unwrap(),
            NonZeroU32::new(HEIGHT as u32).unwrap(),
        )
        .unwrap();

    let context = Rc::new(Mutex::new(ExecutionContext::default()));
    let parsed_language = parse(context.clone(), &code).unwrap();
    println!("Finished parsing!");
    let mut context = Rc::try_unwrap(context).unwrap().into_inner().unwrap();
    let r_identifier = context.register("r");
    let g_identifier = context.register("g");
    let b_identifier = context.register("b");
    let time_identifier = context.register("time");
    let random_identifier = context.register("random");
    let x_identifier = context.register("x");
    let y_identifier = context.register("y");
    let random: f32 = rand::random();
    let latest_queued_time = Arc::new(Mutex::new(Instant::now()));
    let last_render_durations = Arc::new(RwLock::new(HeapRb::<Duration>::new(16)));

    let (frame_tx, frame_rx) = std::sync::mpsc::channel();
    for _ in 0..8 {
        let scope_locations = context.export_scope_locations();
        let frame_tx = frame_tx.clone();
        let parsed_language = parsed_language.clone();
        let last_render_durations = Arc::clone(&last_render_durations);
        let latest_queued_time = Arc::clone(&latest_queued_time);
        std::thread::spawn(move || {
            let random = Value::Number(random);
            let mut context = ExecutionContext::new_with_scope_locations(scope_locations);
            loop {
                println!("Hi?");
                let mut message = FrameMessage {
                    buffer: [0u32; WIDTH * HEIGHT],
                    time: {
                        let mut latest_queued_time = latest_queued_time.lock().unwrap();

                        let avg_render_time = {
                            let last_render_durations = last_render_durations.read().unwrap();
                            let length = last_render_durations.len() as u64;
                            let mut average_ms = 0 as u64;
                            for frame_time in last_render_durations.iter() {
                                average_ms += (frame_time.as_millis() as u64) / length;
                            }
                            Duration::from_millis(average_ms)
                        };
                        *latest_queued_time += avg_render_time;
                        *latest_queued_time
                    },
                };
                let time = Value::Number((message.time).elapsed().as_millis() as f32);

                for index in 0..HEIGHT * WIDTH {
                    let x = index / HEIGHT;
                    let y = index % HEIGHT;
                    context.reset();
                    context.set(x_identifier, Value::Number(x as f32));
                    context.set(y_identifier, Value::Number(y as f32));
                    context.set(time_identifier, time.clone());
                    context.set(random_identifier, random.clone());
                    anarchy_core::execute(&mut context, &parsed_language).unwrap();
                    let red: f32 = UntrackedValue(context.unattributed_get(r_identifier).unwrap())
                        .try_into()
                        .unwrap();
                    let green: f32 =
                        UntrackedValue(context.unattributed_get(g_identifier).unwrap())
                            .try_into()
                            .unwrap();
                    let blue: f32 = UntrackedValue(context.unattributed_get(b_identifier).unwrap())
                        .try_into()
                        .unwrap();
                    message.buffer[index as usize] =
                        blue as u32 | ((green as u32) << 8) | ((red as u32) << 16);
                }
                println!("Sending...");
                frame_tx.send(message).unwrap();
                println!("Sent!");
            }
        });
    }

    {
        let event_loop = event_loop.create_proxy();
        std::thread::spawn(move || loop {
            let frame = frame_rx.recv().unwrap();
            let mut frame_queue = Vec::new();
            frame_queue.push(frame);
            while let Ok(frame) = frame_rx.recv() {
                frame_queue.push(frame);
            }

            frame_queue
        });
    }

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == window.id() => {
                    elwt.exit();
                }
                _ => {}
            }
        })
        .unwrap();
}
