use imgui::*;

use ModelState;
use ssi_model::machine::*;
use worker::Worker as Worker;

pub struct SimulateWindow {
    pub worker : Worker<()>,
    pub config : MachineConfig,
}

impl SimulateWindow {
    pub fn new() -> Self {
        SimulateWindow {
            worker: Worker::dummy(),
            config: MachineConfig::new(),
        }
    }

    pub fn render(&mut self, model : &mut ModelState, ui: &Ui) {
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
                ui.slider_int(im_str!("num machines"), &mut self.config.num_machines, 1, 50).build();
                ui.slider_int(im_str!("run queue size"), &mut self.config.local_queue_length, 1, 100).build();
                ui.slider_int(im_str!("cycles/context"), &mut self.config.num_cycles_per_context, 1, 1000).build();
                ui.slider_int(im_str!("max hops"), &mut self.config.max_hops, 1, 200).build();
            });
    }
}
