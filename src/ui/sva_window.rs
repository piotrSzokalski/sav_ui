use eframe::glow::NONE;
use egui::Button;
use egui::Color32;
use egui::Stroke;
use simple_virtual_assembler::vm::flag::Flag;
use std::cell::Ref;
use std::cell::RefCell;
use std::collections::btree_map::Range;
use std::default;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use egui::{containers::Window, widgets::Label, Context};
use egui::{Align, Slider, TextEdit, Ui, Widget};

use simple_virtual_assembler::assembler::parsing_err::ParsingError;
use simple_virtual_assembler::components::connection::Connection;
use simple_virtual_assembler::components::port::Port;
use simple_virtual_assembler::vm::instruction::Instruction;
use simple_virtual_assembler::vm::virtual_machine::{VirtualMachine, VmStatus};

use egui_code_editor::{CodeEditor, ColorTheme, Syntax};

use simple_virtual_assembler::assembler::assembler::Assembler;

use simple_virtual_assembler::language::Language;

use crate::storage::connections_manager::ConnectionManager;
use crate::storage::custom_logger::CustomLogger;
use crate::storage::toasts::TOASTS;

use super::indicator_widget::IndicatorWidget;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SVAWindow {
    /// Id
    id: i32,
    /// Tile
    title: String,

    /// Simple virtual machine
    pub vm: Arc<Mutex<VirtualMachine>>,
    /// Assembler for simple virtual machine
    assembler: Assembler,
    /// Code before assembly
    code: String,
    /// ( Currently useless ) Program to be executed by vm
    program: Vec<Instruction>,
    /// Error assembling code to program
    parsing_error: Option<ParsingError>,
    /// ( Currently useless ) Parsing error message
    parsing_error_msg: String,
    /// Language
    language: Language,
    /// 'Start' or 'Stop' text for button
    control_button_text: String,

    //  --------------------------------------RCs--------------------------------------
    /// Connecting in progress

    /// Connections

    /// Disconnect port on clikc

    //  -------------------------------------------------------------------------------
    ///Delay ms
    delay_ms: u64,

    port_colors: [Color32; 4],

    vm_state: (i32, usize, Flag, [i32; 4], [i32; 4], VmStatus, u32),

    vm_state_previous: (i32, usize, Flag, [i32; 4], [i32; 4], VmStatus, u32),
    #[serde(skip)]
    indicators: [IndicatorWidget; 13],

    conn_ids: [Option<usize>; 4],

    stack_present: bool,

    stack_data: Vec<i32>,
    #[serde(skip)]
    stack_indicators: Vec<IndicatorWidget>,
}

impl Default for SVAWindow {
    fn default() -> Self {
        let stack_indicators = (0..32)
            .collect::<Vec<i32>>()
            .iter()
            .map(|index| IndicatorWidget::new("".to_owned()))
            .collect();
        Self {
            id: -1,
            title: "BRAK".to_owned(),
            vm: Arc::new(Mutex::new(VirtualMachine::new())),
            assembler: Assembler::new(),
            code: String::new(),
            program: Vec::new(),

            parsing_error_msg: String::new(),
            parsing_error: None,
            language: Language::En,
            control_button_text: "Start".to_owned(),

            delay_ms: 1000,

            port_colors: [Color32::GRAY, Color32::GRAY, Color32::GRAY, Color32::GRAY],
            vm_state: (0, 0, Flag::EQUAL, [0; 4], [0; 4], VmStatus::Initial, 0),
            vm_state_previous: (0, 0, Flag::EQUAL, [0; 4], [0; 4], VmStatus::Initial, 0),
            indicators: [
                IndicatorWidget::new("acc".to_owned()),
                IndicatorWidget::new("pc".to_owned()),
                IndicatorWidget::new("flag".to_owned()),
                IndicatorWidget::new("r0".to_owned()),
                IndicatorWidget::new("r1".to_owned()),
                IndicatorWidget::new("r2".to_owned()),
                IndicatorWidget::new("r3".to_owned()),
                IndicatorWidget::new("p0".to_owned()),
                IndicatorWidget::new("p1".to_owned()),
                IndicatorWidget::new("p2".to_owned()),
                IndicatorWidget::new("p3".to_owned()),
                IndicatorWidget::new("status".to_owned()),
                IndicatorWidget::new("delay".to_owned()),
            ],
            conn_ids: [None; 4],
            stack_present: false,
            stack_data: Vec::new(),
            stack_indicators,
        }
    }
}

impl SVAWindow {
    pub fn new(id: i32, title: String, stack_present: bool) -> SVAWindow {
        let stack_indicators = (0..32)
            .collect::<Vec<i32>>()
            .iter()
            .map(|index| IndicatorWidget::new("".to_owned()))
            .collect();
        let mut s = SVAWindow {
            id,
            title,
            vm: Arc::new(Mutex::new(VirtualMachine::new())),
            assembler: Assembler::new(),
            code: String::new(),
            program: Vec::new(),

            parsing_error_msg: String::new(),
            parsing_error: None,
            language: Language::En,
            control_button_text: "Start".to_owned(),

            delay_ms: 1000,

            port_colors: [Color32::GRAY, Color32::GRAY, Color32::GRAY, Color32::GRAY],
            vm_state: (0, 0, Flag::EQUAL, [0; 4], [0; 4], VmStatus::Initial, 0),
            vm_state_previous: (0, 0, Flag::EQUAL, [0; 4], [0; 4], VmStatus::Initial, 0),
            indicators: Default::default(),
            conn_ids: [None; 4],
            stack_present,
            stack_data: Vec::new(),
            stack_indicators: stack_indicators,
        };
        if stack_present {
            s.assembler = Assembler::new().with_stack();
            s.vm = Arc::new(Mutex::new(VirtualMachine::new().with_stack(32)))
        }
        s.vm.lock().unwrap().set_delay(1000);
        s
    }

    /// Sets filds that are Rc to new instances to presist state between seralzianon and deseralziaon
    pub fn set_refs(&mut self) {}

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn set_language(&mut self, language: Language) {
        self.assembler.set_language(language);
    }

    pub fn show_stack(&mut self, ctx: &Context, ui: &mut Ui) {
        if !self.stack_present {
            return;
        }
        ui.collapsing("stac", |ui| {
            egui::ScrollArea::horizontal()
                .max_width(200.0)
                .enable_scrolling(true)
                .show(ui, |ui| {
                    ui.separator();
                    ui.horizontal(|ui| {
                        let mut indicator_index = 0;
                        for (index, item) in self.stack_data.iter().enumerate().rev() {
                            //ui.label(format!("{}", item));

                            //let indicator_data = 0;
                            ui.button(&item.to_string());
                            //println!("{}", index);
                            //self.stack_indicators[indicator_index].set(*item, "").show(ctx, ui);
                            indicator_index += 1;
                        }
                    });
                    ui.add_space(10.0);
                });
        });
    }

    // runs each frame
    pub fn show(&mut self, ctx: &Context, ui: &mut Ui) {
        {
            self.vm_state_previous = self.vm_state;

            match self.vm.lock() {
                Ok(vm) => {
                    self.vm_state = vm.get_state_for_display();
                    if self.stack_present {
                        self.stack_data = vm.get_stack();
                    }
                }
                Err(err) => CustomLogger::log(&format!("{:?}", err)),
            }
        }
        let (acc, pc, flag, r, p, vm_status, delay) = self.vm_state;

        egui::Window::new(&self.id.to_string())
            .open(&mut true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(600.0)
                    .show(ui, |ui| {
                        // setting vm delay
                        if ui
                            .add(egui::Slider::new(&mut self.delay_ms, 0..=5000).text("delay"))
                            .changed()
                        {
                            self.vm
                                .lock()
                                .unwrap()
                                .set_delay(self.delay_ms.try_into().unwrap());
                        }

                        self.show_vm_controll_buttons(ui, vm_status);

                        self.show_code_editor(ui);

                        self.show_registers(ui, acc, ctx, pc, flag, r);

                        self.show_ports(ui);

                        self.show_stack(ctx, ui);
                    });

                if self.delay_ms > 10 {
                    ctx.request_repaint_after(Duration::from_millis(self.delay_ms));
                } else {
                    ctx.request_repaint_after(Duration::from_millis(10));
                }
            });
    }

    fn show_ports(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label("p 0-3");
            let ports;

            {
                ports = self.vm.lock().unwrap().get_ports();
            }
            let mut index = 0;
            for p in ports {
                let port_button = Button::new(format!("{:?}", p)).stroke(Stroke::new(
                    4.0,
                    *self.port_colors.get(index).unwrap_or(&Color32::LIGHT_GRAY),
                ));
                if ui.button(format!("{:?}", p)).clicked() {
                    if let Some(conn_index) = ConnectionManager::get_current_id_index() {
                        if let Some(conn) = ConnectionManager::get_connections()
                            .lock()
                            .unwrap()
                            .get_mut(conn_index)
                        {
                            let id = self.id.to_string() + "P" + &index.to_string();
                            self.vm.lock().unwrap().connect_with_id(index, conn, id);
                        }
                    } else if ConnectionManager::in_disconnect_mode() {
                        self.vm.lock().unwrap().disconnect(index);
                    }
                }

                if index < 4 {
                    index += 1;
                }
            }
        });
    }

    fn show_registers(
        &mut self,
        ui: &mut Ui,
        acc: i32,
        ctx: &Context,
        pc: usize,
        flag: Flag,
        r: [i32; 4],
    ) {
        let labels = ["acc", "pc", "flag", "r:0-3", "p:0-3", "status", "delay"];

        ui.horizontal(|ui| {
            self.indicators[0].set(acc, "acc").show(ctx, ui);
            self.indicators[1]
                .set(pc.try_into().unwrap_or(0), "pc")
                .show(ctx, ui);

            // flag
            ui.label("flag");
            ui.button(flag.to_string());

            self.indicators[2].set(r[0], "r0").show(ctx, ui);
            self.indicators[2].set(r[1], "r1").show(ctx, ui);
            self.indicators[2].set(r[2], "r2").show(ctx, ui);
            self.indicators[2].set(r[3], "r3").show(ctx, ui);
        });
    }

    fn show_code_editor(&mut self, ui: &mut Ui) {
        let id = ui.make_persistent_id("Vm code heder");
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.heading(t!("sva_shell.code_block"));
            })
            .body(|ui| {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .max_width(400.0)
                    .show(ui, |ui| {
                        CodeEditor::default()
                            .id_source("code editor")
                            .with_rows(12)
                            .with_fontsize(14.0)
                            .with_theme(ColorTheme::GRUVBOX)
                            .with_syntax(Syntax::rust())
                            .with_numlines(true)
                            .show(ui, &mut self.code);
                    });
            });
        //FIXME: should do this always
        self.try_assemble_and_load();
    }

    fn show_vm_controll_buttons(&mut self, ui: &mut Ui, vm_status: VmStatus) {
        if let Some(parsing_error) = &self.parsing_error {
            ui.label(
                egui::RichText::new(parsing_error.to_string())
                    .color(egui::Color32::from_rgb(255, 0, 0)),
            );
        } else {
            ui.horizontal(|ui| {
                if vm_status == VmStatus::Running {
                    self.control_button_text = t!("sva_shell.button.stop").to_owned();
                }

                match vm_status {
                    VmStatus::Initial => {
                        self.control_button_text = t!("sva_shell.button.start").to_owned()
                    }
                    VmStatus::Running => {
                        self.control_button_text = t!("sva_shell.button.stop").to_owned()
                    }
                    VmStatus::Stopped => {
                        self.control_button_text = t!("sva_shell.button.resume").to_owned()
                    }
                    VmStatus::Finished => {
                        self.control_button_text = t!("sva_shell.button.start").to_owned()
                    }
                }

                if vm_status == VmStatus::Running || vm_status == VmStatus::Stopped {
                    if ui.button(t!("sva_shell.button.halt")).clicked() {
                        VirtualMachine::halt(self.vm.clone());
                    }
                }

                if ui.button(&self.control_button_text).clicked() {
                    {
                        self.vm
                            .lock()
                            .unwrap()
                            .set_delay(self.delay_ms.try_into().unwrap());
                    }
                    match vm_status {
                        VmStatus::Initial => {
                            VirtualMachine::start(self.vm.clone());
                        }
                        VmStatus::Running => {
                            VirtualMachine::stop(self.vm.clone());
                        }
                        VmStatus::Stopped => {
                            VirtualMachine::resume(self.vm.clone());
                        }
                        VmStatus::Finished => {
                            VirtualMachine::start(self.vm.clone());
                        }
                    }
                }

                if ui.button(t!("sva_shell.button.step")).clicked() {
                    self.step();
                }
                if ui.button(t!("sva_shell.button.reset")).clicked() {
                    self.vm.lock().unwrap().clear_registers();
                    match self.vm.lock() {
                        Ok(mut vm) => {
                            vm.clear_registers();
                        }
                        Err(err) => {
                            TOASTS
                                .lock()
                                .unwrap()
                                .error("could clear register")
                                .set_duration(Some(Duration::from_secs(5)));
                        }
                    }
                }
            });
        }
    }

    /// Tries Assembles code to instructions and loads them to vm
    pub fn try_assemble_and_load(&mut self) {
        let res = self.assembler.parse(&self.code);

        match res {
            Ok(program) => {
                //TODO:
                //temp
                //self.vm = Arc::new(Mutex::new(VirtualMachine::new_with_program(program)));
                {
                    match self.vm.lock() {
                        Ok(mut vm) => vm.load_program(program),
                        Err(err) => CustomLogger::log(&format!("{:?}", err)),
                    }
                }
                //self.vm.load_program(program);
                self.parsing_error = None
            }
            Err(err) => self.parsing_error = Some(err),
        }
    }
    /// Execute one instruction FIXME:
    fn step(&mut self) {
        // if self.vm.lock().unwrap().get_program().is_empty() {
        //     self.try_assemble_and_load();
        // }

        {
            let mut vm = self.vm.lock().unwrap();
            if vm.get_pc() >= vm.get_program().len() {
                vm.clear_registers();
            }
        }

        match self.parsing_error {
            Some(_) => todo!(),
            None => {
                self.vm.lock().unwrap().execute();
            }
        }
    }
}
