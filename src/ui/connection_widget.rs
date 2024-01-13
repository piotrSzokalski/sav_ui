use egui::{Context, Ui};
use egui_modal::Modal;
use simple_virtual_assembler::components::connection::Connection;

use crate::storage::{
    connections_manager::{
        ConnectionManager, CONNECTIONS, CURRENT_CONN_ID_FOR_RENAME, NEW_CONNECTION_NAME_BUFFER,
    },
    custom_logger::CustomLogger,
};

pub struct ConnectionWidget<'a> {
    conn: Connection,
    change_conn_name_modal_open: &'a mut bool,
}

impl<'a> ConnectionWidget<'a> {
    pub fn new(conn: Connection, change_conn_name_modal_open: &'a mut bool) -> Self {
        Self {
            conn,
            change_conn_name_modal_open,
        }
    }

    pub fn show(&mut self, ctx: &Context, ui: &mut Ui) {
        let id = self.conn.get_id();

        let mut name = "".to_owned();
        if let Some(id_c) = id {
            if let Some(res) = ConnectionManager::get_name(id_c) {
                name = res;
            }
        }

        ui.separator();

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{:?}", id));
                ui.separator();
                ui.label(name);
            });
            let button_text = "connect".to_owned();
            // it's an index

            let current_index = ConnectionManager::get_current_id();
            if ui.button(button_text).clicked() {
                CustomLogger::log(&format!("CONN IDS: {:?} {:?}", current_index, id));
                if current_index == id {
                    ConnectionManager::set_current_id(None);
                    CustomLogger::log("HERE");
                } else {
                    ConnectionManager::set_current_id(id);
                }
            }
            if ui.button("rename").clicked() {
                *CURRENT_CONN_ID_FOR_RENAME.lock().unwrap() = id;
                *self.change_conn_name_modal_open = true;
            }
            if ui.button("remove").clicked() {
                ConnectionManager::remove_connection(id);
            }
        });
        let collapsing_id = ui.make_persistent_id(self.conn.get_id());
        egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            collapsing_id,
            false,
        )
        .show_header(ui, |ui| {
            ui.label(t!("ports"));
        })
        .body(|ui| {
            ui.label(format!("{:?}", self.conn));
        });
    }
}
