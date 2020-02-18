// Draw the classic triangle to the screen
extern crate quicksilver;
extern crate svg;

//#[macro_use]
extern crate log;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
extern crate env_logger;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
extern crate web_logger;

use quicksilver::{
    geom::{Transform, Vector},
    graphics::{Color, ShapeRenderer},
    lyon::{
        path::Path,
        svg::path_utils::build_path,
        tessellation::{FillOptions, FillTessellator},
    },
};
use std::fs::File;
use std::io::Read;

pub struct MutableMesh {
    pub color: Color,
    pub transform: Transform,
    path: Path,
    tessellator: FillTessellator,
}

/// A renderable vector object from SVG with a runtime tranformation matrix
impl MutableMesh {
    /// Create a default with key values specified
    pub fn new(svg_file_name: &str) -> Self {
        let path = svg_to_path(svg_file_name);
        let tessellator = FillTessellator::new();
        let color = Color::RED; // Initial state will be overriden on first draw

        Self {
            color,
            transform: Transform::IDENTITY,
            path,
            tessellator,
        }
    }

    /// Render the vector shape with current transform into screen triangles
    pub fn tesselate(&mut self, shape_renderer: &mut ShapeRenderer) {
        shape_renderer.set_color(self.color);
        shape_renderer.set_transform(self.transform);

        self.tessellator
            .tessellate_path(&self.path, &FillOptions::tolerance(0.01), shape_renderer)
            .unwrap();
    }

    /// This transform will be applied to all new shapes as well
    /// Call tesselate() after all such mutations are complete
    pub fn set_transform(&mut self, transform: Transform) -> &mut Self {
        self.transform = transform;

        self
    }

    /// Call tesselate() after all such mutations are complete
    pub fn set_color(&mut self, color: Color) -> &mut Self {
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
pub struct Mandala {
    petal_count: usize,
    state_open: MandalaState,
    state_closed: MandalaState,
    mandala_center: Transform,
    petal_rotation: Vec<Transform>,
    current_phase_start: f32, // [Sec] When we started the latest transition
    current_phase_duration: f32, // [Sec] Where we got here
    petal: MutableMesh,
}

impl Mandala {
    pub fn new(
        petal_svg_filename: &str,
        screen_position: impl Into<Vector>,
        scale: impl Into<Vector>,
        petal_count: usize,
        color_open: Color,
        color_closed: Color,
    ) -> Self {
        let mandala_center = Transform::translate(screen_position) * Transform::scale(scale);
        let current_phase_start = 0.0; // Start the transition clock when the application starts
        let current_phase_duration = 3.0; // Start the transition clock when the application starts
        let petal = MutableMesh::new(petal_svg_filename);
        let mut petal_rotation: Vec<Transform> = Vec::new();
        let petal_angle = 360.0 / petal_count as f32;
        for i in 0..petal_count {
            petal_rotation.push(Transform::rotate(petal_angle * i as f32));
        }

        Self {
            petal_count,
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
            mandala_center,
            petal_rotation,
            current_phase_start,
            current_phase_duration,
            petal,
        }
    }

    pub fn current_percent(&self, current_time: f32) -> f32 {
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

    pub fn draw(&mut self, current_time: f32, shape_renderer: &mut ShapeRenderer) {
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

#[derive(Clone, Debug, PartialEq)]
pub struct ParseError;

pub fn svg_to_path(file_name: &str) -> Path {
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
            svg::parser::Event::Tag(_path, _type, attributes) => {
                if let Some(data) = attributes.get("d") {
                    return data.to_string();
                }
            }
            _ => (),
        }
    }

    panic!("Can not find path data in SVG file");
}

#[cfg(test)]
mod tests {
    use quicksilver::geom::Transform;

    #[test]
    fn test_add_mandala_transforms() {
        let left = Transform::IDENTITY;
        let right = left * 2 - Transform::IDENTITY;
        assert_eq!(left, right);
    }
}
