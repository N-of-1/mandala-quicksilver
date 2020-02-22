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
use std::io::{prelude::*, BufReader};

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
        let mut path = parse_path_from_svg_str(svg_file_name);
        let tessellator = FillTessellator::new();
        let color = Color::RED; // Initial state will be overriden on first draw

        Self {
            color,
            transform: Transform::IDENTITY,
            path,
            tessellator,
        }
    }

    pub fn update_path(&mut self, svg_file_name: &str) -> &mut Self {
        self.path = parse_path_from_svg_str(svg_file_name);

        self
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
pub struct MandalaState {
    color: Color,
    petal_rotate_transform: Transform,
    petal_scale_transform: Transform,
    petal_translate_transform: Transform,
}

impl MandalaState {
    pub fn new(
        color: Color,
        petal_rotate_transform: Transform,
        petal_scale_transform: Transform,
        petal_translate_transform: Transform,
    ) -> Self {
        Self {
            color,
            petal_rotate_transform,
            petal_scale_transform,
            petal_translate_transform,
        }
    }
}

/// A flower-like set of "petals" arranged evenly around an invisible central hub
///
/// The petals can "open", change color and other tranformations applied at runtime with clock-based smoothing between rendered frames
pub struct Mandala {
    petal_count: usize,
    mandala_state_open: MandalaState,
    mandala_state_closed: MandalaState,
    mandala_center: Transform,
    petal_rotation: Vec<Transform>,
    current_phase_start: f32, // [Sec] When we started the latest transition
    current_phase_duration: f32, // [Sec] Where we got here
    //petal_nodes: Vec<MutableMesh>,
    svg_d_strings: Vec<String>,
}

impl Mandala {
    pub fn new(
        petal_shapes_filename: &str,
        screen_position: impl Into<Vector>,
        scale: impl Into<Vector>,
        petal_count: usize,
        petal_stages: usize,
        mandala_state_open: MandalaState,
        mandala_state_closed: MandalaState,
    ) -> Self {
        let mandala_center = Transform::translate(screen_position) * Transform::scale(scale);
        let current_phase_start = 0.0; // Start the transition clock when the application starts
        let current_phase_duration = 3.0; // Start the transition clock when the application starts
        let svg_d_strings: Vec<String> = lines_from_file(petal_shapes_filename);
        //let mut petal_nodes: Vec<MutableMesh> = Vec::new();
 //       for i in 0..petal_stages {
  //          petal_nodes.push(MutableMesh::new(&svg_d_strings[i]))
    //    }
        let mut petal: MutableMesh = MutableMesh::new(&svg_d_strings[29 as usize]);
        //let petal: MutableMesh = p;
        let mut petal_rotation: Vec<Transform> = Vec::new();
        let petal_angle = 360.0 / petal_count as f32;
        for i in 0..petal_count {
            petal_rotation.push(Transform::rotate(petal_angle * i as f32));
        }

        Self {
            petal_count,
            state_open: MandalaState {
                color: color_open,
                petal_translate_transform: Transform::translate((0.0,0.0)),
                petal_rotate_transform: Transform::rotate(-5),
                petal_scale_transform: Transform::scale((1., 1.)),
            },
            state_closed: MandalaState {
                color: color_closed,
                petal_translate_transform: Transform::translate((0.0,0.0)),
                petal_rotate_transform: Transform::rotate(5.0),
                petal_scale_transform: Transform::scale((1.0, 1.0)),
            },
            mandala_center,
            petal_rotation,
            current_phase_start,
            current_phase_duration,
            svg_d_strings
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
                self.mandala_state_closed.color.r,
                self.mandala_state_open.color.r,
            ),
            g: self.interpolate(
                current_time,
                self.mandala_state_closed.color.g,
                self.mandala_state_open.color.g,
            ),
            b: self.interpolate(
                current_time,
                self.mandala_state_closed.color.b,
                self.mandala_state_open.color.b,
            ),
            a: self.interpolate(
                current_time,
                self.mandala_state_closed.color.a,
                self.mandala_state_open.color.a,
            ),
        }
    }

    fn current_state(&mut self, current_time: f32) -> MandalaState {
        let color = self.interpolate_color(current_time);
        let petal_rotate_transform = self.interpolate_transform(
            current_time,
            &self.mandala_state_open.petal_rotate_transform,
            &self.mandala_state_closed.petal_rotate_transform,
        );
        let petal_scale_transform = self.interpolate_transform(
            current_time,
            &self.mandala_state_open.petal_scale_transform,
            &self.mandala_state_closed.petal_scale_transform,
        );
        let petal_translate_transform = self.interpolate_transform(
            current_time,
            &self.mandala_state_open.petal_translate_transform,
            &self.mandala_state_closed.petal_translate_transform,
        );

        MandalaState {
            color,
            petal_rotate_transform,
            petal_scale_transform,
            petal_translate_transform,
        }
    }

    pub fn draw(&mut self, current_time: f32, shape_renderer: &mut ShapeRenderer, index: usize) {
        let mandala_state: MandalaState = self.current_state(current_time);
        let mut petal = MutableMesh::new(&self.svg_d_strings[index]);
        petal.set_color(mandala_state.color);

        // For each petal
        for i in 0..self.petal_count {
            let petal_rot: &Transform = self.petal_rotation.get(i).unwrap();
            petal.set_transform(
                self.mandala_center
                    * *petal_rot
                    * mandala_state.petal_translate_transform
                    * mandala_state.petal_scale_transform
                    * mandala_state.petal_rotate_transform,
            );

            petal.tesselate(shape_renderer);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParseError;

fn parse_path_from_svg_str(svg_str: &str) -> Path {
    //let path_str = build_path_str_from_svg_str(svg_str);
    build_path(Path::builder().with_svg(), &svg_str).unwrap()
}

fn lines_from_file(filename: impl AsRef<std::path::Path>) -> Vec<String> {
    let file = File::open(filename).expect("no such file");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::MandalaState;
    use quicksilver::{geom::Transform, graphics::Color};

    #[test]
    fn test_add_mandala_transforms() {
        let left = Transform::IDENTITY;
        let right = left * 2 - Transform::IDENTITY;
        assert_eq!(left, right);
    }

    #[test]
    fn test_create_mandala_state() {
        let _mandala_state_open = MandalaState {
            color: Color::RED,
            petal_rotate_transform: Transform::rotate(90),
            petal_translate_transform: Transform::translate((50.0, 0.0)),
            petal_scale_transform: Transform::scale((1.0, 1.0)),
        };
    }
}
