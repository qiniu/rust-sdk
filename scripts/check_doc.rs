#!/usr/bin/env run-cargo-script

//! ```cargo
//! [dependencies]
//! clap = "2.33.0"
//! tempfile = "3.1.0"
//! ```

extern crate clap;
extern crate tempfile;

use clap::{App, Arg};
use std::{
    fs::File,
    io::{BufRead, BufReader, Error, ErrorKind, Result, Write},
    process::{Command, Stdio},
};

fn main() {
    let matches = App::new("Qiniu usage document code checker")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS").split(':').last().unwrap())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("usage-file")
                .long("usage-file")
                .required(true)
                .value_name("FILE")
                .help("To check doc from this Usage file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("language")
                .long("lang")
                .required(true)
                .value_name("LANG")
                .help("Programming language in Usage file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("language-ext")
                .long("ext")
                .required(true)
                .value_name("EXT")
                .help("Ext name for the Programming language")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("check-command")
                .long("cmd")
                .required(true)
                .value_name("CMD")
                .help("Specify command to check code")
                .takes_value(true),
        )
        .get_matches();

    let command = matches.value_of("check-command").unwrap();
    let language_ext_name = matches.value_of("language-ext").unwrap();
    let code_block_start_matches = "```".to_owned() + matches.value_of("language").unwrap() + "\n";
    let code_block_end_matches = "```\n";
    let mut file =
        BufReader::new(File::open(matches.value_of_os("usage-file").unwrap()).expect("Failed to open usage file"));
    let mut line = String::new();
    let mut code_buffer: Option<String> = None;
    for line_number in 1.. {
        match file.read_line(&mut line).expect("Failed to read usage file") {
            0 => {
                break;
            }
            _ => match &mut code_buffer {
                Some(buffer) if line == code_block_end_matches => {
                    if check_code_block(&buffer, &command, &language_ext_name).is_err() {
                        panic!(
                            "Code block test failed at {}:{}",
                            matches.value_of("usage-file").unwrap(),
                            line_number
                        );
                    }
                    code_buffer = None;
                }
                Some(buffer) => {
                    buffer.push_str(&line);
                }
                None if line == code_block_start_matches => {
                    if code_buffer.is_some() {
                        panic!("Matches code block begin, but the previous code block is not ended");
                    } else {
                        code_buffer = Some(String::new());
                    }
                }
                None => {}
            },
        }
        line.clear();
    }
}

fn check_code_block(code_block: &str, command: &str, language_ext_name: &str) -> Result<()> {
    let language_ext_name = ".".to_owned() + language_ext_name;
    let mut file = tempfile::Builder::new().suffix(&language_ext_name).tempfile()?;
    file.write_all(code_block.as_bytes())?;
    let file_path = file.into_temp_path();
    let mut cmd: Option<Command> = None;
    command.split(char::is_whitespace).for_each(|arg| match &mut cmd {
        Some(cmd) => {
            if arg == "{}" {
                cmd.arg(&file_path);
            } else {
                cmd.arg(arg);
            }
        }
        None => {
            cmd = Some(Command::new(arg));
        }
    });
    if let Some(mut cmd) = cmd {
        if !cmd
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?
            .success()
        {
            Err(Error::new(ErrorKind::Other, "Command returns non-zero"))
        } else {
            Ok(())
        }
    } else {
        Err(Error::new(ErrorKind::Other, "Empty command cannot be executed"))
    }
}
