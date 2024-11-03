#![allow(dead_code)]
#![allow(unused_imports)]

use std::env;
use std::collections::HashSet;
use std::io::{stdin, stdout, Read, Write, BufReader, BufRead};
use std::path::{PathBuf, Path};
use std::process::Command;

mod processes;
mod line_reader;
mod input_reader;


use crate::processes::run_command;
use crate::line_reader::{process_streams, File};
use crate::input_reader::{read_args, CommandCall, Config};

fn main() {
    match run() {
        Ok(_) => (),
        Err(message) => println!("{}", message),
    }
}

fn run() -> Result<(), String> {
    let args = env::args().collect();
    let (command, config) = read_args(args)?;
    let files = execute_command_and_read_files(&config, command)?;

    if files.len() != 0 {
        let file_num = read_file_number(files.len())?;
        open_file(&files, file_num)?;
    }
    else {
        println!("No files found in output");
    }
    Ok(())
}


// Run a command an extract a list of files
fn execute_command_and_read_files(config: &Config, command: CommandCall) -> Result<HashSet<File>, String> {
    let stream = run_command(&command)
        .map_err(|err| format!("Failed to start process: '{}', {}", command.command, err))?;
    let file_set = process_streams(&config, stream);
    Ok(file_set)
}


fn read_input(prompt: &str) -> String {
    let mut input = String::new();
    print!("{}", prompt);
    let _ = stdout().flush();
    let _ = stdin().read_line(&mut input);
    input
}


fn read_file_number(max_n: usize) -> Result<usize, String> {
    let input = read_input("Enter a file number: ");

    match input.trim().parse::<usize>() {
        Ok(n) => {
            if n > max_n {
                Err(format!("{}, is not a valid file number", n))
            } 
            else {
                Ok(n)
            }
        }
        Err(_) => {
                Err(format!("{}, is not a number", input))
        }
    }
}


fn open_file(files: &HashSet<File>, file_num: usize) -> Result<(), String> {
    let file = files.into_iter().find(|f| f.idx == file_num).unwrap();

    let mut cmd = Command::new("nvim");

    if let Ok(home_dir) = std::env::var("HOME") {
        let server_path = Path::new(&home_dir).join(".config/nvim/server.pipe");
        cmd.arg("--server")
           .arg(server_path)
           .arg("--remote");
    };

    let out = cmd.arg(file.name.clone()).status();

    match out {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Failed to open file: {}", err)),
    }
}
