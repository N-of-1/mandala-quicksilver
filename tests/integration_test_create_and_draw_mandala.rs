extern crate mandala_quicksilver;

use mandala_quicksilver::{Mandala, MandalaState};
use quicksilver::{
    geom::Transform,
    graphics::{Color, Mesh, ShapeRenderer},
};

#[test]
fn test_create_and_draw_mandala() {
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
    );
    let mut mesh = Mesh::new();
    let mut shape_renderer = ShapeRenderer::new(&mut mesh, Color::PURPLE);
    let mut seconds_since_start = 0.1;
    mandala.draw(seconds_since_start, &mut shape_renderer);
    seconds_since_start = 65.0;
    mandala.draw(seconds_since_start, &mut shape_renderer);
    let triangles = mesh.triangles;
    let expected = 1320;

    assert_eq!(expected, triangles.len());
}
