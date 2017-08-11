extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate ssi_model;
extern crate rand;

mod support;
mod worker;

use std::collections::VecDeque;

mod simulation_window;
use simulation_window::*;

use ssi_model::*;
use router::*;

const CLEAR_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub struct ModelState {
    packets : VecDeque<Packet>,
}

impl ModelState {
    pub fn new() -> Self {
        ModelState {
            packets: VecDeque::new(),
        }
    }

    pub fn clear(&mut self) {
        self.packets = VecDeque::new();
    }
}

fn main() {
    let mut model_state = ModelState::new();
    let mut process_window = SimulationWindow::new();

    support::run("ssi-prototype".to_string(), CLEAR_COLOR, move |ui| {
        process_window.render(&mut model_state, ui);
        true
    });
}
