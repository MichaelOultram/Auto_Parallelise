use imgui::*;

use ModelState;
use machine::*;
use router::*;

pub struct RawPacketWindow {
    pub show_vector_clock : bool,
    pub show_packet_type : bool,
}

impl RawPacketWindow {
    pub fn new() -> Self {
        RawPacketWindow {
            show_vector_clock: false,
            show_packet_type: true,
        }
    }

    pub fn render(&mut self, model : &mut ModelState, ui: &Ui) {
        ui.window(im_str!("Raw Packet Viewer"))
            .size((324.0, 621.0), ImGuiSetCond_FirstUseEver)
            .build(|| {
                ui.checkbox(im_str!("show vector clock"), &mut self.show_vector_clock);
                ui.checkbox(im_str!("show packet type"), &mut self.show_packet_type);
                if model.packets.len() > 0 {
                    for packet in &model.packets {
                        let title = self.render_packet(packet);
                        ui.tree_node(&ImString::new(title)).opened(false, ImGuiSetCond_FirstUseEver).build(|| {});
                    }
                } else {
                    ui.tree_node(im_str!("No simulation data")).opened(false, ImGuiSetCond_FirstUseEver).build(|| {});
                }
            });
    }

    fn render_packet(&self, packet: &Packet) -> String {
        let packet_type: String;
        let packet_body: String;
        match packet.data {
            PacketData::NetData(ref to_id, ref data) => {
                packet_type = "NET".to_string();
                packet_body = match data {
                    &NetData::Request(consumer, hops) => format!("[?->{}] Request for {} ({} hop)", to_id, consumer, hops),
                    &NetData::Reply(ref process) => format!("[?->{}] Reply giving {}", to_id, process.name),
                    &NetData::Terminate => format!("[?->{}] Terminate", to_id),
                };
            },
            PacketData::SimData(ref from_id, ref data) => {
                packet_type = "SIM".to_string();
                packet_body = match data {
                    &SimData::ProcessStart(ref process_name) => format!("{} Start {}", from_id, process_name),
                    &SimData::ProcessEnd(ref process_name) => format!("{} End {}", from_id, process_name),
                    &SimData::ProcessSpawn(ref process_name) => format!("{} Spawn {}", from_id, process_name),
                }
            },
        };

        let mut output = "".to_string();
        if self.show_vector_clock {
            output.push_str(&format!("{:?} ", packet.vector_clock));
        }
        if self.show_packet_type {
            output.push_str(&format!("{}: ", packet_type));
        }
        output.push_str(&packet_body);
        output
    }
}
