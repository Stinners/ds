
use std::collections::HashSet;
use std::io::{BufRead, BufReader, stdout, stderr, Read};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::hash::{Hash, Hasher};
use std::borrow::Cow;
use std::path::Path;

use crate::processes::{LineSource, LineMessage};

// ====================== Constants =========================

const GREEN_TEXT: &'static str = "\x1B[92m";
const YELLOW_TEXT: &'static str = "\x1B[93m";
const UNDERLINE_TEXT: &'static str = "\x1B[4m";
const RESET_TEXT: &'static str = "\x1B[0m";

// ====================== Types =========================

#[derive(Debug, Clone)]
pub struct File {
    pub idx: usize,
    pub name: String, 
    pub line: Option<usize>,
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.line == other.line
    }
}
impl Eq for File {}

impl Hash for File {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.line.hash(state);
    }
}


#[derive(PartialEq, Eq, Debug)]
enum LinePart<'a> {
    Text(&'a str),
    Candidate(&'a str),
    File(File),
    Space,
}

impl<'a> LinePart<'a> {
    fn new(slice: &'a str, is_candidate: bool) -> LinePart<'a> {
        if slice == "" {
            LinePart::Space
        } else if is_candidate {
            LinePart::Candidate(slice)
        } else {
            LinePart::Text(slice)
        }
    }

    pub fn render(&self) -> Option<Cow<'a, str>> {
        match self {
            LinePart::Text(text) => Some(Cow::Borrowed(text)),
            LinePart::Space => None,
            LinePart::File(file) => {
                let text = format!("{YELLOW_TEXT}{0}. {UNDERLINE_TEXT}{1}{RESET_TEXT}", file.idx, file.name);
                Some(Cow::Owned(text))
            }
            LinePart::Candidate(_) => {
                println!("Invalid Candidate line part in 'print_part': {:?}", self);
                println!("This should have been convtered to either a File or Text by now");
                panic!();
            },
        }
    }
}


// ====================== Main Function =========================


pub fn process_streams(rx: Receiver<LineMessage>, files: &mut HashSet<File>) {
    let mut stdout_closed = false;
    let mut stderr_closed = false;

    loop {
        let message = rx.recv().unwrap();
        let line_parts = parse_line(&message.line);
        print_line_parts(files, line_parts);

        if message.close_stream {
            match message.source {
                LineSource::Out => stdout_closed = true,
                LineSource::Error => stderr_closed = true,
            };
        }

        if stderr_closed && stdout_closed {
            break;
        }
    }
}


// ====================== Helpers =========================


// Parse a file and break it into a list of line parts
fn parse_line<'a>(line: &'a str) -> Vec<LinePart<'a>> {
    let mut parts = vec!();

    let mut start_idx = 0;
    let mut is_candiate = false;

    for (idx, byte) in line.bytes().enumerate() {

        // A space indicates the end of a token 
        // TODO: find a better way to do this then comparing to magic numbers
        if byte == 32 {
            let slice = &line[start_idx..idx];
            let line_part = LinePart::new(slice, is_candiate);

            parts.push(line_part);
            start_idx = idx + 1;
            is_candiate = false;
        } 

        // A dot in a token indicates a potential file
        else if byte == 46 {
            is_candiate = true;
        }
    }

    // Capture the final token 
    let slice = &line[start_idx..];
    if slice != "" {
        let line_part = LinePart::new(slice, is_candiate);
        parts.push(line_part);
    }

    parts
}



fn print_line_parts(files: &mut HashSet<File>, line: Vec<LinePart>) -> String {
    let parts = line.into_iter().map(|part| check_if_file_exists(files, part));

    let mut output = String::new();
    for part in parts {
        let text = part.render();

        if let Some(text) = text {
            output.push_str(&text);
        }
        output.push_str(" ");
    }
    output

}


fn check_if_file_exists<'a>(files: &mut HashSet<File>, raw_part: LinePart<'a>) -> LinePart<'a> {
    // Check candidates in the line and return 
    match raw_part {
        LinePart::Candidate(name) => {
            if Path::new(name).exists() {
            let file = File { idx: files.len() + 1, name: name.to_string(), line: None };
                if !files.contains(&file) {
                    files.insert(file.clone());
                    return LinePart::File(file);
                } 
            }
            LinePart::Text(name)

        },

        other_part => other_part,
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_handles_single_token() {
        let line = "token ".to_string();

        let expected = LinePart::Text("token");
        let actual = parse_line(&line);

        assert!(actual.len() == 1);
        assert_eq!(expected, actual[0]);
    }

    #[test]
    fn parser_handles_end_of_string() {
        let line = "token".to_string();

        let expected = LinePart::Text("token");
        let actual = parse_line(&line);

        assert_eq!(expected, actual[0]);
    }

    #[test]
    fn parser_handles_multiple_tokens() {
        let line = "token1 token2".to_string();

        let expected = vec!(LinePart::Text("token1"), LinePart::Text("token2"));
        let actual = parse_line(&line);

        assert_eq!(expected, actual);
    }

    #[test]
    fn parser_recognized_candidates() {
        let line = "token token.txt".to_string();

        let expected = vec!(LinePart::Text("token"), LinePart::Candidate("token.txt"));
        let actual = parse_line(&line);

        assert_eq!(expected, actual);
    }

    #[test]
    fn parser_handles_multiple_spaces() {
        let line = "token  token".to_string();

        let expected = vec!(LinePart::Text("token"), LinePart::Space, LinePart::Text("token"));
        let actual = parse_line(&line);

        assert_eq!(expected , actual);
    }
}
