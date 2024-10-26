#![allow(dead_code)]
#![allow(unused_imports)]

mod processes;
mod line_reader;

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

use crate::processes::run_command;
use crate::line_reader::process_streams;

fn main() {
    let command = "./spaces.bash";
    let reciever = run_command(command);

    let mut file_set = HashSet::new();

    process_streams(reciever, &mut file_set);
}

