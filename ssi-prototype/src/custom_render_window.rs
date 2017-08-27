use imgui::*;
use imgui_sys::*;

use ModelState;

pub struct CustomRenderWindow {
    pub visible: bool,
}

impl CustomRenderWindow {
    pub fn new() -> Self {
        CustomRenderWindow {
            visible: true,
        }
    }

    pub fn render(&mut self, model : &mut ModelState, ui: &Ui) {
        if self.visible {
            ui.window(im_str!("Custom Render"))
            .size((324.0, 621.0), ImGuiSetCond_FirstUseEver)
            .build(|| {

                ui.text(im_str!("Primatives"));

                let mut p = ImVec2::zero();
                let colour = ImVec4::new(1.0, 1.0, 0.4, 1.0);
                unsafe {
                    igGetCursorScreenPos(&mut p);
                }

                p.x += 10.0;
                p.y += 10.0;

                unsafe {
                    let draw_list = igGetWindowDrawList();
                    let col32 = igColorConvertFloat4ToU32(colour);
                    ImDrawList_AddCircleFilled(draw_list, p, 18.0, col32, 32);
                }

            });
        }
    }
}
