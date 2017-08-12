use imgui::*;

use ModelState;
use machine::*;
use router::*;

pub struct MachineUsageWindow {
    plot_lines: Vec<Vec<f32>>,
    plot_size: usize,
}

impl MachineUsageWindow {
    pub fn new() -> Self {
        MachineUsageWindow {
            plot_lines : vec![],
            plot_size: 0,
        }
    }

    pub fn render(&mut self, model : &mut ModelState, ui: &Ui) {
        ui.window(im_str!("Machine Usage"))
            .size((324.0, 621.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                // Print message if no simulation data
                if model.num_machines == 0 {
                    ui.text(im_str!("No simulation data"));
                    return;
                }

                // Reset machine plots if machine numbers are not equal
                if true { //self.plot_lines.len() != model.num_machines {
                    self.reset_plots(model);
                    self.update_machine_usage(model);
                }

                // Render usage graphs
                for i in 0..model.num_machines {
                    let title = ImString::new(format!("machine-{}", i));
                    ui.plot_lines(&title, &self.plot_lines.get(i).unwrap())
                      .scale_min(0.0).scale_max(model.max_queue_length as f32).build();
                }
            });
    }


    fn reset_plots(&mut self, model : &mut ModelState) {
        self.plot_lines = vec![];
        self.plot_size = 1;
        for i in 0..model.num_machines {
            self.plot_lines.insert(i, vec![0.0]);
        }
    }

    fn extend_plot(&mut self) {
        for i in 0..self.plot_lines.len() {
            let plot = self.plot_lines.get_mut(i).unwrap();
            let last_element = plot.get(self.plot_size - 1).unwrap().clone();
            plot.insert(self.plot_size, last_element);
        }
        self.plot_size += 1;
    }

    fn update_machine_usage(&mut self, model: &mut ModelState) {
        // Recalculate machine usage
        for packet_i in 0..model.packets.len() {
            let packet = model.packets.get(packet_i).unwrap();
            self.extend_plot();

            if let PacketData::SimData(ref from_id, ref data) = packet.data {
                let vec = self.plot_lines.get_mut(from_id.clone()).unwrap();
                let last_element = vec.get_mut(self.plot_size - 1).unwrap();
                match data {
                    &SimData::ProcessStart(_) => *last_element += 1.0,
                    &SimData::ProcessEnd(_) => *last_element -= 1.0,
                    &SimData::ProcessSpawn(_) => {},
                }
            }
        }
    }
}
