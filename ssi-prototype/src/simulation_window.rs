use rand;
use imgui::*;

use std::sync::mpsc::Receiver;
use std::io::{BufRead, BufReader};
use std::fs::File;

use ModelState;
use process::*;
use machine::*;
use router::*;
use worker::Worker as Worker;

pub struct SimulationWindow {
    pub generator_worker : Worker<Process>,
    pub simulation_worker : Worker<()>,
    pub export_worker : Worker<()>,
    pub terminate_simulation: Option<Box<Fn()>>,
    pub simulation_relay: Option<Receiver<Packet>>,

    pub generator_config : Generator,
    pub machine_config: MachineConfig,
    pub init_process: Option<Process>,

    pub show_cpu_io : bool,
}

impl SimulationWindow {
    pub fn new() -> Self {
        SimulationWindow {
            generator_worker: Worker::dummy(),
            simulation_worker: Worker::dummy(),
            export_worker: Worker::dummy(),
            generator_config: Generator::default(),
            machine_config: MachineConfig::new(),
            init_process: None,
            terminate_simulation: None,
            simulation_relay: None,
            show_cpu_io: false,
        }
    }

    pub fn render(&mut self, model : &mut ModelState, ui: &Ui) {
        ui.window(im_str!("Simulation"))
            .size((324.0, 621.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                // Check workers if they are running every loop
                if self.generator_worker.working {
                    self.init_process = self.generator_worker.result();

                    if let Some(ref mut p) = self.init_process {
                        p.status = Status::Runnable;
                    }
                }
                if self.export_worker.working {
                    self.export_worker.result();
                }
                if self.simulation_worker.working {
                    self.simulation_worker.result();
                    if let Some(ref simulation_relay) = self.simulation_relay {
                        for packet in simulation_relay.try_iter() {
                            model.packets.push_back(packet);
                        }
                    }
                }

                // Render UI panels
                self.process_generator_section(ui);
                self.render_process_tree_section(ui);
                self.render_simulation_section(model, ui);
            });
    }
}

// Process Generation functions
impl SimulationWindow {
    fn process_generator_section(&mut self, ui: &Ui) {
        let section = ui.collapsing_header(im_str!("Process Generator")).default_open(true).build();

        if section && !self.generator_worker.working && !self.export_worker.working && !self.simulation_worker.working {
            // Show if section and no workers are running
            if ui.button(im_str!("Generate"), ImVec2::new(100.0, 25.0)) {
                let generator = self.generator_config.clone();
                self.generator_worker = Worker::start(move || {
                    let dict = SimulationWindow::dictionary();
                    let mut rng = rand::StdRng::new().unwrap();
                    generator.generate_process_tree(&mut rng, &dict)
                });
            }

            ui.same_line(110.0);
            if ui.button(im_str!("Import"), ImVec2::new(100.0, 25.0)) {
                // TODO: Import process tree
                unimplemented!();
            }

            ui.same_line(215.0);
            if ui.button(im_str!("Export"), ImVec2::new(100.0, 25.0)) {
                // TODO: Export into a file
                let init_process = self.init_process.clone();
                self.export_worker = Worker::start(move || {
                    if let Some(ref p) = init_process {
                        println!("{}", p.to_json());
                    } else {
                        println!("No process tree");
                    }
                });
            }

            ui.separator();

            ui.slider_int(im_str!("number"), &mut self.generator_config.num_processes, 1, 2000).build();
            ui.spacing();

            ui.text(im_str!("Cycles:"));
            ui.slider_int(im_str!("min##1"), &mut self.generator_config.min_cycles, 1, 2000).build();
            ui.slider_int(im_str!("max##1"), &mut self.generator_config.max_cycles, 1, 2000).build();
            ui.spacing();

            ui.text(im_str!("Instructions:"));
            ui.slider_int(im_str!("min##2"), &mut self.generator_config.min_instructions, 1, 2000).build();
            ui.slider_int(im_str!("max##2"), &mut self.generator_config.max_instructions, 1, 2000).build();
            ui.spacing();

            ui.text(im_str!("Child Processes:"));
            ui.slider_int(im_str!("max##3"), &mut self.generator_config.max_child_processes, 1, 250).build();
            ui.slider_float(im_str!("initial %"), &mut self.generator_config.child_branch_rate_initial, 0.0001, 1.0).build();
            ui.slider_float(im_str!("ramp %"), &mut self.generator_config.child_branch_rate_ramp, -0.5, 0.5).build();
            ui.spacing();
        } else if section {
            // Otherwise show if section is open
            ui.text(im_str!("Unavailable whilst working"));
        }
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
}

// Process tree functions
impl SimulationWindow {
    fn render_process_tree_section(&mut self, ui : &Ui) {
        let num_processes = match self.init_process {
            Some(ref p) => p.num_processes(),
            None => 0,
        };
        if ui.collapsing_header(im_str!("Process Tree [{}]", num_processes)).build() {
            ui.checkbox(im_str!("show cpu/io instructions"), &mut self.show_cpu_io);
            if let Some(ref process) = self.init_process {
                SimulationWindow::render_process_tree_helper(ui, process, self.show_cpu_io);
            }
        }
    }

    fn render_process_tree_helper(ui : &Ui, process: &Process, display_cpu_io : bool) {
        ui.tree_node(&ImString::new(process.to_string()))
            .opened(true, ImGuiSetCond_FirstUseEver)
            .build(|| {
                let mut instruction_count = 0;
                for instruction in &process.program {
                    match instruction {
                        &Instruction::Spawn(ref p) => {
                            if display_cpu_io && instruction_count > 0 {
                                ui.tree_node(&ImString::new(format!("{} instructions", instruction_count)))
                                    .opened(false, ImGuiSetCond_FirstUseEver).build(|| {});
                            }
                            SimulationWindow::render_process_tree_helper(ui, p, display_cpu_io);
                            instruction_count = 0;
                        },
                        _ => instruction_count += 1, //TODO: Count CPU and IO separately
                    }
                }
                if display_cpu_io && instruction_count > 0 {
                    ui.tree_node(&ImString::new(format!("{} instructions", instruction_count)))
                        .opened(false, ImGuiSetCond_FirstUseEver).build(|| {});
                }
            });
    }
}

// Simulation functions
impl SimulationWindow {
    fn render_simulation_section(&mut self, model : &mut ModelState, ui: &Ui) {
        let section = ui.collapsing_header(im_str!("Simulation")).default_open(true).build();
        if section && !self.generator_worker.working && !self.export_worker.working && !self.simulation_worker.working {
            // Simulate Button
            if ui.button(im_str!("Simulate"), ImVec2::new(100.0, 25.0)) {
                self.start_simulation(model);
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
            ui.slider_int(im_str!("num machines"), &mut self.machine_config.num_machines, 1, 50).build();
            ui.slider_int(im_str!("run queue size"), &mut self.machine_config.local_queue_length, 1, 100).build();
            ui.slider_int(im_str!("cycles/context"), &mut self.machine_config.num_cycles_per_context, 1, 1000).build();
            ui.slider_int(im_str!("max hops"), &mut self.machine_config.max_hops, 1, 200).build();

        } else if section && self.simulation_worker.working {
            ui.text(im_str!("Simulating\nPlease Wait"));
            let mut reset_pressed = false;
            if let Some(ref terminate_simulation) = self.terminate_simulation {
                if ui.button(im_str!("Stop Simulation"), ImVec2::new(150.0, 25.0)) {
                    terminate_simulation();
                    reset_pressed = true;
                }
            }
            if reset_pressed {
                self.terminate_simulation = None;
            }
        } else if section {
            ui.text(im_str!("Unavailable whilst working"));
        }
    }

    fn start_simulation(&mut self, model : &mut ModelState) {
        match self.init_process {
            Some(ref p) => {
                let init_process = p.clone(); //TODO remove this clone

                model.clear(); // Clearing as running a new simulation
                model.num_machines = self.machine_config.num_machines as usize;
                model.max_queue_length = self.machine_config.local_queue_length as u32;

                let mut router = Router::new(300);
                let machine_handles = self.machine_config.start_machines(&mut router, init_process);

                let (terminate_simulation, simulation_relay) = router.start_router();
                self.terminate_simulation = Some(terminate_simulation);
                self.simulation_relay = Some(simulation_relay);

                self.simulation_worker = Worker::start(move || {
                    // Wait for all machines to have finished
                    for m in machine_handles {
                        match m.join() {
                            Ok(_) => {},
                            Err(e) => panic!(e),
                        }
                    }
                });
            },
            None => println!("No init process"),
        }
    }
}
