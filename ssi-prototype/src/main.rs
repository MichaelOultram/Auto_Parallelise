extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_sys;
extern crate imgui_glium_renderer;
extern crate ssi_model;
extern crate rand;

mod support;
mod worker;

use std::collections::VecDeque;

mod simulation_window;
use simulation_window::*;

mod raw_packet_window;
use raw_packet_window::*;

mod machine_usage_window;
use machine_usage_window::*;

mod timescale_window;
use timescale_window::*;

mod extra_widgets;
use extra_widgets::*;

use ssi_model::*;
use router::*;

const CLEAR_COLOR: [f32; 4] = [0.2, 0.2, 0.2, 1.0];

pub struct ModelState {
    pub num_machines: usize,
    pub packets : VecDeque<Packet>,
    pub max_queue_length: u32,

    pub start_time_plot: f32,
    pub end_time_plot: f32,
}

impl ModelState {
    pub fn new() -> Self {
        ModelState {
            num_machines: 0,
            max_queue_length: 0,
            packets: VecDeque::new(),
            start_time_plot: 0.0,
            end_time_plot: 1.0,
        }
    }

    pub fn clear(&mut self) {
        self.packets = VecDeque::new();
    }
}

fn main() {
    let mut model_state = ModelState::new();
    let mut simulation_window = SimulationWindow::new();
    let mut raw_packet_window = RawPacketWindow::new();
    let mut machine_usage_window = MachineUsageWindow::new();
    let mut timescale_window = TimescaleWindow::new();
    let mut custom_render_window = CustomRenderWindow::new();


    support::run("ssi-prototype".to_string(), CLEAR_COLOR, move |ui, render_stats| {
        ui.main_menu_bar(|| {
            ui.menu(im_str!("File")).build(|| {});

            ui.menu(im_str!("Windows")).build(|| {
                ui.menu_item(im_str!("Simulation")).selected(&mut simulation_window.visible).build();
                ui.separator();
                ui.menu_item(im_str!("Timescale Window")).selected(&mut timescale_window.visible).build();
                ui.menu_item(im_str!("Raw Packet Viewer")).selected(&mut raw_packet_window.visible).build();
                ui.menu_item(im_str!("Machine Usage")).selected(&mut machine_usage_window.visible).build();
            });

            // UI Performance
            ui.text(im_str!("{} FPS, {} ms", render_stats.frames_per_second as u32, render_stats.frame_time as u32));
        });


        simulation_window.render(&mut model_state, ui);
        raw_packet_window.render(&mut model_state, ui);
        machine_usage_window.render(&mut model_state, ui);
        timescale_window.render(&mut model_state, ui);
        custom_render_window.render(&mut model_state, ui);

        true
    });
}
