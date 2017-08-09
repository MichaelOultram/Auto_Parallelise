extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate ssi_model;
extern crate rand;

mod support;
mod worker;

mod process_window;
use process_window::*;

mod simulate_window;
use simulate_window::*;

use ssi_model::process as process;
use process::Process as Process;


const CLEAR_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub struct ModelState {
    pub init_process: Option<Process>,
}

impl ModelState {
    pub fn new() -> Self {
        ModelState {
            init_process: None,
        }
    }
}

fn main() {

    let mut model_state = ModelState::new();
    let mut process_window = ProcessWindow::new();
    let mut simulate_window = SimulateWindow::new();

    support::run("ssi-prototype".to_string(), CLEAR_COLOR, move |ui| {
        process_window.render(&mut model_state, ui);
        simulate_window.render(&mut model_state, ui);
        true
    });
}
