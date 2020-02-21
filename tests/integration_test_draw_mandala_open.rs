extern crate mandala_quicksilver;

use mandala_quicksilver::{Mandala, MandalaState};
use quicksilver::{
    geom::Transform,
    graphics::{Color, Mesh, ShapeRenderer},
};

#[test]
fn integration_test_draw_mandala_open() {
    let mandala_state_open = MandalaState::new(
        Color::RED,
        Transform::rotate(90),
        Transform::translate((50.0, 0.0)),
        Transform::scale((1.0, 1.0)),
    );
    let mandala_state_closed = MandalaState::new(
        Color::YELLOW,
        Transform::rotate(0.0),
        Transform::translate((0.0, 0.0)),
        Transform::scale((0.1, 1.0)),
    );
    let mut mandala = Mandala::new(
        "tests/petal.svg",
        (500, 500),
        (2, 2),
        5,
        mandala_state_open,
        mandala_state_closed,
        1.0, // Open
    );
    assert_eq!(1.0, mandala.current_value(1.0));

    let mut mesh = Mesh::new();
    let mut shape_renderer = ShapeRenderer::new(&mut mesh, Color::PURPLE);
    let seconds_since_start = 0.1;

    mandala.draw(seconds_since_start, &mut shape_renderer);
    let expected = 660; //TODO Is this correct
    assert_eq!(expected, (&mesh.triangles).len());
}
