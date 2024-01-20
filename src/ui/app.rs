use std::borrow::BorrowMut;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use std::{fmt::Display, fs::File};

use eframe::glow::NONE;
use egui::epaint::tessellator::path;
use egui::{containers::Window, Context};
use egui::{Button, Color32, CursorIcon, Label, ScrollArea, Stroke, Ui};
use egui_code_editor::{CodeEditor, ColorTheme, Syntax};

use egui_file::FileDialog;
use egui_modal::Modal;
use env_logger::Logger;
use serde::de::value;
use simple_virtual_assembler::assembler::parsing_err::ParsingError;
use simple_virtual_assembler::components::connection;
use simple_virtual_assembler::vm::instruction::Instruction;
use simple_virtual_assembler::vm::virtual_machine::VirtualMachine;
use simple_virtual_assembler::{
    assembler::assembler::Assembler, components::connection::Connection,
};

use egui_notify::{Toast, Toasts};

use simple_virtual_assembler::language::Language;

use serde::{Deserialize, Serialize};

use serde_json;

use wasm_bindgen::prelude::*;
use web_sys::{js_sys::Array, *};

use crate::storage::connections_manager::{
    self, ConnectionManager, CONNECTION_NAMES, CURRENT_CONN_ID_FOR_RENAME, RELOAD_CONNECTION,
};
use crate::storage::custom_logger::CustomLogger;
use crate::storage::modals_manager::{
    ModalManager, MODAL_BUFFER_VALUE_I32, MODAL_INDEX_BUFFER, MODAL_TEXT_EDIT_BUFFER, RAM_ID,
};
use crate::storage::toasts::{ToastsManager, TOASTS};

use super::component_list_widget::{ComponentAction, ComponentListWidget};
use super::connection_widget::ConnectionWidget;
use super::help_window::HelpWindow;
use super::ram_window::RamWidow;
use super::sva_window::SVAWindow;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct SvaUI {
    language: Language,

    vms: Vec<SVAWindow>,

    ui_scale: f32,
    help_widow: HelpWindow,

    /// Custom logger
    #[serde(skip)]
    logger: CustomLogger,

    debug_mode: bool,

    #[serde(skip)]
    opened_file: Option<PathBuf>,
    #[serde(skip)]
    open_file_dialog: Option<FileDialog>,

    #[serde(skip)]
    save_file_dialog: Option<FileDialog>,
    #[serde(skip)]
    toasts: Toasts,

    connections_copy: Vec<Connection>,

    conn_names_copies: HashMap<usize, String>,

    rams: Vec<RamWidow>,

    connections_panel_visible: bool,

    components_panel_visible: bool,

    new_connection_name_buffer: String,

    change_conn_name_modal_open: bool,

    active_vms: HashMap<usize, bool>,

    active_rams: HashMap<usize, bool>,

    component_change_name_is_ram: Option<bool>,
    component_change_name_id: Option<usize>,
    component_change_name_buffer: String,
}

impl Default for SvaUI {
    fn default() -> Self {
        rust_i18n::set_locale("en");
        let sva_ui = Self {
            // Example stuff:
            // vm_shell: SVAShell::new(0, "First VM window".to_string()),
            language: Language::En,
            vms: Vec::new(),

            ui_scale: 1.25,
            help_widow: HelpWindow {
                is_open: false,
                language: Language::En,
            },

            logger: CustomLogger::new(),
            debug_mode: false,
            opened_file: None,
            open_file_dialog: None,
            save_file_dialog: None,
            toasts: Toasts::default(),
            connections_copy: Default::default(),
            conn_names_copies: HashMap::new(),
            rams: Vec::new(),
            connections_panel_visible: false,
            new_connection_name_buffer: String::new(),
            change_conn_name_modal_open: false,
            components_panel_visible: true,
            active_vms: HashMap::new(),
            component_change_name_is_ram: None,
            component_change_name_id: None,
            component_change_name_buffer: String::new(),
            active_rams: HashMap::new(),
        };
        sva_ui
    }
}

impl SvaUI {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        //rust_i18n::set_locale("en");
        if let Some(storage) = cc.storage {
            let mut sav_ui: SvaUI = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            sav_ui.set_connections_and_their_names();
            sav_ui.reconnect_vm_ports();
            sav_ui.reconnect_ram_ports();
            sav_ui.logger = CustomLogger::new();

            return sav_ui;
        }
        let mut sva_ui: SvaUI = Default::default();
        sva_ui.logger = CustomLogger::new();
        sva_ui
    }

    pub fn set_language(&mut self, language: Language) {
        rust_i18n::set_locale(language.string_code());
        CustomLogger::log("Changing language");
        match language {
            Language::Pl => {
                self.vms
                    .iter_mut()
                    .for_each(|vm| vm.set_language(Language::Pl));
                self.help_widow.set_language(Language::Pl);
            }
            Language::En => {
                self.vms
                    .iter_mut()
                    .for_each(|vm| vm.set_language(Language::En));
                self.help_widow.set_language(Language::Pl);
            }
        }
    }

    fn disconnect_vm_ports(&mut self) {
        for vm in self.vms.iter_mut() {
            for i in 0..4 {
                {
                    vm.vm.lock().unwrap().disconnect(i);
                }
            }
        }
    }

    fn disconnect_ram_ports(&mut self) {
        for ram in self.rams.iter_mut() {
            ram.ram.disconnect_data_port();
            ram.ram.disconnect_index_port();
            ram.ram.disconnect_mode_port();
        }
    }

    fn copy_connections_and_their_names(&mut self) {
        self.connections_copy = ConnectionManager::get_connections().lock().unwrap().clone();
        CustomLogger::log("Copying connections");
        CustomLogger::log(&format!("{:?}", self.connections_copy));
        CustomLogger::log("________________________________________");
        self.conn_names_copies = ConnectionManager::get_names();
    }

    fn set_connections_and_their_names(&mut self) {
        ConnectionManager::set_connection(self.connections_copy.clone());
        self.connections_copy.clear();
        ConnectionManager::set_names(self.conn_names_copies.clone());
        self.conn_names_copies.clear();
    }

    fn reconnect_vm_ports(&mut self) {
        let binding = ConnectionManager::get_connections();
        let mut connections = binding.lock().unwrap();
        for conn in connections.iter_mut() {
            let id_pairs = conn.get_connected_vms_and_ports('P');
            for (vm_id, port_index) in id_pairs {
                let x = self
                    .vms
                    .iter()
                    .find(|vm| vm.get_id() == TryInto::<usize>::try_into(vm_id).unwrap());
                if x.is_some() {
                    {
                        x.unwrap().vm.lock().unwrap().connect(port_index, conn);
                    }
                }
            }
        }
    }

    fn reconnect_ram_ports(&mut self) {
        let binding = ConnectionManager::get_connections();
        let mut connections = binding.lock().unwrap();
        for conn in connections.iter_mut() {
            let id_pairs = conn.get_connected_rams();
            for (ram_id, port_index) in id_pairs {
                let x = self.rams.iter_mut().find(|ram| ram.get_id() == ram_id);
                if x.is_some() {
                    if port_index == 0 {
                        x.unwrap().ram.connect_index_port(conn);
                    } else if port_index == 1 {
                        x.unwrap().ram.connect_data_port(conn);
                    } else if port_index == 2 {
                        x.unwrap().ram.connect_mode_port(conn);
                    }
                }
            }
        }
    }

    fn export_to_file(&mut self, path: String) {
        self.copy_connections_and_their_names();
        self.disconnect_vm_ports();
        self.disconnect_ram_ports();

        let serialized_state = serde_json::to_string(&self);
        self.set_connections_and_their_names();
        self.reconnect_vm_ports();
        self.reconnect_ram_ports();

        match serialized_state {
            Ok(data) => {
                let file = File::create(path).unwrap();
                let mut writer = BufWriter::new(file);
                // Write the data directly, without using serde_json::to_writer
                writer.write_all(data.as_bytes()).unwrap();
                writer.flush().unwrap();
            }
            Err(err) => {
                self.toasts
                    .info(t!("error.export.cant_serialize"))
                    .set_duration(Some(Duration::from_secs(10)));
            }
        };
    }

    fn import_file(&mut self, path: String) {
        let data = fs::read_to_string(path);
        match data {
            Ok(data) => {
                let json: Result<SvaUI, serde_json::Error> = serde_json::from_str(&data);
                match json {
                    Ok(sva_ui) => {
                        *self = sva_ui;
                        self.set_connections_and_their_names();
                        self.reconnect_vm_ports();
                        self.reconnect_ram_ports();
                    }
                    Err(err) => {
                        CustomLogger::log(&format!("{} \n {}", t!("error.import.bad_json"), err));
                        TOASTS
                            .lock()
                            .unwrap()
                            .error(t!("error.import.bad_json"))
                            .set_duration(Some(Duration::from_secs(10)));
                    }
                }
            }
            Err(err) => {
                CustomLogger::log(&format!("Could not open file \n {}", err));
                TOASTS
                    .lock()
                    .unwrap()
                    .error(t!("error.file.cant_open"))
                    .set_duration(Some(Duration::from_secs(10)));
            }
        }
    }

    // --------------------UI--------------------

    /// Shows debug window with logs and global variables
    fn show_debug_window(&mut self, ctx: &Context, ui: &mut Ui) {
        egui::Window::new(t!("window.debug"))
            .open(&mut self.debug_mode)
            .show(ctx, |ui| {
                ui.collapsing("variables", |ui| {
                    ui.label("Connection state");
                    ui.separator();
                    ui.label(format!("{:?}", CONNECTION_NAMES));
                    ui.separator();
                    ui.label(format!("{:?}", ConnectionManager::get_current_id_index()));
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("MODAL VALUE BUFFER (for setting ram)");
                        ui.label(format!("{:?}", *MODAL_BUFFER_VALUE_I32.lock().unwrap()));
                    });
                });
                ui.collapsing("logs", |ui| {
                    ScrollArea::vertical().max_height(600.0).show(ui, |ui| {
                        let logs = CustomLogger::get_logs_c();
                        if ui.button(t!("button.clear")).clicked() {
                            CustomLogger::clear_logs();
                        }
                        for log in logs.iter() {
                            ui.separator();
                            ui.label(log);
                        }
                    });
                });
            });
    }

    fn show_save_to_file_dialog(&mut self, ctx: &Context, ui: &mut Ui) {
        if let Some(dialog) = &mut self.save_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(file) = dialog.path() {
                    self.opened_file = Some(PathBuf::from(file));
                    CustomLogger::log(&format!("{:?}", self.opened_file));
                    self.export_to_file(
                        self.opened_file
                            .clone()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_owned(),
                    );
                }
            }
        }
    }
    fn show_import_file_dialog(&mut self, ctx: &Context, ui: &mut Ui) {
        // open file dialog
        if let Some(dialog) = &mut self.open_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(file) = dialog.path() {
                    self.opened_file = Some(PathBuf::from(file));
                    CustomLogger::log(&format!("{:?}", self.opened_file));
                    self.import_file(
                        self.opened_file
                            .clone()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_owned(),
                    );
                }
            }
        }
    }

    fn show_connection_name_change_modal(&mut self, ctx: &Context) -> Modal {
        let change_conn_name_modal = Modal::new(ctx, "change_conn_name_modal");
        change_conn_name_modal.show(|ui| {
            change_conn_name_modal.title(ui, "change name");

            ui.text_edit_singleline(&mut self.new_connection_name_buffer);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    self.change_conn_name_modal_open = false;
                    change_conn_name_modal.close();
                }
                if ui.button("Save").clicked() {
                    let id = *CURRENT_CONN_ID_FOR_RENAME.lock().unwrap();
                    ConnectionManager::set_name(id, self.new_connection_name_buffer.clone());

                    self.change_conn_name_modal_open = false;
                    change_conn_name_modal.close();
                }
            });
        });
        change_conn_name_modal
    }

    fn set_ram_value(&mut self, value: i32, index: usize) {
        let ram_id = *RAM_ID.lock().unwrap();
        if ram_id.clone().is_none() {
            return;
        }
        let ram_id = ram_id.unwrap();
        let ram = self.rams.iter_mut().find(|ram| ram.get_id() == ram_id);
        match ram {
            Some(ram) => {
                ram.set_value_at_index(index, value);
            }
            None => {}
        }
    }

    fn change_ram_name(&mut self, id: usize, name: String) {
        let res = self.rams.iter_mut().find(|ram| ram.get_id() == id);
        match res {
            Some(ram) => ram.set_name(name),
            None => ToastsManager::show_err("Could not change name of vm".to_owned(), 10),
        }
    }

    fn change_vm_name(&mut self, id: usize, name: String) {
        let res = self.vms.iter_mut().find(|vm| vm.get_id() == id);
        match res {
            Some(vm) => {
                vm.set_name(name);
                ToastsManager::show_info("vm namge chageed".to_owned(), 5);
            }
            None => ToastsManager::show_err("Could not change name of vm".to_owned(), 10),
        }
    }

    fn crate_component_change_name_modal(&mut self, ctx: &Context) {
        let change_component_name_modal = Modal::new(ctx, "change_component_name_modal");
        change_component_name_modal.show(|ui| {
            let mut buffer = &mut *MODAL_TEXT_EDIT_BUFFER.lock().unwrap();
            ui.text_edit_singleline(buffer);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    change_component_name_modal.close();
                    ModalManager::unset_current_modal();
                }
                if ui.button("Save").clicked() {
                    if self.component_change_name_is_ram == Some(true) {
                        self.change_ram_name(
                            self.component_change_name_id.unwrap(),
                            buffer.to_string(),
                        );
                    } else if self.component_change_name_is_ram == Some(false) {
                        self.change_vm_name(
                            self.component_change_name_id.unwrap(),
                            buffer.to_string(),
                        );
                    } else {
                    }
                    self.component_change_name_is_ram = None;
                    self.component_change_name_id = None;
                    change_component_name_modal.close();
                    ModalManager::unset_current_modal();
                }
            });
        });

        ModalManager::add_modal(2, change_component_name_modal);
    }

    fn create_ram_value_setter_modal(&mut self, ctx: &Context) {
        let set_ram_value_modal = Modal::new(ctx, "set_ram_value_modal");

        {
            *MODAL_BUFFER_VALUE_I32.lock().unwrap() = None;
        }

        set_ram_value_modal.show(|ui| {
            set_ram_value_modal.title(ui, "set value");
            let mut buffer = &mut *MODAL_TEXT_EDIT_BUFFER.lock().unwrap();

            ui.text_edit_singleline(buffer);

            let mut can_save = false;
            let res = buffer.parse::<i32>();
            match res {
                Ok(v) => {
                    *MODAL_BUFFER_VALUE_I32.lock().unwrap() = Some(v);
                    can_save = true;
                }
                Err(_) => {
                    *MODAL_BUFFER_VALUE_I32.lock().unwrap() = None;
                }
            }

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    //{ MODAL_TEXT_EDIT_BUFFER.lock().unwrap().clear();}
                    set_ram_value_modal.close();
                    ModalManager::unset_current_modal();
                }
                if can_save {
                    if ui.button("Save").clicked() {
                        //{
                        set_ram_value_modal.close();
                        ModalManager::unset_current_modal();
                        let x = MODAL_BUFFER_VALUE_I32.lock().unwrap().unwrap();
                        let y = MODAL_INDEX_BUFFER.lock().unwrap().unwrap();
                        self.set_ram_value(x, y);
                        //}
                        // MODAL_TEXT_EDIT_BUFFER.lock().unwrap().clear();
                    }
                }
            });
        });

        ModalManager::add_modal(1, set_ram_value_modal);
    }

    fn show_file_menu(&mut self, ui: &mut Ui) {
        ui.menu_button(format!("\u{1F4C1} {}", t!("menu.file")), |ui| {
            // clear button
            if ui.button(t!("button.clear")).clicked() {
                self.vms.clear();
                self.rams.clear();

                ConnectionManager::clear_connections();
            }
            // import button
            if ui.button(t!("menu.file.import")).clicked() {
                let mut dialog = FileDialog::open_file(self.opened_file.clone());
                dialog.open();
                self.open_file_dialog = Some(dialog);
            }
            // export button
            if ui.button(t!("menu.file.export")).clicked() {
                let mut dialog = FileDialog::save_file(self.opened_file.clone());
                dialog.open();
                self.save_file_dialog = Some(dialog);
            }
        });
        ui.add_space(16.0);
    }

    fn show_language_select(&mut self, ui: &mut Ui) {
        egui::ComboBox::from_label("")
            .selected_text(format!("{:?}", self.language.string_code()))
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(&mut self.language, Language::Pl, "Polski")
                    .changed()
                {
                    self.set_language(Language::Pl);
                }
                if ui
                    .selectable_value(&mut self.language, Language::En, "English")
                    .changed()
                {
                    self.set_language(Language::En);
                }
            });
    }

    fn show_component_add_menu(&mut self, ui: &mut Ui) {
        let max_height = 400.0 * (2.25 / self.ui_scale);
        ui.menu_button(t!("button.add"), |ui| {
            // vm
            if ui.button("vm").clicked() {
                let id = self.vms.last().map_or(0, |last| last.get_id() + 1);
                let mut x = SVAWindow::new(id, false, max_height);
                self.active_vms.insert(id, true);
                self.vms.push(x);
            }
            // vm with stack
            if ui.button("vm with stack").clicked() {
                let id = self.vms.last().map_or(0, |last| last.get_id() + 1);
                let mut x = SVAWindow::new(id, true, max_height);
                self.active_vms.insert(id, true);
                self.vms.push(x);
            }
            // ram module
            if ui.button("ram").clicked() {
                let id = self.rams.last().map_or(0, |last| last.get_id() + 1);
                self.active_rams.insert(id, true);
                self.rams.push(RamWidow::new(id));
            }
        });
    }

    fn show_components_side_panel(&mut self, ctx: &Context) {
        egui::SidePanel::right("components_panel")
            .resizable(true)
            .show(ctx, |ui| {
                let mut actions = vec![ComponentAction::DoNothing];
                ui.heading("Components");
                ui.label(format!("{:?}", actions));

                ui.collapsing("VMs", |ui| {
                    ScrollArea::new(true).show(ui, |ui| {
                        for vm in &self.vms {
                            let id = vm.get_id();
                            actions.push(
                                ComponentListWidget::new(
                                    id,
                                    vm.get_name(),
                                    true,
                                    None,
                                    true,
                                    vm.has_stack(),
                                )
                                .show(ctx, ui),
                            );
                        }
                    });
                });
                ui.collapsing("RAMs", |ui| {
                    for ram in &self.rams {
                        actions.push(
                            ComponentListWidget::new(
                                ram.get_id(),
                                ram.get_name(),
                                true,
                                None,
                                false,
                                false,
                            )
                            .show(ctx, ui),
                        );
                    }
                });
                ScrollArea::new(true).show(ui, |ui| {});
                for action in actions {
                    match action {
                        ComponentAction::DoNothing => {}
                        ComponentAction::ToggleVmVisibility(id) => {
                            if let Some(value) = self.active_vms.get_mut(&id) {
                                *value = !*value; // Toggle the boolean value
                            }
                        }
                        ComponentAction::RenameVm(id) => {
                            self.component_change_name_is_ram = Some(false);
                            self.component_change_name_id = Some(id);
                            ModalManager::set_modal(2);
                        }
                        ComponentAction::RemoveVm(id) => {
                            self.component_change_name_is_ram = Some(false);
                            self.component_change_name_id = Some(id);
                        }
                        ComponentAction::ToggleRamVisibility(id) => {
                            if let Some(value) = self.active_rams.get_mut(&id) {
                                *value = !*value; // Toggle the boolean value
                            }
                        }
                        ComponentAction::RenameRam(id) => {
                            self.component_change_name_is_ram = Some(true);
                            self.component_change_name_id = Some(id);
                            ModalManager::set_modal(2);
                        }
                        ComponentAction::RemoveRam(id) => {
                            self.component_change_name_is_ram = Some(true);
                            self.component_change_name_id = Some(id);
                        }
                    }
                }
            });
    }

    fn show_connections_side_panel(&mut self, ctx: &Context) {
        egui::SidePanel::right("connections_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Connections");
                ui.vertical(|ui| {
                    if ui.button("add").clicked() {
                        ConnectionManager::create_connection();
                    }
                    let mut disconnect_button_text = "disconnect";
                    if ConnectionManager::in_disconnect_mode() {
                        disconnect_button_text = "stop disconnecting";
                    }
                    if ui.button(disconnect_button_text).clicked() {
                        ConnectionManager::toggle_disconnect_mode();
                    }

                    if ui.button("stop connecting/disconnecting").clicked() {
                        ConnectionManager::set_current_id(None);
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.separator();

                    let conns = ConnectionManager::get_connections().lock().unwrap().clone();
                    for mut c in conns {
                        ConnectionWidget::new(c, &mut self.change_conn_name_modal_open)
                            .show(ctx, ui);
                    }
                });
            });
    }
}

impl eframe::App for SvaUI {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        CustomLogger::log("auto saving");
        self.copy_connections_and_their_names();
        self.disconnect_vm_ports();
        self.disconnect_ram_ports();
        eframe::set_value(storage, eframe::APP_KEY, self);
        self.reconnect_ram_ports();
        self.reconnect_vm_ports();
        self.set_connections_and_their_names();
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        //refreasing ram
        self.rams.iter_mut().for_each(|ram| ram.refresh());

        ctx.set_pixels_per_point(self.ui_scale);

        // reconnect connection after removal
        {
            let mut done_reconnecting = false;
            if *RELOAD_CONNECTION.lock().unwrap() == true {
                self.disconnect_ram_ports();
                self.disconnect_vm_ports();
                self.reconnect_ram_ports();
                self.reconnect_vm_ports();
                done_reconnecting = true;
            }
            if done_reconnecting {
                *RELOAD_CONNECTION.lock().unwrap() = true;
            }
        }

        // creating modals

        self.create_ram_value_setter_modal(ctx);
        self.crate_component_change_name_modal(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                egui::menu::bar(ui, |ui| {
                    #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
                    {
                        self.show_file_menu(ui);
                    }

                    egui::widgets::global_dark_light_mode_switch(ui);

                    ui.separator();
                    self.show_language_select(ui);
                    ui.separator();
                    // ui scale slider
                    if ui
                        .add(
                            egui::Slider::new(&mut self.ui_scale, 0.75..=2.25)
                                .step_by(0.25)
                                .text("ui scale"),
                        )
                        .changed()
                    {
                        let max_height = 400.0 * (2.25 / self.ui_scale);
                        for vm in self.vms.iter_mut() {
                            vm.set_max_height(max_height);
                        }
                    }

                    ui.separator();

                    self.show_component_add_menu(ui);

                    if ui.button(t!("label.help")).clicked() {
                        self.help_widow.toggle_open_close();
                    }

                    if ui.button("Debug").clicked() {
                        self.debug_mode = !self.debug_mode;
                    }
                    if ui
                        .button("connections")
                        .on_hover_text("opens panel with connections")
                        .clicked()
                    {
                        self.connections_panel_visible = !self.connections_panel_visible;
                    }
                    if ui
                        .button("components")
                        .on_hover_text("opens panel with components")
                        .clicked()
                    {
                        self.components_panel_visible = !self.components_panel_visible;
                    }
                });
            });
        });

        if self.components_panel_visible {
            self.show_components_side_panel(ctx);
        }

        if self.connections_panel_visible {
            self.show_connections_side_panel(ctx);
        }

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            // vms
            for index in 0..self.vms.len() {
                let vm = &mut self.vms[index];

                let active = *self.active_vms.get(&vm.get_id()).unwrap_or(&false);

                if active {
                    vm.show(ctx, ui);
                }
            }
            // rams
            for index in 0..self.rams.len() {
                let ram = &mut self.rams[index];
                let active = *self.active_rams.get(&ram.get_id()).unwrap_or(&false);
                if active {
                    ram.show(ctx, ui);
                }
            }

            // powered by
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });

            // file dialogs
            self.show_save_to_file_dialog(ctx, ui);

            self.show_import_file_dialog(ctx, ui);

            //help window
            self.help_widow.show(ctx, ui);

            // debug window
            self.show_debug_window(ctx, ui);

            // notifications
            self.toasts.show(ctx);
            {
                TOASTS.lock().unwrap().show(ctx);
            }
        }); // Central panel

        // Modal for changing connection name
        let change_conn_name_modal = self.show_connection_name_change_modal(ctx);

        if self.change_conn_name_modal_open {
            change_conn_name_modal.open();
        }

        // Other Modals

        ModalManager::open_modal();
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
