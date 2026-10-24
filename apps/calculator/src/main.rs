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
            let next_op = match ev {
                Ev::Click(43) => '+',
                Ev::Click(45) => '-',
                Ev::Click(42) => '*',
                Ev::Click(47) => '/',
                _ => return,
            };
            if let Ok(val) = state.display.parse::<f64>() {
                if let Some(op) = state.operation {
                    let result = calculate(state.prev_operand, val, op);
                    if result.is_nan() {
                        state.display = "Error".to_string();
                    } else {
                        state.display = result.to_string();
                    }
                    state.prev_operand = result;
                } else {
                    state.prev_operand = val;
                }
                state.operation = Some(next_op);
                state.expression = format!("{} {}", state.prev_operand, next_op);
                state.waiting_for_operand = true;
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
    let _palette = Palette::dark();

    let button = |label, id| Element::Button {
        label: format!("  {}  ", label),
        on_click_id: id,
    };

    Element::Column {
        children: vec![
            Element::Text {
                content: "🧮 Alap ক্যালকুলেটর".to_string(),
            },
            Element::Text {
                content: format!("  {}", state.expression),
            },
            Element::Text {
                content: format!("  {}", state.display),
            },
            Element::Row {
                children: vec![
                    button("C", 67),
                    button("⌫", 8324),
                    button("/", 47),
                    button("*", 42),
                ],
            },
            Element::Row {
                children: vec![
                    button("7", 7),
                    button("8", 8),
                    button("9", 9),
                    button("-", 45),
                ],
            },
            Element::Row {
                children: vec![
                    button("4", 4),
                    button("5", 5),
                    button("6", 6),
                    button("+", 43),
                ],
            },
            Element::Row {
                children: vec![
                    button("1", 1),
                    button("2", 2),
                    button("3", 3),
                    button("=", 61),
                ],
            },
            Element::Row {
                children: vec![
                    button("0", 0),
                    button(".", 46),
                ],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_math() {
        assert_eq!(calculate(10.0, 5.0, '+'), 15.0);
        assert_eq!(calculate(10.0, 5.0, '-'), 5.0);
        assert_eq!(calculate(10.0, 5.0, '*'), 50.0);
        assert_eq!(calculate(10.0, 5.0, '/'), 2.0);
        assert!(calculate(10.0, 0.0, '/').is_nan());
    }

    #[test]
    fn test_calculator_full_flow() {
        let mut state = CalcState::default();

        // 7 + 3 = 10
        update(&mut state, Ev::Click(7));
        assert_eq!(state.display, "7");

        update(&mut state, Ev::Click(43)); // +
        assert_eq!(state.operation, Some('+'));

        update(&mut state, Ev::Click(3));
        assert_eq!(state.display, "3");

        update(&mut state, Ev::Click(61)); // =
        assert_eq!(state.display, "10");

        // Clear
        update(&mut state, Ev::Click(67)); // C
        assert_eq!(state.display, "0");
        assert_eq!(state.operation, None);
    }
}
