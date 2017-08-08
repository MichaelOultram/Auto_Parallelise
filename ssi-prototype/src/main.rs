extern crate glium;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;
extern crate ssi_model;
extern crate rand;

mod support;
mod worker;

use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;

use imgui::*;
use ssi_model::process as process;

use process::Process as Process;
use process::Instruction as Instruction;

use worker::Worker as Worker;

const CLEAR_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

fn main() {
    let mut num_machines = 10;

    let mut process_worker = Worker::dummy();
    let mut process_settings = process::Generator::default();
    let mut init_process : Option<Process> = None;

    support::run("ssi-prototype".to_string(), CLEAR_COLOR, move |ui| {
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
                if process_worker.working {
                    ui.text(im_str!("Generating Process Tree\nPlease Wait"));
                    init_process = process_worker.result();
                } else {
                    if render_process_settings(ui, &mut process_settings) {
                        let generator = process_settings.clone();
                        process_worker = Worker::start(move || {
                            let dict = dictionary();
                            let mut rng = rand::StdRng::new().unwrap();
                            generator.generate_process_tree(&mut rng, &dict)
                        });
                    }

                    ui.spacing();

                    // Print tree
                    if ui.collapsing_header(im_str!("Process Tree")).build() {
                        match init_process {
                            Some(ref process) => {
                                render_process_tree(ui, process);
                                ui.button(im_str!("Export"), ImVec2::new(-1.0, 15.0));
                            },
                            None => ui.text(im_str!("[empty]")),
                        }
                    }

                    ui.spacing();

                    ui.button(im_str!("Import"), ImVec2::new(-1.0, 15.0));
                    // Toggle for CPU/IO counts
                    // Save/Load
                }
            });
        true
    });
}

fn dictionary() -> Vec<String> {
    let mut dict = vec![];

    // Put each line of dictionary.txt into the vector
    let f = File::open("dictionary.txt").unwrap();
    let file = BufReader::new(&f);
    for line in file.lines(){
        let l = line.unwrap();
        dict.push(l);
    }

    dict
}

fn render_process_settings(ui : &Ui, generator : &mut process::Generator) -> bool {
    if ui.collapsing_header(im_str!("Process Generator")).build() {

        ui.slider_int(im_str!("num_processes"), &mut generator.num_processes, 1, 2000).build();
        ui.spacing();

        ui.slider_int(im_str!("min_cycles"), &mut generator.min_cycles, 1, 2000).build();
        ui.slider_int(im_str!("max_cycles"), &mut generator.max_cycles, 1, 2000).build();
        ui.spacing();

        ui.slider_int(im_str!("min_instructions"), &mut generator.min_instructions, 1, 2000).build();
        ui.slider_int(im_str!("max_instructions"), &mut generator.max_instructions, 1, 2000).build();
        ui.spacing();

        ui.slider_int(im_str!("max_child_processes"), &mut generator.max_child_processes, 1, 2000).build();
        ui.slider_float(im_str!("child_branch_rate"), &mut generator.child_branch_rate, 0.0, 1.0).build();
        ui.spacing();

        ui.button(im_str!("Generate"), ImVec2::new(-1.0, 25.0))
    } else {
        false
    }
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
