// services/nilimed/src/engine.rs — Latin to Bengali Phonetic Engine
use std::collections::HashMap;

pub struct PhoneticEngine {
    map: HashMap<&'static str, &'static str>,
}

impl PhoneticEngine {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        map.insert("ami", "আমি");
        map.insert("tumi", "তুমি");
        map.insert("bangla", "বাংলা");
        map.insert("nilos", "নিলওএস");
        map.insert("dhaka", "ঢাকা");
        Self { map }
    }

    pub fn transliterate(&self, input: &str) -> String {
        if let Some(res) = self.map.get(input) {
            res.to_string()
        } else {
            input.to_string()
        }
    }
}
