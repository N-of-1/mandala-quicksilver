extern crate mandala_quicksilver;

use mandala_quicksilver::{Mandala, MandalaState};
use quicksilver::{geom::Transform, graphics::Color};

#[test]
fn test_mandala_transition() {
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
    let initial_value = 3.0;
    let mut mandala = Mandala::new(
        "tests/petal.svg",
        (500, 500),
        (2, 2),
        5,
        mandala_state_open,
        mandala_state_closed,
        initial_value,
    );
    let mut current_time = 4.0;
    let transition_duration = 4.0;
    let target_value = 5.0; //+4.0 from initial open value
    mandala.start_transition(current_time, transition_duration, target_value);

    let mut current_value = mandala.current_value(current_time);
    assert_eq!(initial_value, current_value); // Value has not changed from initial creation default

    current_time = 6.0; // Half way through transition
    current_value = mandala.current_value(current_time);
    assert_eq!(4.0, current_value);

    current_time = 20.0; // Transition finished
    current_value = mandala.current_value(current_time);
    assert_eq!(5.0, current_value);
}
