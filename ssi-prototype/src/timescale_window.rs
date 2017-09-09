use imgui::*;

use ModelState;
use extra_widgets::UiExtras;

pub struct TimescaleWindow {
    pub visible: bool,
}

impl TimescaleWindow {
    pub fn new() -> Self {
        TimescaleWindow {
            visible: true,
        }
    }

    pub fn render(&mut self, model : &mut ModelState, ui: &Ui) {
        if self.visible {
            ui.window(im_str!("Timescale"))
            .size((324.0, 621.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                let prev_start = model.start_time_plot;
                ui.slider_float(im_str!("start time"), &mut model.start_time_plot, 0.0, 0.999).build();
                ui.slider_float(im_str!("end time"), &mut model.end_time_plot, 0.001, 1.0).build();

                ui.range_slider(im_str!("Slider"), &mut model.start_time_plot, &mut model.end_time_plot, 0.0, 1.0);


                // Push the other slide so start is never before end
                if model.start_time_plot >= model.end_time_plot {
                    if prev_start != model.start_time_plot {
                        model.end_time_plot = model.start_time_plot + 0.001;
                    } else {
                        model.start_time_plot = model.end_time_plot - 0.001;
                    }
                }
            });
        }
    }
}
