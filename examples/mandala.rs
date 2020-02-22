// examples/display_mandala.rs

use mandala_quicksilver::{Mandala, MandalaState};

use quicksilver::{
    geom::{Transform, Vector},
    graphics::{Color, Mesh, ShapeRenderer},
    input::{ButtonState, Key},
    lifecycle::{run, Event, Settings, State, Window},
    Result,
};

#[macro_use]
extern crate log;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
extern crate wasm_timer;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use instant::Instant;

//const PETAL_FILENAME: &str = "petal_bad.svg";
const PETAL_STRINGS_FILENAME: &str = "svg_strings.txt";
const PETAL_COUNT: usize = 12;
const PETAL_STAGE: usize = 30;
const CANVAS_SIZE: (f32, f32) = (1024.0, 1024.0);
const FPS: f64 = 60.0; // Frames per second
const UPS: f64 = 60.0; // Updates per second

const COLOR_CRIMSON: Color = Color {
    // Crimson
    r: 220.0 / 256.0,
    g: 20.0 / 256.0,
    b: 60.0 / 256.0,
    a: 1.0,
};
const COLOR_TURQOISE: Color = Color {
    // Turqoise, translucent
    r: 64.0 / 256.0,
    g: 224.0 / 256.0,
    b: 208.0 / 256.0,
    a: 0.2,
};
const COLOR_BACKGROUND: Color = Color::BLACK;
const COLOR_PETAL_OPEN: Color = COLOR_CRIMSON;
const COLOR_PETAL_CLOSED: Color = COLOR_TURQOISE;

fn main() {
    run::<MandalaExample>(
        "Mandala Demo - press SPACE to restart easing, ESC to exit",
        Vector::new(CANVAS_SIZE.0, CANVAS_SIZE.1),
        Settings {
            multisampling: Some(4),
            update_rate: 1000. / UPS,
            draw_rate: 1000. / FPS,
            ..Settings::default()
        },
    );
}

struct MandalaExample {
    start_time: Instant,
    mandala: Mandala,
}

impl MandalaExample {
    fn seconds_since_start(&self) -> f32 {
        self.start_time.elapsed().as_nanos() as f32 / 1000000000.0
    }
}

impl State for MandalaExample {
    fn new() -> Result<MandalaExample> {
        let start_time = Instant::now();
        let mandala_state_open = MandalaState::new(
            COLOR_PETAL_OPEN,
            Transform::rotate(90),
            Transform::translate((50.0, 0.0)),
            Transform::scale((1.0, 1.0)),
        );
        let mandala_state_closed = MandalaState::new(
            COLOR_PETAL_CLOSED,
            Transform::rotate(0.0),
            Transform::translate((0.0, 0.0)),
            Transform::scale((0.1, 1.0)),
        );
        let mandala = Mandala::new(
            PETAL_STRINGS_FILENAME,
            (CANVAS_SIZE.0 / 2.0, CANVAS_SIZE.1 / 2.0),
            (2.0, 2.0),
            PETAL_COUNT,

            PETAL_STAGE,
           
            mandala_state_open,
            mandala_state_closed,

        );

        Ok(MandalaExample {
            start_time,
            mandala,
        })
    }

    fn event(&mut self, event: &Event, window: &mut Window) -> Result<()> {
        match *event {
            Event::Key(Key::Space, ButtonState::Pressed) => {
                self.start_time = Instant::now(); // Restart the transition
            }
            Event::Key(Key::Escape, ButtonState::Pressed) => {
                window.close();
            }
            _ => (),
        }
        Ok(())
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(COLOR_BACKGROUND)?;

        let mut mesh = Mesh::new();
        let mut shape_renderer = ShapeRenderer::new(&mut mesh, Color::RED);

        self.mandala
            .draw(self.seconds_since_start(), &mut shape_renderer, 1 as usize);
        window.mesh().extend(&mesh);

        Ok(())
    }

    fn update(&mut self, _window: &mut Window) -> Result<()> {
        Ok(())
    }

    fn handle_error(error: quicksilver::Error) {
        error!("Unhandled error: {:?}", error);
        panic!("Unhandled error: {:?}", error);
    }
}
