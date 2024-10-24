#![allow(dead_code)]
#![allow(unused_imports)]

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

const GREEN_TEXT: &'static str = "\x1B[92m";
const YELLOW_TEXT: &'static str = "\x1B[93m";
const UNDERLINE_TEXT: &'static str = "\x1B[4m";
const RESET_TEXT: &'static str = "\x1B[0m";

fn red_print() {
    println!("{YELLOW_TEXT}{}{RESET_TEXT}", "Hello World!");
}


fn launch_vim(file: &str, line_no: Option<usize>) { 
    // Launch Vim
    let mut cmd = Command::new("nvim");
    let mut launch = cmd.arg(file);

    if let Some(line) = line_no {
        launch = launch.arg(format!("+{}", line));
    }

    let status = launch.status().expect("Failed to open Vim");

    // Check if Vim exited successfully
    if !status.success() {
        println!("Vim did not exit successfully");
    }
}

#[derive(Copy, Clone)]
enum LineSource { Out, Error }

struct LineMessage {
    line: String,
    source: LineSource,
    close_stream: bool,
}

// This takes a stream (stdout or stderr) from a process and writes it's output
fn capture_stream<R>(mut stream: R, stream_type: LineSource, tx: Sender<LineMessage>)
where 
    R: Read + Send + 'static 
{ 
    // Spawn a tread to listen to the output of this stream and send it to the channel
    let _ = thread::Builder::new() 
        .name("Capturing output".into()) 
        .spawn(move || loop {
            let mut str_buffer = String::with_capacity(80);
            let mut buffer = BufReader::new(&mut stream);
            // Repeatedly read lines from the stream and writing to the channel
            loop {
                let read_result = buffer.read_line(&mut str_buffer);
                match read_result {
                    Ok(code) => {
                        let should_close = code == 0;
                        let message = LineMessage {
                            line:  str_buffer.clone(),
                            source: stream_type,
                            close_stream: should_close,
                        };

                        let _ = tx.send(message);

                        if should_close {  // End of stream 
                            break;
                        }
                    },
                    Err(_msg) => {
                        todo!();
                    }
                }
                str_buffer.clear();
            }
        });
}

fn process_streams(rx: Receiver<LineMessage>) {
    let mut stdout_closed = false;
    let mut stderr_closed = false;

    loop {
        let message = rx.recv().unwrap();

        parse_line(&message.line);

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

#[derive(Debug, Clone)]
struct File {
    idx: usize,
    name: String, 
    line: Option<usize>,
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

fn make_part<'a>(slice: &'a str, is_candidate: bool) -> LinePart<'a> {
    if slice == "" {
        LinePart::Space
    } else if is_candidate {
        LinePart::Candidate(slice)
    } else {
        LinePart::Text(slice)
    }
}

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
            let line_part = make_part(slice, is_candiate);

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
        let line_part = make_part(slice, is_candiate);
        parts.push(line_part);
    }

    parts
}


fn print_part<'a>(part: &'a LinePart) -> Option<Cow<'a, str>> {
    match part {
        LinePart::Text(text) => Some(Cow::Borrowed(text)),
        LinePart::File(file) => {
            let text = format!("{YELLOW_TEXT}{0}. {UNDERLINE_TEXT}{1}{RESET_TEXT}", file.idx, file.name);
            Some(Cow::Owned(text))
        }
        LinePart::Space => None,
        LinePart::Candidate(_) => unreachable!(),
    }
}
    

fn check_file<'a>(files: &mut HashSet<File>, raw_part: LinePart<'a>) -> LinePart<'a> {
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


fn interpret_line(files: &mut HashSet<File>, line: Vec<LinePart>) -> String {
    let mut output = String::new();

    for raw_part in line.into_iter() {
        let part = check_file(files, raw_part);
        let text = print_part(&part);

        if let Some(text) = text {
            output.push_str(&text);
        }
        output.push_str(" ");
    }
    output

}

fn start_process(command: &str) {

    let output = Command::new(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let stdout = output.stdout.unwrap();
    let stderr = output.stderr.unwrap();

    let (tx, rx): (Sender<LineMessage>, Receiver<LineMessage>) = mpsc::channel();
    capture_stream(stdout, LineSource::Out, tx.clone());
    capture_stream(stderr, LineSource::Error, tx);
    process_streams(rx);

}

fn main() {
    let command = "./spaces.bash";
    start_process(command);

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
