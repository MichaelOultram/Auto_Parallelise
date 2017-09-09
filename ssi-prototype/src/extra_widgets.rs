use std::ffi::CString;
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

                ui.range_slider_f32(&"Slider".to_string(), &mut self.min, &mut self.max, 0.0, 1.0);
                ui.text(im_str!("END"));

            });
        }
    }
}

pub trait UiExtras<'ui> {
    fn range_slider_f32<'p>(&self, label: &str, min: &mut f32, max: &mut f32, min_limit: f32, max_limit: f32);
    fn range_slider_i32<'p>(&self, label: &str, min: &mut i32, max: &mut i32, min_limit: i32, max_limit: i32);
}

impl<'ui> UiExtras<'ui> for Ui<'ui> {
    fn range_slider_f32<'p>(&self, label: &str, min_value: &mut f32, max_value: &mut f32, min_limit: f32, max_limit: f32) {
        let store = unsafe {
            igGetStateStorage()
        };

        // Constants
        let col32_bkg_gray = get_colour(87, 87, 87, 1.0);
        let col32_frg_gray = get_colour(138, 138, 138, 1.0);
        let bar_height = 20.0;
        let slider_margin = 2.0;
        let slider_width = 25.0; // min_width of each half of the slider

        // Slider percentage calculations
        let limit_range: f32 = max_limit - min_limit;
        let min_percent: f32 = (*min_value - min_limit) / limit_range;
        let max_percent: f32 = (*max_value - min_limit) / limit_range;
        println!("min: {}, max: {}", min_percent, max_percent);

        // Get needed information for drawing
        let mut cursor_location = ImVec2::zero();
        let window_width;
        let draw_list;
        let mouse_down;
        let mut mouse = ImVec2::zero();
        unsafe {
            igGetCursorScreenPos(&mut cursor_location);
            window_width = igGetWindowWidth() - 14.0; // Padding of 7 pixels either side
            draw_list = igGetWindowDrawList();
            mouse_down = igIsMouseDown(0);
            igGetMousePos(&mut mouse);
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

        left_slider.x += (right_end - left_slider.x - slider_width - slider_margin) * min_percent; // Apply percentage
        let mut left_slider_end = left_slider.clone();
        left_slider.x -= slider_width - 2.0 * slider_margin;

        // Right Slider
        let mut right_slider = cursor_location.clone();
        right_slider.x = right_end - slider_width + slider_margin;
        right_slider.y += bar_height - slider_margin;

        right_slider.x -= (right_slider.x - left_end - slider_width - slider_margin) * (1.0 - max_percent);
        let mut right_slider_end = right_slider.clone();
        right_slider.x += slider_width - 2.0 * slider_margin;

        // Buffer for mouse pointer
        left_slider_end.y += bar_height - slider_margin;
        right_slider_end.y -= bar_height - slider_margin;

        let (section_id, mut section_down); // -1 - Waiting for click, 0 - None, 1 - Left, 2 - Middle, 3 - Right
        let (click_pos_id, mut click_pos);
        unsafe {
            let section_s = CString::new(format!("{}-section", label)).unwrap();
            section_id = igGetIdStr(section_s.as_ptr());
            section_down = ImGuiStorage_GetInt(store, section_id, -1);

            let click_pos_s = CString::new(format!("{}-click_pos", label)).unwrap();
            click_pos_id = igGetIdStr(click_pos_s.as_ptr());
            click_pos = ImGuiStorage_GetFloat(store, click_pos_id, 0.0);
        }

        // Work out which section should move
        if mouse_down && section_down == -1 {
            if isMouseInBounds(left_slider, left_slider_end) {
                section_down = 1; // Left
            } else if isMouseInBounds(left_slider_end, right_slider_end) {
                section_down = 2; // Middle
            } else if isMouseInBounds(right_slider, right_slider_end) {
                section_down = 3; // Right
            } else {
                section_down = 0; // Nothing
            }
            click_pos = mouse.x;
        } else if !mouse_down {
            section_down = -1; // Wait for click
            click_pos = 0.0;
        }
        println!("section_down: {}, click_pos: {}", section_down, click_pos);

        // Caculate new percentage
        let bar_length = bar_end.x - bar_start.x - 2.0 * slider_margin;
        let mut percentage = (mouse.x - bar_start.x - slider_margin) / bar_length;
        if percentage < 0.0 {
            percentage = 0.0;
        } else if percentage > 1.0 {
            percentage = 1.0;
        }


        // Move the slider to the new percentage
        match section_down {
            // Left
            1 => {
                *min_value = min_limit + percentage * limit_range;
                if max_value < min_value {
                    *min_value = *max_value;
                }
            },

            // Middle
            2 => {},

            // Right
            3 => {
                *max_value = min_limit + percentage * limit_range;
                if max_value < min_value {
                    *max_value = *min_value;
                }
            },

            // Nothing
            _ => {},
        }


        unsafe {
            // Save section_down, click_pos
            ImGuiStorage_SetInt(store, section_id, section_down);
            ImGuiStorage_SetFloat(store, click_pos_id, click_pos);

            // Add to the draw list
            ImDrawList_AddRectFilled(draw_list, bar_start, bar_end, col32_bkg_gray, 0.0, 0);
            ImDrawList_AddRectFilled(draw_list, left_slider, right_slider, col32_frg_gray, 5.0, 15); // 5.0 and 15 are for rounding all the corners
            igInvisibleButton(&0, slider_size);
        }
    }

    fn range_slider_i32<'p>(&self, label: &str, min_value: &mut i32, max_value: &mut i32, min_limit: i32, max_limit: i32) {
        let mut min_value_f32 = *min_value as f32;
        let mut max_value_f32 = *max_value as f32;
        let min_limit_f32 = min_limit as f32;
        let max_limit_f32 = max_limit as f32;
        self.range_slider_f32(label, &mut min_value_f32, &mut max_value_f32, min_limit_f32, max_limit_f32);
        *min_value = min_value_f32 as i32;
        *max_value = max_value_f32 as i32;
    }
}

fn isMouseInBounds(a: ImVec2, b: ImVec2) -> bool {
    let mut mouse = ImVec2::zero();
    unsafe {
        igGetMousePos(&mut mouse);
    }

    let (min_x, max_x) = min_max(a.x, b.x);
    let (min_y, max_y) = min_max(a.y, b.y);
    //println!("{} <= {} <= {} && {} <= {} <= {}", min_x, mouse.x, max_x, min_y, mouse.y, max_y);

    min_x <= mouse.x && mouse.x <= max_x &&
    min_y <= mouse.y && mouse.y <= max_y
}

fn min_max<T: PartialOrd>(a: T, b: T) -> (T, T) {
    if a < b {
        (a, b)
    } else {
        (b, a)
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
