pub mod helper;
pub mod engine;
pub mod trackpad;
pub mod utils;
pub mod controller;
pub mod tests;

fn main() {
    helper::fix_cursor();
    // let controller = controller::Controller::new();
    let mut engine = engine::Engine::new();
    engine::Engine::sync_to_virtual_position(&mut engine);
    // trackpad::start_stream();
    println!("Hello, world!");
}
