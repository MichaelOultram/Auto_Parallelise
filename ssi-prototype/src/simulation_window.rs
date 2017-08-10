use rand;
use imgui::*;

use std::thread;
use std::thread::JoinHandle;
use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;
use std::collections::VecDeque;

use ModelState;
use process::*;
use ssi_model::machine::*;
use ssi_model::router::*;
use worker::Worker as Worker;

pub struct SimulationWindow {
    pub generator_worker : Worker<Process>,
    pub simulation_worker : Worker<()>,
    pub export_worker : Worker<()>,
    pub generator_config : Generator,
    pub simulation_config: MachineConfig,
    pub terminate_simulation: Option<Box<Fn()>>,
    pub show_cpu_io : bool,
}

impl SimulationWindow {
    pub fn new() -> Self {
        SimulationWindow {
            generator_worker: Worker::dummy(),
            simulation_worker: Worker::dummy(),
            export_worker: Worker::dummy(),
            generator_config: Generator::default(),
            simulation_config: MachineConfig::new(),
            terminate_simulation: None,
            show_cpu_io: false,
        }
    }

    pub fn render(&mut self, model : &mut ModelState, ui: &Ui) {
        ui.window(im_str!("Simulation"))
            .size((300.0, 100.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                if self.generator_worker.working {
                    ui.text(im_str!("Generating Process Tree\nPlease Wait"));
                    model.init_process = self.generator_worker.result();

                    match model.init_process {
                        Some(ref mut p) => p.status = Status::Runnable,
                        None => {},
                    }
                } else if self.export_worker.working {
                    ui.text(im_str!("Exporting Process Tree\nPlease Wait"));
                    self.export_worker.result();
                } else if self.simulation_worker.working {
                    ui.text(im_str!("Simulating\nPlease Wait"));
                    self.simulation_worker.result();
                    let mut reset_pressed = false;
                    match self.terminate_simulation {
                        Some(ref terminate_simulation) => {
                            if ui.button(im_str!("Stop Simulation"), ImVec2::new(150.0, 25.0)) {
                                terminate_simulation();
                                reset_pressed = true;
                            }
                        },
                        None => {},
                    }
                    if reset_pressed {
                        self.terminate_simulation = None;
                    }
                } else {
                    // Process Generator
                    if ui.collapsing_header(im_str!("Process Generator")).default_open(true).build() {
                        self.process_generator(model, ui);
                    }

                    // Process Tree tree
                    let num_processes = match model.init_process {
                        Some(ref p) => p.num_processes(),
                        None => 0,
                    };
                    if ui.collapsing_header(im_str!("Process Tree [{}]", num_processes)).build() {
                        ui.checkbox(im_str!("show cpu/io instructions"), &mut self.show_cpu_io);
                        match model.init_process {
                            Some(ref process) => SimulationWindow::render_process_tree(ui, process, self.show_cpu_io),
                            None => {},
                        }
                    }

                    // Simulation Settings
                    if ui.collapsing_header(im_str!("Simulation")).default_open(true).build() {
                        self.simulation(model, ui);
                    }

                }
            });
    }
}

// Process Tree / Generation functions
impl SimulationWindow {
    fn process_generator(&mut self, model : &mut ModelState, ui: &Ui) {
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
            // TODO: Export without cloning
            let init_process = model.init_process.clone();
            self.export_worker = Worker::start(move || {
                match init_process {
                    Some(ref p) => println!("{}", p.to_json()),
                    None => println!("No process tree"),
                }
            });
        }

        ui.separator();

        SimulationWindow::render_process_generator_config(ui, &mut self.generator_config);
        ui.spacing();
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

    fn render_process_generator_config(ui : &Ui, generator : &mut Generator) {
        ui.slider_int(im_str!("number"), &mut generator.num_processes, 1, 2000).build();
        ui.spacing();

        ui.text(im_str!("Cycles:"));
        ui.slider_int(im_str!("min##1"), &mut generator.min_cycles, 1, 2000).build();
        ui.slider_int(im_str!("max##1"), &mut generator.max_cycles, 1, 2000).build();
        ui.spacing();

        ui.text(im_str!("Instructions:"));
        ui.slider_int(im_str!("min##2"), &mut generator.min_instructions, 1, 2000).build();
        ui.slider_int(im_str!("max##2"), &mut generator.max_instructions, 1, 2000).build();
        ui.spacing();

        ui.text(im_str!("Child Processes:"));
        ui.slider_int(im_str!("max##3"), &mut generator.max_child_processes, 1, 250).build();
        ui.slider_float(im_str!("initial %"), &mut generator.child_branch_rate_initial, 0.0001, 1.0).build();
        ui.slider_float(im_str!("ramp %"), &mut generator.child_branch_rate_ramp, -0.5, 0.5).build();
        ui.spacing();
    }

    fn render_process_tree(ui : &Ui, process: &Process, display_cpu_io : bool) {
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
                            SimulationWindow::render_process_tree(ui, p, display_cpu_io);
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
    fn simulation(&mut self, model : &mut ModelState, ui: &Ui) {
        if ui.button(im_str!("Simulate"), ImVec2::new(100.0, 25.0)) {
            match model.init_process {
                Some(ref p) => {
                    let init_process = p.clone(); //TODO remove this clone

                    let mut router = Router::new(300);

                    let machine_handles : Vec<JoinHandle<()>>;
                    {
                        // Create the machines
                        let mut machines = VecDeque::new();
                        for machine_id in 0..self.simulation_config.num_machines as usize {
                            machines.push_back(Machine::new(self.simulation_config.clone(), machine_id, &mut router));
                        }

                        // Give the init process to the first machine
                        machines[0].global_queue.push_back(init_process);

                        // Start all machine threads
                        machine_handles = machines.into_iter().map(|mut m| thread::spawn(move || m.switch())).collect();
                    }

                    self.terminate_simulation = Some(router.start_router());

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
        ui.slider_int(im_str!("num machines"), &mut self.simulation_config.num_machines, 1, 50).build();
        ui.slider_int(im_str!("run queue size"), &mut self.simulation_config.local_queue_length, 1, 100).build();
        ui.slider_int(im_str!("cycles/context"), &mut self.simulation_config.num_cycles_per_context, 1, 1000).build();
        ui.slider_int(im_str!("max hops"), &mut self.simulation_config.max_hops, 1, 200).build();
    }
}
