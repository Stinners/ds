#![allow(dead_code)]
#![allow(unused_imports)]

use std::io::{stdin, stdout, Read, Write};

mod processes;
mod line_reader;

use std::collections::HashSet;
use std::process::Command;

use crate::processes::run_command;
use crate::line_reader::{process_streams, File};

fn main() {
    let command = "./test.bash";
    let files = run(command).unwrap();

    let file_num = read_file_number(files.len()).unwrap();
    open_file(&files, file_num);
}

// Run a command an extract a list of files
fn run(command: &str) -> Option<HashSet<File>> {
    match run_command(command) {
        Ok(stream) => {
            let mut file_set = HashSet::new();
            process_streams(stream, &mut file_set);
            Some(file_set)
        },
        Err(error) => {
            println!("Failed to start process: {}, {}", command, error);
            None
        }
    }
}


fn print_prompt(prompt: &str) {
    print!("{}", prompt);
    let _ = stdout().flush();
}



fn read_file_number(max_n: usize) -> Option<usize> {
    let mut input = String::new();
    print_prompt("Enter a file number: ");
    let _ = stdin().read_line(&mut input);
    match input.trim().parse::<usize>() {
        Ok(n) => {
            if n > max_n {
                println!("{}, is not a valid file number", n);
                None
            } 
            else {
                Some(n)
            }
        }
        Err(_) => {
            print!("{}, is not a number", input);
            None
        }
    }
}


fn open_file(files: &HashSet<File>, file_num: usize) {
    let file = files.into_iter().find(|f| f.idx == file_num).unwrap();
    let _ = Command::new("nvim")
        .arg(file.name.clone())
        .status()
        .unwrap();
}
