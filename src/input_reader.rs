
use std::env;
use std::io::{Seek, SeekFrom};

#[derive(Debug, PartialEq)]
pub struct Config {
    pub no_colour: bool , 
    pub last_files: bool, 
    pub replay_last: bool,
    pub store_only: bool,
    pub files_only: bool,
    pub open_here: bool,
    pub print_help: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            no_colour: false,
            last_files: false,
            replay_last: false,
            store_only: false,
            files_only: false,
            open_here: false,
            print_help: false,
        }
    }
}

#[derive(Debug)]
struct Flag {
    short: char, 
    long: String,
    description: String,
}

impl Flag {
    fn new(short: char, long: &str, description: &str) -> Flag {
        Flag {
            short: short.into(),
            long: long.into(),
            description: description.into(),
        }
    }

    fn build_flags() -> Vec<Flag> {
        vec!(
            Flag::new('c', "no-colour",   "Prints output without coloring file names"),
            Flag::new('l', "last",        "Prints the stored filenames from the last run"),
            Flag::new('r', "replay-last", "Runs the last command found in the shell history file"),
            Flag::new('s', "store",       "Stores the files found, but does not prompt to select a file"),
            Flag::new('f', "files-only",  "Prints only the filenames, not surrounding context"),
            Flag::new('o', "open-here",   "Open file in current terminal, not using nvim server"),
            Flag::new('h', "help",        "Prints this message and exits"),
        )
    }
}

impl Config {
    fn set_flag(&mut self, flag: &Flag) {
        match flag.short {
            'c' => self.no_colour = true,
            'l' => self.last_files = true,
            'r' => self.replay_last = true,
            's' => self.store_only = true,
            'f' => self.files_only = true,
            'h' => self.print_help = true,
            _ => unreachable!("Invalid input flag passed to set_flag")
        }
    }
}


#[derive(Debug)]
pub struct CommandCall {
    pub command: String,
    pub args: Vec<String>,
}


pub fn read_args(args: Vec<String>) -> Result<(CommandCall, Config), String> {
    let (config_args, command_args) = split_config_command(args);
    
    let config = parse_config(config_args)?;

    /*
    let command = if config.replay_last {
        read_last_command_from_hist_file()?
    }
    else {
    };
    */
    let  command = parse_command(command_args)?;

    // TODO: strip of the ds parts and call the underlying command
    if command.command == "ds" {
        return Err(format!("Command is a recursive call to 'ds'"));
    }

    Ok((command, config))
}

fn split_config_command(args: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut config = vec!();
    let mut command = vec!();
    let mut reading_config = true;

    for arg in args.into_iter().skip(1) {
        if !arg.starts_with("-") {
            reading_config = false;
        }

        if reading_config {
            config.push(arg);
        } 
        else {
            command.push(arg);
        }
    }

    (config, command)
}


fn parse_config(args: Vec<String>) -> Result<Config, String> {
    let mut config = Config::default();
    let flags = Flag::build_flags();

    for arg in args {

        if arg.starts_with("---") {
            return Err(format!("Invalid argument '{}'. Start parametes with 1 or 2 dashes", arg));
        }

        // Long form args
        else if arg.starts_with("--") {
            let arg_name = &arg[2..];
            let this_flag = flags.iter().find(|flag| flag.long == arg_name);
            
            match this_flag {
                Some(flag) => config.set_flag(&flag),
                None => return Err(format!("Invalid parameter '{}' found", arg)),
            };
        }

        // Short form params
        else if arg.starts_with("-") {
            let arg_chars = &arg[1..];
            for c in arg_chars.chars() {
                let this_flag = flags.iter().find(|flag| flag.short == c);

                match this_flag {
                    Some(flag) => config.set_flag(&flag),
                    None => return Err(format!("Invalid parameter {} found in group {}", c, arg))

                };
            }
        }
    }
    Ok(config)
}


fn parse_command(args: Vec<String>) -> Result<CommandCall, String> {
    let args: Vec<String> = args.into_iter().collect();

    if args.len() < 1 {
        return Err("No command found".to_string());
    }

    Ok(CommandCall {
        command: args[0].clone(),
        args: args[1..].to_vec(),
    })
}

/*
fn read_last_command_from_hist_file() -> Result<CommandCall, String> {
    let filename = std::env::var("HISTFILE")
        .map_err(|_| "No HISTFILE configured, cannot use -r".to_string())?;

    let file = std::fs::File::open(&filename)
        .map_err(|err| format!("Failed to open history file: '{}', {}", &filename, err))?;

    // Read the second to last line from the file 
    // The last line is the current call to ds
    unimplemented!();

    parse_command(parts)
}
*/



#[cfg(test)]
mod test {
    use super::*;

    fn string_args(args: &[&str]) -> Vec<String> {
        args.into_iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn can_read_single_char_config() {
        let input = string_args(&["-cl", "-r"]);
        let config = parse_config(input);

        let config = config.unwrap();
        assert!(config.no_colour);
        assert!(config.last_files);
        assert!(config.replay_last);
        assert!(!config.print_help);
    }

    #[test]
    fn rejects_unrecognized_chars() {
        let input = string_args(&["-p"]);
        let config = parse_config(input);
        assert!(config.is_err());
    }

    #[test]
    fn can_read_long_config() {
        let input = string_args(&["--no-colour", "--replay-last"]);
        let config = parse_config(input);

        let config = config.unwrap();
        assert!(config.no_colour);
        assert!(config.replay_last);
        assert!(!config.print_help);
    }

    #[test]
    fn rejects_unknown_long_config() {
        let input = string_args(&["--foo"]);
        let config = parse_config(input);
        assert!(config.is_err());
    }

    #[test]
    fn can_parse_command_with_no_args() {
        let input = string_args(&["vi"]);
        let command = parse_command(input);

        let command = command.unwrap();
        assert_eq!(command.command, "vi");
        assert_eq!(command.args.len(), 0);
    }

    #[test]
    fn can_parse_command_with_args() {
        let input = string_args(&["vi", "a_file.txt"]);
        let command = parse_command(input);

        let command = command.unwrap();
        assert_eq!(command.command, "vi");
        assert_eq!(command.args.len(), 1);
        assert_eq!(command.args[0], "a_file.txt");
    }

    #[test] 
    fn can_parse_whole_line() {
        let input = string_args(&["ds", "-c", "--help", "alr", "build"]);
        let parsed_input = read_args(input);

        assert!(parsed_input.is_ok());

        let (command, config) = parsed_input.unwrap(); 

        assert_eq!(command.command, "alr");
        assert_eq!(command.args.len(), 1);
        assert_eq!(command.args[0], "build");

        assert!(config.no_colour);
        assert!(config.print_help);
    }

    #[test] 
    fn can_parse_with_no_config() {
        let input = string_args(&["ds", "alr", "build"]);
        let parsed_input = read_args(input);

        assert!(parsed_input.is_ok());

        let (command, _config) = parsed_input.unwrap(); 

        assert_eq!(command.command, "alr");
        assert_eq!(command.args.len(), 1);
        assert_eq!(command.args[0], "build");
    }
}
