use rand;
use imgui::*;

use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;

use ModelState;
use process::*;
use worker::Worker as Worker;

pub struct ProcessWindow {
    pub generator_worker : Worker<Process>,
    pub export_worker : Worker<()>,
    pub settings : Generator,
    pub show_cpu_io : bool,
}

impl ProcessWindow {
    pub fn new() -> Self {
        ProcessWindow {
            generator_worker: Worker::dummy(),
            export_worker: Worker::dummy(),
            settings: Generator::default(),
            show_cpu_io: false,
        }
    }

    pub fn render(&mut self, model : &mut ModelState, ui: &Ui) {
        ui.window(im_str!("Process Generator"))
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
                } else {
                    ui.same_line(5.0);
                    if ui.button(im_str!("Generate"), ImVec2::new(100.0, 25.0)) {
                        let generator = self.settings.clone();
                        self.generator_worker = Worker::start(move || {
                            let dict = ProcessWindow::dictionary();
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

                    ProcessWindow::render_process_settings(ui, &mut self.settings);
                    ui.spacing();

                    // Print tree
                    if ui.collapsing_header(im_str!("Process Tree")).build() {
                        ui.checkbox(im_str!("show cpu/io instructions"), &mut self.show_cpu_io);
                        match model.init_process {
                            Some(ref process) => ProcessWindow::render_process_tree(ui, process, self.show_cpu_io),
                            None => {},
                        }
                    }

                }
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

    fn render_process_settings(ui : &Ui, generator : &mut Generator) {
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
                            ProcessWindow::render_process_tree(ui, p, display_cpu_io);
                            instruction_count = 0;
                        },
                        _ => instruction_count += 1,
                    }
                }
                if display_cpu_io && instruction_count > 0 {
                    ui.tree_node(&ImString::new(format!("{} instructions", instruction_count)))
                        .opened(false, ImGuiSetCond_FirstUseEver).build(|| {});
                }
            });
    }
}
