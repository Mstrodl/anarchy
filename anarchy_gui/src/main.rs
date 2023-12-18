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

const HEIGHT: usize = 200;
const WIDTH: usize = 200;

#[derive(Debug, Clone)]
struct FrameMessage {
  buffer: Vec<u32>,
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
  let latest_drawn_time = Arc::new(RwLock::new(Instant::now()));
  let latest_queued_time = Arc::new(Mutex::new(Instant::now()));
  let start_time = Instant::now();

  let (frame_tx, frame_rx) = std::sync::mpsc::channel();

  const WORKER_COUNT: u32 = 16;

  for _ in 0..WORKER_COUNT {
    let scope_locations = context.export_scope_locations();
    let frame_tx = frame_tx.clone();
    let parsed_language = parsed_language.clone();
    let latest_queued_time = Arc::clone(&latest_queued_time);
    let latest_drawn_time = Arc::clone(&latest_drawn_time);
    let start_time = start_time.clone();
    std::thread::spawn(move || {
      let mut last_render_durations = HeapRb::<Duration>::new(16);
      let random = Value::Number(random);
      let mut context = ExecutionContext::new_with_scope_locations(scope_locations);
      loop {
        let mut message = FrameMessage {
          buffer: Vec::with_capacity(HEIGHT * WIDTH),
          time: {
            let mut latest_queued_time = latest_queued_time.lock().unwrap();
            let avg_render_time = {
              let length = last_render_durations.len() as u64;
              let mut average_ms = 0 as u64;
              for frame_time in last_render_durations.iter() {
                println!("Avg entry: {frame_time:?}");
                average_ms += frame_time.as_millis() as u64;
              }
              if length == 0 {
                average_ms = 100;
              } else {
                average_ms /= length;
              }
              Duration::from_millis(average_ms)
            };
            println!("Current avg render time is {avg_render_time:?}");

            let our_time = *latest_queued_time + avg_render_time / WORKER_COUNT;
            let latest_drawn_time = latest_drawn_time.read().unwrap();
            let our_time = if *latest_drawn_time > our_time {
              // We're falling behind, catch up:
              println!("Falling behind, catching up!");
              *latest_drawn_time
            } else {
              our_time
            };
            *latest_queued_time = our_time;
            our_time
          },
        };
        message.buffer.resize(HEIGHT * WIDTH, 0u32);
        let time = Value::Number((message.time - start_time).as_millis() as f32);

        let render_start = Instant::now();
        for index in 0..HEIGHT * WIDTH {
          let x = index % WIDTH;
          let y = index / WIDTH;
          context.reset();
          context.set(x_identifier, Value::Number(x as f32));
          context.set(y_identifier, Value::Number(y as f32));
          context.set(time_identifier, time.clone());
          context.set(random_identifier, random.clone());
          anarchy_core::execute(&mut context, &parsed_language).unwrap();
          let red: f32 = UntrackedValue(context.unattributed_get(r_identifier).unwrap())
            .try_into()
            .unwrap();
          let green: f32 = UntrackedValue(context.unattributed_get(g_identifier).unwrap())
            .try_into()
            .unwrap();
          let blue: f32 = UntrackedValue(context.unattributed_get(b_identifier).unwrap())
            .try_into()
            .unwrap();
          message.buffer[index as usize] =
            ((blue as u32) & 0xff) | (((green as u32) & 0xff) << 8) | (((red as u32) & 0xff) << 16);
        }
        last_render_durations.push_overwrite(render_start.elapsed());
        println!("Alright, sending. We took {:?}", render_start.elapsed());
        frame_tx.send(message).unwrap();
      }
    });
  }

  {
    let event_loop = event_loop.create_proxy();
    std::thread::spawn(move || {
      let mut frame_queue = Vec::new();
      let mut drawn_frames = Vec::new();
      loop {
        // println!("Starting a loop...");
        if frame_queue.len() == 0 {
          // println!("Waiting for events...");
          let frame = frame_rx.recv().unwrap();
          frame_queue.push(frame);
        }
        // std::thread::sleep(Duration::from_millis(10));
        // println!("Grabbing all the ready frames");
        while let Ok(frame) = frame_rx.try_recv() {
          println!("Picking one up...");
          frame_queue.push(frame);
        }
        // println!("Picking the best! (We have {})", frame_queue.len());

        let mut chosen_frame: Option<(FrameMessage, Duration)> = None;
        let mut now = Instant::now();
        let mut latest_drawn_time = latest_drawn_time.write().unwrap();
        {
          let latest_drawn_time = *latest_drawn_time;
          frame_queue.retain(|frame| frame.time > latest_drawn_time);
        }
        if *latest_drawn_time > now {
          now = *latest_drawn_time;
        }
        for frame in frame_queue.iter() {
          let delta = if frame.time > now {
            frame.time - now
          } else {
            now - frame.time
          };
          if chosen_frame
            .as_ref()
            .map(|(_, chosen_delta)| chosen_delta > &delta)
            .unwrap_or(true)
          {
            chosen_frame = Some((frame.clone(), delta));
          }
        }
        if let Some((chosen_frame, _)) = chosen_frame {
          *latest_drawn_time = chosen_frame.time;
          // Retain only frames after this one:
          frame_queue.retain(|frame| frame.time > chosen_frame.time);
          drawn_frames.push(Instant::now());
          drawn_frames.retain(|then| Instant::now() - *then < Duration::from_secs(10));
          println!("FPS: {}", drawn_frames.len() / 10);
          event_loop.send_event(chosen_frame).unwrap();
        } else {
          println!("We're starving!");
        }
      }
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
        Event::UserEvent(event) => {
          let mut buffer = surface.buffer_mut().unwrap();
          for index in 0..(WIDTH * HEIGHT) {
            buffer[index] = event.buffer[index];
          }
          buffer.present().unwrap();
        }
        _ => {}
      }
    })
    .unwrap();
}
