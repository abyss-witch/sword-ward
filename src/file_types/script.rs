use std::str::FromStr;
use crate::{file_types::*, PlayerData, GraphicsData};
pub struct ScriptReader {
    script_data: Option<ScriptReaderData>,
}
impl ScriptReader {
    pub fn new() -> Self { Self { script_data: None, } }
    pub fn current_options(&self) -> Vec<&str> {
        let Some(segment) = self.current_segment() else { return Vec::new() };
        let mut options = Vec::new();
        for o in &segment.options { options.push(o.text.as_str()) }
        let no_change = segment.add_tags.is_empty() && segment.remove_tags.is_empty();
        if !self.script_data.as_ref().unwrap().option_index.is_empty() && no_change {
            options.push("[Go Back]");
        }
        options
    }
    pub fn current_segment(&self) -> Option<&Segment> {
        let script_data = self.script_data.as_ref()?;
        let mut segment = &script_data.script.segments[script_data.index];
        for i in &script_data.option_index {
            segment = &segment.options[*i];
        }
        Some(segment)
    }
    pub fn valid_current_segment(&self, data: &PlayerData) -> bool {
        let Some(segment) = self.current_segment() else { return true };
        segment.requirements
            .as_ref()
            .map(|i| i.evaluate(data))
            .unwrap_or(true)
    }
    pub fn next(&mut self, selection: usize, data: &mut PlayerData) -> Option<()> {
        data.acquired_tags.append(&mut self.current_segment()?.add_tags.clone());
        if !self.current_options().is_empty() {
            // go back option
            if self.current_segment().as_ref()?.options.len() == selection {
                self.script_data.as_mut()?.option_index.pop();
                return Some(());
            } else {
                self.script_data.as_mut()?.option_index.push(selection);
            }
            if self.current_options().is_empty() { self.next(0, data); }
        } else {
            self.script_data.as_mut()?.option_index = Vec::new();
            self.script_data.as_mut()?.index += 1;
            if self.script_data.as_ref()?.index >= self.script_data.as_ref()?.script.segments.len() {
                self.script_data = None;
                return Some(())
            }
            while !self.valid_current_segment(data) {
                self.script_data.as_mut()?.index += 1;
                if self.script_data.as_ref()?.index >= self.script_data.as_ref()?.script.segments.len() {
                    self.script_data = None;
                    return Some(())
                }
            }
        }
        Some(())
    }
    pub fn set_script(&mut self, name: &str, player_data: &mut PlayerData, scene_loader: &mut GraphicsData) {
        let script = scene_loader.scripts[name].clone();
        let name = name.to_string();
        if player_data.read_scripts.contains(&name) { return }
        player_data.read_scripts.push(name);
        self.script_data = Some(ScriptReaderData { script, index: 0, option_index: Vec::new() })
    }
}
struct ScriptReaderData {
    script: Script,
    index: usize,
    option_index: Vec<usize>,
}
#[derive(Debug, PartialEq, Clone)]
pub struct Script {
    pub segments: Vec<Segment>
}
impl Script {
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, ParseErr> {
        std::io::read_to_string(std::fs::File::open(path)?)?.parse()
    }
}
impl FromStr for Script {
    type Err = ParseErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { segments: split_segments(s)? })
    }
}
#[derive(Debug, PartialEq, Clone)]
pub struct Segment {
    pub text: String,
    pub options: Vec<Segment>,
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
    pub requirements: Option<Requirements>,
}
impl FromStr for Segment {
    type Err = ParseErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if debug_parse() { println!("parsing segment: {s}") }
        let mut word = String::new();
        let mut last_was_control = false;
        let mut skip = 0;

        let mut text = String::new();
        let mut requirements = None;
        let mut add_tags    = Vec::new();
        let mut remove_tags = Vec::new();
        let mut options     = Vec::new();

        for (i, c) in s.chars().enumerate() {
            if skip != 0 { skip -= 1; continue }
            if c.is_whitespace() {
                if !last_was_control { text += &word; word = String::new() } 
                else { last_was_control = false }
            }
            if c == '[' { match word.trim() {
                "$req" => {
                    let (inner, temp_skip) = split_bracket(&s[(i+1)..])?;
                    skip = temp_skip;
                    requirements = Some(inner.parse()?);
                    last_was_control = true;
                },
                "$add" => {
                    let (inner, temp_skip) = split_bracket(&s[(i+1)..])?;
                    skip = temp_skip;
                    let mut adding = inner.split_whitespace().map(|i| i.trim().to_string()).collect();
                    add_tags.append(&mut adding);
                    last_was_control = true;
                },
                "$rem" => {
                    let (inner, temp_skip) = split_bracket(&s[(i+1)..])?;
                    skip = temp_skip;
                    let mut removing = inner.split_whitespace().map(|i| i.trim().to_string()).collect();
                    remove_tags.append(&mut removing);
                    last_was_control = true;
                },
                "$opt" => {
                    let (inner, temp_skip) = split_bracket(&s[(i+1)..])?;
                    skip = temp_skip;
                    options.append(&mut split_segments(&inner)?);
                    last_was_control = true;
                },
                _ => ()
            } }
            word.push(c);
        }
        let mut parsed_text = String::new();
        let mut last_was_control = false;
        let mut trimmed = false;
        for c in text.chars() {
            last_was_control = !last_was_control && c == '\\';
            if !c.is_whitespace() { trimmed = true }
            if last_was_control || (!trimmed && c.is_whitespace()) { continue }
            parsed_text.push(c);
        }
        Ok(Self {
            requirements,
            text: parsed_text,
            add_tags,
            remove_tags,
            options,
        })
    }
}
const CONTROL_WORDS: &[&str] = &["$req", "$add", "$rem", "$opt"];
fn split_next_segment(s: &str) -> Result<(String, usize), ParseErr> {
    let mut segment = String::new();
    let mut word    = String::new();
    let mut segment_args_started = false;
    let mut skip = 0;
    for (i, c) in s.chars().enumerate() {
        if skip != 0 { skip -= 1; continue }
        if c.is_whitespace() {
            word.push(c);
            segment += &word;
            word = String::new();
            if segment_args_started && !CONTROL_WORDS.contains(&(word.clone()+"[").trim()) {
                return Ok((s[..i].to_string(), i))
            }
        }
        if c == '[' && CONTROL_WORDS.contains(&word.trim()) {
            segment_args_started = true;
            (_, skip) = split_bracket(&s[(i+1)..])?;
        }
        word.push(c);
    }
    Ok((s.to_string(), s.len()))
}
fn split_segments(s: &str) -> Result<Vec<Segment>, ParseErr> {
    if debug_parse() { println!("splitting segments: {s}") }
    let mut segments = Vec::new();
    let mut s = s.to_string();
    loop {
        let (segment, i) = split_next_segment(&s)?;
        if !segment.trim().is_empty() { segments.push(segment.parse()?) }
        if i == s.len() { return Ok(segments) }
        s = s[i..].to_string();
    }
}
