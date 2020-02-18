// examples/display_mandala.rs

use mandala_quicksilver::{Mandala, MutableMesh};

use quicksilver::{
    geom::{Transform, Vector},
    graphics::{Color, Mesh, ShapeRenderer},
    input::{ButtonState, Key},
    lifecycle::{run, Event, Settings, State, Window},
    Result,
};
use std::time::Instant;

#[macro_use]
extern crate log;

const CANVAS_SIZE: (f32, f32) = (1024.0, 1024.0);
const FPS: f64 = 60.0; // Frames per second
const UPS: f64 = 60.0; // Updates per second
const COLOR_BACKGROUND: Color = Color {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    a: 1.0,
};
const COLOR_PETAL_OPEN: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};
const COLOR_PETAL_CLOSED: Color = Color {
    r: 0.0,
    g: 1.0,
    b: 1.0,
    a: 0.1,
};

fn main() {
    run::<LyonExample>(
        "Lyon Demo - press Space to switch between tessellation methods",
        Vector::new(CANVAS_SIZE.0, CANVAS_SIZE.1),
        Settings {
            multisampling: Some(4),
            update_rate: 1000. / UPS,
            draw_rate: 1000. / FPS,
            ..Settings::default()
        },
    );
}

struct LyonExample {
    filled_logo: MutableMesh,
    start_time: Instant,
    mandala: Mandala,
}

impl LyonExample {
    fn seconds_since_start(&self) -> f32 {
        self.start_time.elapsed().as_nanos() as f32 / 1000000000.0
    }
}

impl State for LyonExample {
    fn new() -> Result<LyonExample> {
        let filled_logo = MutableMesh::new("N-of-1-logo.svg", Color::RED);
        let start_time = Instant::now();
        let mandala = Mandala::new(
            "petal1.svg",
            (CANVAS_SIZE.0 / 2.0, CANVAS_SIZE.1 / 2.0),
            (2.0, 2.0),
            20,
            50.0,
            COLOR_PETAL_OPEN,
            COLOR_PETAL_CLOSED,
        );

        Ok(LyonExample {
            filled_logo,
            start_time,
            mandala,
        })
    }

    fn event(&mut self, event: &Event, window: &mut Window) -> Result<()> {
        match *event {
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
        let seconds_since_start = self.seconds_since_start();
        let scale = ((seconds_since_start * 3.0).sin() as f32 + 1.0) * 2.0;
        let color = Color {
            r: 1.0,
            g: 0.0,
            a: 1.0,
            b: (seconds_since_start * 4.0).sin(),
        };
        self.filled_logo.set_color(color);
        self.filled_logo.set_transform(
            Transform::translate((200, 200))
                * Transform::rotate(seconds_since_start * 5.0)
                * Transform::scale((scale, 1.0)),
        );
        let mut shape_renderer = ShapeRenderer::new(&mut mesh, self.filled_logo.color);

        // Draw the logo
        // self.filled_logo.tesselate(&mut shape_renderer);

        // Draw the mandala
        self.mandala.draw(seconds_since_start, &mut shape_renderer);

        // Merge the rendered mesh to screen
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
