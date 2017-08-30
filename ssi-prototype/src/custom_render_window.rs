use imgui::*;
use imgui_sys::*;

use ModelState;

pub struct CustomRenderWindow {
    pub visible: bool,
    pub min : f32,
    pub max : f32,
}

impl CustomRenderWindow {
    pub fn new() -> Self {
        CustomRenderWindow {
            visible: true,
            min: 0.2,
            max: 0.8,
        }
    }

    pub fn render(&mut self, model : &mut ModelState, ui: &Ui) {
        if self.visible {
            ui.window(im_str!("Custom Render"))
            .size((324.0, 621.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                ui.text(im_str!("Primatives"));

                let mut mouse_coord = ImVec2::zero();
                unsafe {
                    igGetMousePos(&mut mouse_coord);
                }

                ui.text(im_str!("mouse_coord: ({}, {})", mouse_coord.x, mouse_coord.y));

                CustomRenderWindow::draw_slider(&mut self.min, &mut self.max);

                ui.text(im_str!("END"));

                /*self.min += 0.01;
                if self.min > 0.99 {
                    self.min = 0.0;
                }*/
            });
        }
    }

    pub fn draw_slider(min: &mut f32, max: &mut f32) {
        // Constants
        let col32_bkg_gray = get_colour(87, 87, 87, 1.0);
        let col32_frg_gray = get_colour(138, 138, 138, 1.0);
        let bar_height = 20.0;
        let slider_margin = 2.0;
        let slider_width = 14.0; // min_width of each half of the slider

        // Get needed information for drawing
        let mut cursor_location = ImVec2::zero();
        let window_width;
        let draw_list;
        unsafe {
            igGetCursorScreenPos(&mut cursor_location);
            window_width = igGetWindowWidth() - 14.0; // Padding of 7 pixels either side
            draw_list = igGetWindowDrawList();
        }
        let slider_size = ImVec2::new(window_width, bar_height);


        // Bar background
        let bar_start = cursor_location.clone();
        let mut bar_end = bar_start.clone();
        bar_end.x += window_width;
        bar_end.y += bar_height;

        // Slider end points
        let left_end = bar_start.x + slider_margin;
        let right_end = bar_end.x - slider_margin;

        // Left Slider
        let mut left_slider = cursor_location.clone();
        left_slider.x += slider_width - slider_margin;
        left_slider.y += slider_margin;

        left_slider.x += (right_end - left_slider.x - slider_width - slider_margin) * *min; // Apply percentage
        left_slider.x -= slider_width - 2.0 * slider_margin;

        // Right Slider
        let mut right_slider = cursor_location.clone();
        right_slider.x = right_end - slider_width + slider_margin;
        right_slider.y += bar_height - slider_margin;

        right_slider.x -= (right_slider.x - left_end - slider_width - slider_margin) * (1.0 - *max);
        right_slider.x += slider_width - 2.0 * slider_margin;

        // Add to the draw list
        unsafe {
            ImDrawList_AddRectFilled(draw_list, bar_start, bar_end, col32_bkg_gray, 0.0, 0);
            ImDrawList_AddRectFilled(draw_list, left_slider, right_slider, col32_frg_gray, 5.0, 15); // 5.0 and 15 are for rounding all the corners
            igDummy(&slider_size);
        }

    }
}

pub fn get_colour(r: u32, g: u32, b: u32, n_a: f32) -> ImU32 {
    assert!(r <= 255 && g <= 255 && b <= 255);
    let n_r = r as f32 / 255.0;
    let n_g = g as f32 / 255.0;
    let n_b = b as f32 / 255.0;
    let colour = ImVec4::new(n_r, n_g, n_b, n_a);

    let col32;
    unsafe {
        col32 = igColorConvertFloat4ToU32(colour);
    }
    col32
}
