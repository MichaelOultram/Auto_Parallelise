use imgui::*;

use support::RenderStats;

pub struct UIPerformanceWindow {
    pub visible: bool,
}

impl UIPerformanceWindow {
    pub fn new() -> Self {
        UIPerformanceWindow {
            visible: true,
        }
    }

    pub fn render(&mut self, render_stats : RenderStats, ui: &Ui) {
        if self.visible {
            ui.window(im_str!("UI Performance"))
            .size((324.0, 621.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                ui.text(im_str!("{} FPS, {} ms", render_stats.frames_per_second as u32, render_stats.frame_time as u32));
            });
        }
    }
}
