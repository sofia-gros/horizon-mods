#[path = "fluid_example_source.rs"]
mod example;
use pluto_api::fluid::{contact::Bounds, Cell, Sample};

#[test]
fn actual_example_preserves_unknown_and_registration_contract() {
    example::initialize_fluids().expect("valid namespaced registration");
    example::initialize_fluids().expect("identical registration is idempotent");
    let bounds = || Bounds { min: [0.1, 0.0, 0.1], max: [0.9, 0.5, 0.9] };
    let dry = example::read_contacts(bounds(), |_| Sample::Air).expect("bounded query");
    assert_eq!(example::completely_known_rainwater_contact(&dry), Some(false));
    let unknown = example::read_contacts(bounds(), |_| Sample::Unloaded).expect("unloaded is a fact");
    assert_eq!(example::completely_known_rainwater_contact(&unknown), None);
    let wet = example::read_contacts(bounds(), |_| Sample::Fluid(Cell::source(example::RAINWATER)))
        .expect("registered source-cell facts");
    assert_eq!(example::completely_known_rainwater_contact(&wet), Some(true));
}
