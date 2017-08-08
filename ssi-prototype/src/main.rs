extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate ssi_model;

use imgui::*;
mod support;
use ssi_model::process::Process as Process;
use ssi_model::process::Instruction as Instruction;

const CLEAR_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

fn main() {
    let mut num_machines = 10;

    let mut num_processes = 500;
    let mut is_generating = false;
    let mut init_process : Option<Process> = None;

    support::run("ssi-prototype".to_string(), CLEAR_COLOR, |ui| {
        ui.window(im_str!("Simulation"))
            .size((300.0, 100.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                ui.slider_int(im_str!("Number of machines"), &mut num_machines, 1, 50).build();
                ui.separator();
                //ui.button(im_str!("Simulate"), (100, 50));
            });

        ui.window(im_str!("Process Configuration"))
            .size((300.0, 100.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                if is_generating {
                    ui.text(im_str!("Generating Process Tree\nPlease Wait"));
                } else {
                    if ui.collapsing_header(im_str!("Generator")).build() {
                        ui.slider_int(im_str!("Number of processes"), &mut num_processes, 1, 2000).build();
                    }

                    // Print tree
                    if ui.collapsing_header(im_str!("Process Tree")).build() {
                        match init_process {
                            Some(ref process) => render_process_tree(ui, process),
                            None => ui.text(im_str!("Not yet generated")),
                        }
                    }

                    if ui.small_button(im_str!("Generate")) {
                        is_generating = true;
                    }

                    // Toggle for CPU/IO counts
                    // Save/Load
                }
            });
        true
    });
}

fn render_process_tree(ui : &Ui, process: &Process) {
    ui.tree_node(&ImString::new(process.to_string()))
        .opened(true, ImGuiSetCond_Always)
        .build(|| {
            for instruction in &process.program {
                match instruction {
                    &Instruction::Spawn(ref p) => render_process_tree(ui, p),
                    _ => {},
                }
            }
        });
}
