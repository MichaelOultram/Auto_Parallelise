use imgui::*;

use extra_widgets::UiExtras;
use ModelState;
use router::*;

pub struct MachineUsageWindow {
    pub visible: bool,
    pub plot_lines: Vec<Vec<f32>>,
    pub wait_queue: u32, // Processes that have spawned, but have not started yet
    pub plot_size: usize,
    pub scale_min: i32,
    pub scale_max: i32,
    pub scale_max_max: i32,
}

impl MachineUsageWindow {
    pub fn new() -> Self {
        MachineUsageWindow {
            visible: true,
            plot_lines : vec![], // last index is wait_queue
            wait_queue: 1, // The init process is never spawned
            plot_size: 0,
            scale_min: 0,
            scale_max: 1,
            scale_max_max: 1,
        }
    }

    pub fn render(&mut self, model : &mut ModelState, ui: &Ui) {
        if self.visible {
            ui.window(im_str!("Machine Usage"))
            .size((324.0, 621.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                // Print message if no simulation data
                if model.num_machines == 0 {
                    ui.text(im_str!("No simulation data"));
                    return;
                }

                // Reset machine plots if machine numbers are not equal
                if self.plot_lines.len() != model.num_machines + 1 || model.packets.len() + 1 < self.plot_size {
                    println!("Reset plots");
                    self.reset_plots(model);
                }

                self.update_machine_usage(model);

                let prev_min = self.scale_min;
                //ui.slider_int(im_str!("Scale min"), &mut self.scale_min, 0, self.scale_max_max - 1).build();
                //ui.slider_int(im_str!("Scale max"), &mut self.scale_max, 1, self.scale_max_max).build();
                ui.range_slider_i32(im_str!("Scale"), &mut self.scale_min, &mut self.scale_max, 0, self.scale_max_max);
                ui.separator();

                // Push the other slide so start is never before end
                if self.scale_min >= self.scale_max {
                    if prev_min != self.scale_min {
                        self.scale_max = self.scale_min + 1;
                    } else {
                        self.scale_min = self.scale_max - 1;
                    }
                }

                // Render usage graphs
                for i in 0..model.num_machines+1 {
                    let title = if i < model.num_machines {
                        ImString::new(format!("machine-{}", i))
                    } else {
                        ImString::new(format!("wait_queue"))
                    };


                    let full_plot = self.plot_lines.get(i).unwrap();
                    let mut start_point = (model.start_time_plot * self.plot_size as f32) as usize;
                    let mut end_point = (model.end_time_plot * self.plot_size as f32) as usize;

                    if end_point > self.plot_size {
                        end_point = self.plot_size;
                    }
                    if start_point >= end_point {
                        start_point = end_point - 1;
                    }

                    ui.plot_lines(&title, &full_plot[start_point..end_point])
                    .scale_min(self.scale_min as f32).scale_max(self.scale_max as f32).build();
                }
            });
        }
    }


    fn reset_plots(&mut self, model : &mut ModelState) {
        self.plot_lines = vec![];
        self.wait_queue = 1;
        self.plot_size = 1;
        self.scale_min = 0;
        self.scale_max = model.max_queue_length as i32;
        self.scale_max_max = model.max_queue_length as i32;
        for i in 0..model.num_machines + 1 { // Extra 1 is for the wait_queue
            self.plot_lines.insert(i, vec![0.0]);
        }
    }

    fn extend_plot(&mut self) {
        let num_plots = self.plot_lines.len();
        for i in 0..num_plots {
            let plot = self.plot_lines.get_mut(i).unwrap();
            let last_element = plot.get(self.plot_size - 1).unwrap().clone();

            // Extend the maxium scale if there is an element bigger
            if last_element > self.scale_max_max as f32 {
                if self.scale_max == self.scale_max_max {
                    self.scale_max = last_element as i32;
                }
                self.scale_max_max = last_element as i32;
            }

            if i != num_plots - 1 {
                plot.insert(self.plot_size, last_element);
            } else {
                // The last plot is the wait queue
                plot.insert(self.plot_size, self.wait_queue as f32);
            }
        }
        self.plot_size += 1;
    }

    fn update_machine_usage(&mut self, model: &mut ModelState) {
        // Recalculate machine usage
        for packet_i in (self.plot_size - 1)..model.packets.len() {
            let packet = model.packets.get(packet_i).unwrap();
            self.extend_plot();

            if let PacketData::SimData(ref from_id, ref data) = packet.data {
                let vec = self.plot_lines.get_mut(from_id.clone()).unwrap();
                let last_element = vec.get_mut(self.plot_size - 1).unwrap();
                match data {
                    &SimData::ProcessStart(_) => {
                        *last_element += 1.0;
                        self.wait_queue -= 1; // Spawned process is no longer waiting
                    },
                    &SimData::ProcessEnd(_) => *last_element -= 1.0,
                    &SimData::ProcessSpawn(_) => self.wait_queue += 1,
                }
            }
        }
    }
}
