use egui::{Context, Ui};
use serde::{Serialize, Deserialize};



#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ValueFormat {
    Dec,
    Bin,
    Hex,
    Unicode,
}

impl Default for ValueFormat {
    fn default() -> Self {
        ValueFormat::Dec
    }
}

#[derive(Default, Clone)]
pub struct IndicatorWidget {
    data: i32,
    format: ValueFormat,
    text: String,
    label: String,
}

impl IndicatorWidget {
    pub fn new(label: String) -> Self {
        Self {
            data: Default::default(),
            format: ValueFormat::Dec,
            text: Default::default(),
            label,
        }
    }
    pub fn set(&mut self, data: i32, label: &str) -> &mut IndicatorWidget {
        self.data = data;
        self.label= label.to_string();
        self
    }
    pub fn set_data(&mut self, data: i32) -> &mut IndicatorWidget {
        self.data = data;
        self
    }

    pub fn show(&mut self, _ctx: &Context, ui: &mut Ui) {
        let formatted_value = self.write_in_format();
        ui.label(&self.label);
        if ui.button(formatted_value).clicked() {
            self.switch_format();
        }
    }

    fn write_in_format(&mut self) -> String {
        match self.format {
            ValueFormat::Dec => format!("{}", self.data),
            ValueFormat::Hex => format!("0b{:b}", self.data),
            ValueFormat::Bin => format!("0X{:X}", self.data),
            ValueFormat::Unicode => {
                if self.data < 0 {
                    return t!("common.invalid_char");
                } else {
                    if let Some(char) = char::from_u32(self.data as u32) {
                        return format!("\'{}\'", char);
                    }
                    return t!("common.invalid_char");
                }
            }
        }
    }

    fn switch_format(&mut self) {
        match self.format {
            ValueFormat::Dec => self.format = ValueFormat::Bin,
            ValueFormat::Bin => self.format = ValueFormat::Hex,
            ValueFormat::Hex => self.format = ValueFormat::Unicode,
            ValueFormat::Unicode => self.format = ValueFormat::Dec,
        }
    }
}
