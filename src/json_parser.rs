use std::collections::HashMap;

use eframe::egui::NumExt;

#[derive(Debug, Clone)]
pub enum JsonStruct {
    Str(String),
    Obj(HashMap<String, JsonStruct>),
    Arr(Vec<JsonStruct>),
}

impl JsonStruct {
    pub fn as_str(&self) -> String {
        match self {
            JsonStruct::Str(s) => json_replace_character_back(s),
            _ => {
                eprintln!("Not a JsonStruct::Str");
                return String::new();
            }
        }
    }

    pub fn as_str_array(&self) -> Vec<String> {
        match self {
            JsonStruct::Arr(arr) => arr.iter().map(|v| v.as_str()).collect(),
            _ => {
                eprintln!("Not a JsonStruct::Arr");
                return Vec::new();
            }
        }
    }

    pub fn as_obj(&self) -> HashMap<String, JsonStruct> {
        match self {
            JsonStruct::Obj(o) => o.clone(),
            _ => {
                eprintln!("Not a JsonStruct::Obj");
                return HashMap::new();
            }
        }
    }
}

pub fn json_replace_character_back(s: &str)  -> String {
    s.replace("\\n", "\n")
     .replace("\\r", "\r")
     .replace("\\t", "\t")
     .replace("\\\"", "\"")
     .replace("\\\\", "\\")
}

pub fn json_split_fields(json: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current_field = String::new();

    let mut depth_obj = 0;
    let mut depth_arr = 0;
    let mut is_str = false;
    let mut previous_character = '\0';

    for character in json.chars() {
        match character {
            '{' if !is_str => {
                depth_obj += 1;
                current_field.push(character);
            }

            '{' if !is_str => {
                depth_obj -= 1;
                current_field.push(character);
            }

            '[' if !is_str => {
                depth_arr += 1;
                current_field.push(character);
            }

            ']' if !is_str => {
                depth_arr -= 1;
                current_field.push(character);
            }

            ',' if !is_str && depth_obj == 0 && depth_arr == 0 => {
                fields.push(current_field.trim().to_string());
                current_field.clear();
            }

            '"' if previous_character != '\\' => {
                is_str = !is_str;
                current_field.push(character);
            }

            _ => current_field.push(character),
        }

        previous_character = character;
    }

    if !current_field.trim().is_empty() {
        fields.push(current_field.trim().to_string());
    }

    fields
}

pub fn json_clasify(json: &str) -> JsonStruct {
    let mut s = json.trim();

    if s.starts_with('{') {
        JsonStruct::Obj(json_parse_obj(s))
    } else if s.starts_with('[') {
        JsonStruct::Arr(json_parse_arr(s))
    } else {
        JsonStruct::Str(json_parse_str(s))
    }
}

pub fn json_parse_str(json: &str) -> String {
    json.trim().trim_matches('"').to_string()
}

pub fn json_parse_arr(json: &str) -> Vec<JsonStruct> {
    let mut content = json.trim();
    let mut content = &content[1..content.len() - 1];

    let fields = json_split_fields(content);
    fields
        .into_iter()
        .map(|field| json_clasify(&field))
        .collect()
}

pub fn json_parse_obj(json: &str) -> HashMap<String, JsonStruct> {
    let mut hashmap = HashMap::new();

    let mut s = json.trim();
    let mut content = &s[1..s.len() - 1];

    let mut fields = json_split_fields(content);

    for field in fields {
        let mut key_value = field.trim().splitn(2, ":");

        let mut key = match key_value.next() {
            Some(k) => k,
            None => "",
        };

        let mut value = match key_value.next() {
            Some(v) => v,
            None => "",
        };

        key = key.trim();
        value = value.trim();

        let k_parsed = json_parse_str(key);
        let v_parsed = json_clasify(value);

        hashmap.insert(k_parsed, v_parsed);
    }

    hashmap
}

// pub fn from_map_to_app_config(map: &HashMap<String, String>) -> Option<AppParameters> {

// }
