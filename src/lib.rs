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
pub struct MandalaState {
    color: Color,
    petal_rotate_transform: Transform,
    petal_scale_transform: Transform,
    petal_translate_transform: Transform,
}

impl MandalaState {
    /// Create a new open or closed state for the manipulation of petals
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

/// A single animation from value to value over a defined time
struct MandalaTransition {
    start_time: f32,  // [Sec] When we started the latest transition
    duration: f32,    // [Sec] Where we got here
    start_value: f32, // [0.0..1.0] The position we are animating from
    end_value: f32,   // [0.0..1.0] The position we are animating to
}

impl MandalaTransition {
    /// An interplated animation from 'start_time' lasting 'duration' and sweeping from mandala state 'start_value' [0.0-1.0] to 'end_value' [0.0-1.0]
    fn new(start_time: f32, duration: f32, start_value: f32, end_value: f32) -> Self {
        Self {
            start_time,
            duration,
            start_value,
            end_value,
        }
    }

    /// A non-animated, fixed value
    fn fixed_value(value: f32) -> Self {
        let start_time = 0.0;
        let duration = 0.1;

        Self {
            start_time,
            duration,
            start_value: value,
            end_value: value,
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
    petal: MutableMesh,
    current_transition: MandalaTransition,
}

impl Mandala {
    /// Create a new Mandala
    ///
    /// By default, this will render a 3sec transition from open to closed state on creation. You can tailor this by
    pub fn new(
        petal_svg_filename: &str,
        screen_position: impl Into<Vector>,
        scale: impl Into<Vector>,
        petal_count: usize,
        mandala_state_open: MandalaState,
        mandala_state_closed: MandalaState,
        value: f32,
    ) -> Self {
        let mandala_center = Transform::translate(screen_position) * Transform::scale(scale);
        let petal = MutableMesh::new(petal_svg_filename);
        let mut petal_rotation: Vec<Transform> = Vec::new();
        let petal_angle = 360.0 / petal_count as f32;
        for i in 0..petal_count {
            petal_rotation.push(Transform::rotate(petal_angle * i as f32));
        }
        let current_transition = MandalaTransition::fixed_value(value);

        Self {
            petal_count,
            mandala_state_open,
            mandala_state_closed,
            mandala_center,
            petal_rotation,
            current_transition,
            petal,
        }
    }

    /// Initiate an animated transition from the value at 'current_time' [sec] value to 'target_value' [0.0-1.0] which will complete 'transition_duration' [sec] from now
    ///
    /// Note that for continuous smooth animation as a sequence of linear slides without pauses in between, you may want 'duration' to be slightly greater than the expected rate at which new values will arrive (example: every 0.2sec with 0.3sec max jitter on data source and network send plus receive task runtime, so set duration to 0.5). This keeps the animation smooth even when the data flow driving it and the computer rendering it are not smooth. The cost is you will be up to 0.3sec behind the latest value received, but this buffer time covers normally expected delays in receiving new values. If the value expected 0.2sec from the previous one receive time is more that 'transition_duration' (0.5sec) late, the animation will have time to complete and the value will appear to freeze until a new value arrives.
    pub fn start_transition(
        &mut self,
        current_time: f32,
        transition_duration: f32,
        target_value: f32,
    ) {
        debug_assert!(current_time >= 0.0);
        debug_assert!(transition_duration >= 0.0);
        debug_assert!(target_value.is_finite());

        let current_value = self.current_value(current_time);
        println!(
            "Start transition current: {}  target: {}",
            current_value, target_value
        );

        self.current_transition = MandalaTransition::new(
            current_time,
            transition_duration,
            current_value,
            target_value,
        )
    }

    /// Get a [0.0..1.0] number representing %open of the mandala based on the transition rendering time
    pub fn current_value(&self, current_time: f32) -> f32 {
        debug_assert!(current_time >= 0.0);
        let start = self.current_transition.start_value;
        let end = self.current_transition.end_value;

        let val = start + (end - start) * self.current_percent(current_time);

        debug_assert!(val.is_finite());

        val
    }

    /// Get a [0.0..1.0] number representing %complete of the transition rendering time
    pub fn current_percent(&self, current_time: f32) -> f32 {
        debug_assert!(current_time >= self.current_transition.start_time);
        let end_time = self.current_transition.start_time + self.current_transition.duration;
        if current_time > end_time {
            return 1.0;
        }

        (current_time - self.current_transition.start_time) / self.current_transition.duration
    }

    /// Find the float % from [start..end] with linear interpolation based on time
    fn interpolate_value(&self, current_time: f32, start: f32, end: f32) -> f32 {
        start + (end - start) * self.current_value(current_time)
    }

    /// Find the Tranform value from [start..end] using independent linear interpolation on each matrix element based on time
    fn current_transform(
        &self,
        current_time: f32,
        start: &Transform,
        end: &Transform,
    ) -> Transform {
        *start + (*end - *start) * self.current_value(current_time)
    }

    /// Find the Color value from [start..end] with linear interpolation of each ARGB value using independent linear interpolation
    /// Note: this may not be aesthetically ideal as you frequently interpolate through a brighter center-of-color-wheel value on the way to your destination. Choose your colors accordingly
    fn interpolate_color(&self, current_time: f32) -> Color {
        Color {
            r: self.interpolate_value(
                current_time,
                self.mandala_state_closed.color.r,
                self.mandala_state_open.color.r,
            ),
            g: self.interpolate_value(
                current_time,
                self.mandala_state_closed.color.g,
                self.mandala_state_open.color.g,
            ),
            b: self.interpolate_value(
                current_time,
                self.mandala_state_closed.color.b,
                self.mandala_state_open.color.b,
            ),
            a: self.interpolate_value(
                current_time,
                self.mandala_state_closed.color.a,
                self.mandala_state_open.color.a,
            ),
        }
    }

    /// Get the state of the mandala based on time and linear interpolation of all values between endpoints
    fn current_state(&mut self, current_time: f32) -> MandalaState {
        let color = self.interpolate_color(current_time);
        let petal_rotate_transform = self.current_transform(
            current_time,
            &self.mandala_state_open.petal_rotate_transform,
            &self.mandala_state_closed.petal_rotate_transform,
        );
        let petal_scale_transform = self.current_transform(
            current_time,
            &self.mandala_state_open.petal_scale_transform,
            &self.mandala_state_closed.petal_scale_transform,
        );
        let petal_translate_transform = self.current_transform(
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

    /// Render the interpolated current time state to the ShapeRenderer's display mesh
    pub fn draw(&mut self, current_time: f32, shape_renderer: &mut ShapeRenderer) {
        let mandala_state = self.current_state(current_time);

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
