// apps/calculator/src/main.rs — OnuronOS Alap Calculator
// Using the official OnuronOS Design System (dark/light themes)
use nilui::design::Palette;
use nilui::{App, Element, Ev};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalcState {
    pub display: String,
    pub expression: String,
    pub prev_operand: f64,
    pub operation: Option<char>,
    pub waiting_for_operand: bool,
}

impl Default for CalcState {
    fn default() -> Self {
        Self {
            display: "0".to_string(),
            expression: String::new(),
            prev_operand: 0.0,
            operation: None,
            waiting_for_operand: true,
        }
    }
}

pub fn calculate(a: f64, b: f64, op: char) -> f64 {
    match op {
        '+' => a + b,
        '-' => a - b,
        '*' => a * b,
        '/' => {
            if b == 0.0 {
                f64::NAN
            } else {
                a / b
            }
        }
        _ => b,
    }
}

pub fn update(state: &mut CalcState, ev: Ev) {
    match ev {
        Ev::Click(d) if d <= 9 => {
            if state.waiting_for_operand {
                state.display = d.to_string();
                state.waiting_for_operand = false;
            } else {
                state.display.push_str(&d.to_string());
            }
        }
        Ev::Click(46) => {
            // '.'
            if !state.display.contains('.') {
                if state.waiting_for_operand {
                    state.display = "0.".to_string();
                    state.waiting_for_operand = false;
                } else {
                    state.display += ".";
                }
            }
        }
        Ev::Click(43) | Ev::Click(45) | Ev::Click(42) | Ev::Click(47) => {
            // + - * /
            let op = match ev {
                Ev::Click(43) => '+',
                Ev::Click(45) => '-',
                Ev::Click(42) => '*',
                Ev::Click(47) => '/',
                _ => return,
            };
            if let Ok(val) = state.display.parse::<f64>() {
                if let Some(op) = state.operation {
                    let result =
                        calculate(state.prev_operand, state.display.parse().unwrap_or(0.0), op);
                    if result.is_nan() {
                        state.display = "Error".to_string();
                    } else {
                        state.display = result.to_string();
                        state.expression = format!("{} {}", state.expression, op);
                    }
                    state.prev_operand = result;
                    state.operation = None;
                    state.waiting_for_operand = true;
                    state.waiting_for_operand = true;
                    state.expression = format!("{}", op);
                }
            }
        }
        Ev::Click(61) => {
            // '='
            if let Some(op) = state.operation {
                if let Ok(val) = state.display.parse::<f64>() {
                    let result = calculate(state.prev_operand, val, op);
                    state.display = if result.is_nan() {
                        "Error".to_string()
                    } else {
                        result.to_string()
                    };
                    state.expression = format!("{} = {}", state.expression, result);
                    state.prev_operand = result;
                }
            }
            state.operation = None;
            state.waiting_for_operand = true;
        }
        Ev::Click(67) => {
            // 'C'
            state.display = "0".to_string();
            state.expression.clear();
            state.prev_operand = 0.0;
            state.operation = None;
            state.waiting_for_operand = true;
        }
        Ev::Click(8324) => {
            // ⌫
            if !state.waiting_for_operand {
                state.display.pop();
                if state.display.is_empty() {
                    state.display = "0".into();
                }
            }
        }
        _ => {}
    }
}

pub fn view(state: &CalcState) -> Element {
    let palette = Palette::dark();

    let button = |label, id| Element::Button {
        label: format!("  {}  ", label),
        on_click_id: id,
    };

    Element::Column {
        children: vec![
            Element::Text {
                content: state.expression.clone(),
            },
            Element::Text {
                content: state.display.clone(),
            },
            Element::Row {
                children: vec![
                    button('C', 100),
                    button('⌫', 101),
                    button('/', 102),
                    button('*', 103),
                ],
            },
            Element::Row {
                children: vec![
                    button('7', 104),
                    button('8', 105),
                    button('9', 106),
                    button('-', 107),
                ],
            },
            Element::Row {
                children: vec![
                    button('4', 108),
                    button('5', 109),
                    button('6', 110),
                    button('+', 111),
                ],
            },
            Element::Row {
                children: vec![
                    button('1', 112),
                    button('2', 113),
                    button('3', 114),
                    button('=', 115),
                ],
            },
            Element::Row {
                children: vec![button('0', 116), button('.', 117)],
            },
        ],
    }
}

fn main() {
    let app = App {
        state: CalcState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Alap Calculator");
}
