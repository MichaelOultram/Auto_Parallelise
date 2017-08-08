extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate ssi_model;
extern crate rand;

mod support;
mod worker;
mod process_window;

use imgui::*;
use ssi_model::process as process;

use process_window::*;
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
    let mut num_machines = 10;

    let mut model_state = ModelState::new();
    let mut process_window = ProcessWindow::new();

    support::run("ssi-prototype".to_string(), CLEAR_COLOR, move |ui| {
        process_window.render(&mut model_state, ui);

        ui.window(im_str!("Simulation"))
            .size((300.0, 100.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                if ui.button(im_str!("Simulate"), ImVec2::new(100.0, 25.0)) {
                    // TODO: Run simulation
                    unimplemented!();
                }

                ui.same_line(110.0);
                if ui.button(im_str!("Import"), ImVec2::new(100.0, 25.0)) {
                    // TODO: Import execution
                    unimplemented!();
                }

                ui.same_line(215.0);
                if ui.button(im_str!("Export"), ImVec2::new(100.0, 25.0)) {
                    // TODO: Export execution
                    unimplemented!();
                }

                ui.separator();
                ui.slider_int(im_str!("number"), &mut num_machines, 1, 50).build();
            });

        true
    });
}
