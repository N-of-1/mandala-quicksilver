// Draw the classic triangle to the screen
extern crate nalgebra;
extern crate quicksilver;
extern crate svg;

#[macro_use]
extern crate log;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
extern crate env_logger;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
extern crate web_logger;

use nalgebra::base::Matrix3;
use quicksilver::{
    geom::{Transform, Vector},
    graphics::{Color, Mesh, ShapeRenderer},
    input::{ButtonState, Key},
    lifecycle::{run, Event, Settings, State, Window},
    lyon::{
        path::Path,
        svg::path_utils::build_path,
        tessellation::{FillOptions, FillTessellator},
    },
    Result,
};
use std::f32::consts::PI;
use std::fs::File;
use std::io::Read;
use std::{iter::Sum, time::Instant};

const CANVAS_SIZE: (f32, f32) = (1024.0, 1024.0);
const FPS: f64 = 60.0; // Frames per second
const UPS: f64 = 60.0; // Updates per second

const COLOR_BACKGROUND: Color = Color {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    a: 1.0,
};

const COLOR_PETAL_CLOSED: Color = Color {
    r: 0.0,
    g: 1.0,
    b: 1.0,
    a: 0.1,
};

const COLOR_PETAL_OPEN: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

struct MutableMesh {
    color: Color,
    transform: Transform,
    path: Path,
    tessellator: FillTessellator,
}

/// A renderable vector object from SVG with a runtime tranformation matrix
impl MutableMesh {
    /// Create a default with key values specified
    fn new(svg_file_name: &str, color: Color) -> Self {
        let path = svg_to_path(svg_file_name);
        let tessellator = FillTessellator::new();

        Self {
            color,
            transform: Transform::IDENTITY,
            path,
            tessellator,
        }
    }

    /// Render the vector shape with current transform into screen triangles
    fn tesselate(&mut self, shape_renderer: &mut ShapeRenderer) {
        shape_renderer.set_color(self.color);
        shape_renderer.set_transform(self.transform);

        self.tessellator
            .tessellate_path(&self.path, &FillOptions::tolerance(0.01), shape_renderer)
            .unwrap();
    }

    /// This transform will be applied to all new shapes as well
    /// Call tesselate() after all such mutations are complete
    fn set_transform(&mut self, transform: Transform) -> &mut Self {
        self.transform = transform;

        self
    }

    /// Call tesselate() after all such mutations are complete
    fn set_color(&mut self, color: Color) -> &mut Self {
        self.color = color;

        self
    }
}

/// A representation of how open/closed the mandala is to mark the endpoints of motion
#[derive(Debug)]
struct MandalaState {
    color: Color,
    petal_rotate_transform: Transform,
    petal_scale_transform: Transform,
    petal_translate_transform: Transform,
}

/// A flower-like set of "petals" arranged evenly around an invisible central hub
///
/// The petals can "open", change color and other tranformations applied at runtime with clock-based smoothing between rendered frames
struct Mandala {
    petal_count: usize,
    center_radius: f32,
    state_open: MandalaState,
    state_closed: MandalaState,
    mandala_angle: f32, // Drive this externally to spin if desired
    mandala_center: Transform,
    petal_rotation: Vec<Transform>,
    current_phase_start: f32, // [Sec] When we started the latest transition
    current_phase_duration: f32, // [Sec] Where we got here
    petal: MutableMesh,
}

impl Mandala {
    fn new(
        petal_svg_filename: &str,
        screen_position: impl Into<Vector>,
        scale: impl Into<Vector>,
        petal_count: usize,
        center_radius: f32,
        color_open: Color,
        color_closed: Color,
    ) -> Self {
        let mandala_angle = 0.0; // No spin of petals relative to center
        let mandala_center = Transform::translate(screen_position) * Transform::scale(scale);
        let current_phase_start = 0.0; // Start the transition clock when the application starts
        let current_phase_duration = 3.0; // Start the transition clock when the application starts
        let petal = MutableMesh::new(petal_svg_filename, Color::GREEN);
        let mut petal_rotation: Vec<Transform> = Vec::new();
        let petal_angle = 360.0 / petal_count as f32;
        for i in 0..petal_count {
            petal_rotation.push(Transform::rotate(petal_angle * i as f32));
        }

        Self {
            petal_count,
            center_radius,
            state_open: MandalaState {
                color: color_open,
                petal_rotate_transform: Transform::rotate(90),
                petal_translate_transform: Transform::translate((50.0, 0.0)),
                petal_scale_transform: Transform::scale((1.0, 1.0)),
            },
            state_closed: MandalaState {
                color: color_closed,
                petal_rotate_transform: Transform::rotate(0.0),
                petal_translate_transform: Transform::translate((0.0, 0.0)),
                petal_scale_transform: Transform::scale((0.1, 1.0)),
            },
            mandala_angle,
            mandala_center,
            petal_rotation,
            current_phase_start,
            current_phase_duration,
            petal,
        }
    }

    fn current_percent(&self, current_time: f32) -> f32 {
        debug_assert!(current_time >= self.current_phase_start);

        let end_time = self.current_phase_start + self.current_phase_duration;
        if current_time > end_time {
            return 1.0;
        }

        (current_time - self.current_phase_start) / self.current_phase_duration
    }

    fn interpolate(&self, current_time: f32, start: f32, end: f32) -> f32 {
        start + (end - start) * self.current_percent(current_time)
    }

    fn interpolate_transform(
        &self,
        current_time: f32,
        start: &Transform,
        end: &Transform,
    ) -> Transform {
        *start + (*end - *start) * self.current_percent(current_time)
    }

    fn interpolate_color(&self, current_time: f32) -> Color {
        Color {
            r: self.interpolate(
                current_time,
                self.state_closed.color.r,
                self.state_open.color.r,
            ),
            g: self.interpolate(
                current_time,
                self.state_closed.color.g,
                self.state_open.color.g,
            ),
            b: self.interpolate(
                current_time,
                self.state_closed.color.b,
                self.state_open.color.b,
            ),
            a: self.interpolate(
                current_time,
                self.state_closed.color.a,
                self.state_open.color.a,
            ),
        }
    }

    fn current_state(&mut self, current_time: f32) -> MandalaState {
        let color = self.interpolate_color(current_time);
        let petal_rotate_transform = self.interpolate_transform(
            current_time,
            &self.state_open.petal_rotate_transform,
            &self.state_closed.petal_rotate_transform,
        );
        let petal_scale_transform = self.interpolate_transform(
            current_time,
            &self.state_open.petal_scale_transform,
            &self.state_closed.petal_scale_transform,
        );
        let petal_translate_transform = self.interpolate_transform(
            current_time,
            &self.state_open.petal_translate_transform,
            &self.state_closed.petal_translate_transform,
        );

        MandalaState {
            color,
            petal_rotate_transform,
            petal_scale_transform,
            petal_translate_transform,
        }
    }

    fn draw(&mut self, current_time: f32, shape_renderer: &mut ShapeRenderer) {
        let mandala_state: MandalaState = self.current_state(current_time);

        self.petal.set_color(mandala_state.color);

        // For each petal
        for i in 0..self.petal_count {
            let petal_rot: &Transform = self.petal_rotation.get(i).unwrap();
            self.petal.set_transform(
                self.mandala_center
                    * *petal_rot
                    * mandala_state.petal_translate_transform
                    * mandala_state.petal_scale_transform
                    * mandala_state.petal_rotate_transform,
            );

            self.petal.tesselate(shape_renderer);
        }
    }
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

#[derive(Clone, Debug, PartialEq)]
pub struct ParseError;

fn svg_to_path(file_name: &str) -> Path {
    match File::open(file_name) {
        Ok(mut file) => {
            let mut svg_str = String::new();
            file.read_to_string(&mut svg_str).unwrap();
            parse_path_from_svg_str(&svg_str)
        }
        Err(e) => panic!("Can not open SVG file: '{}', {}", file_name, e),
    }
}

fn parse_path_from_svg_str(svg_str: &str) -> Path {
    let path_str = extract_path_str_from_svg_str(svg_str);

    build_path(Path::builder().with_svg(), &path_str).unwrap()
}

fn extract_path_str_from_svg_str(svg_str: &str) -> String {
    let parser = svg::parser::Parser::new(svg_str);
    for event in parser {
        match event {
            svg::parser::Event::Tag(path, Start, attributes) => {
                if let Some(data) = attributes.get("d") {
                    return data.to_string();
                }
            }
            _ => (),
        }
    }

    panic!("Can not find path data in SVG file");
}

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

#[cfg(test)]
mod tests {
    use nalgebra::base::Matrix3;
    use quicksilver::geom::Transform;

    extern crate nalgebra;
    extern crate quicksilver;

    #[test]
    fn test_matrix_transform_identity() {
        let left = Transform::IDENTITY;
        let right: Transform = Matrix3::from_diagonal_element(1.0).into();

        assert_eq!(left, right);
    }
}
