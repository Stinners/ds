
use std::io::{self, BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;

use crate::input_reader::CommandCall;

#[derive(Copy, Clone, Debug)]
pub enum LineSource { Out, Error }

#[derive(Debug)]
pub struct LineMessage {
    pub line: String,
    pub source: LineSource,
    pub close_stream: bool,
}


// This takes a stream (stdout or stderr) from a process, formats it's output into LineMessages 
// and writes it's output to the sender
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

pub fn run_command(command: &CommandCall) -> io::Result<Receiver<LineMessage>> {
    dbg!(&command);

    let output = Command::new(command.command.clone())
        .args(command.args.clone())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = output.stdout.unwrap();
    let stderr = output.stderr.unwrap();

    let (tx, rx): (Sender<LineMessage>, Receiver<LineMessage>) = mpsc::channel();
    capture_stream(stdout, LineSource::Out, tx.clone());
    capture_stream(stderr, LineSource::Error, tx);

    Ok(rx)
}
