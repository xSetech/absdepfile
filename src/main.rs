//! Simple tool to resolve and writeback depfile paths

use std::env::{Args, args};
use std::fs::{read_to_string, write};
use std::iter::Skip;
use std::path::Path;
use std::process::exit;


/// Helpful information about the CLI printed out via --help
const HELP: &str = "
Usage: absdepfile [OPTION] DEFILE DIR

Reads the given dependency file generated by passing the '-MF' flag
to a C compiler (e.g. clang). Paths are parsed from the read content
and are resolved relative to the given directory. The resolved paths
are then written back to the file.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, version 3 of the License. This program
comes with ABSOLUTELY NO WARRANTY.

Arguments and options:
    DEPFILE             Path to a file
    DIR                 Path to a directory
    --help, -h          Print the text you're currently reading.
    --verbose, -v       Print the thought process of this program.
";


/// Entry point
fn main() {

    let arguments: Skip<Args> = args().skip(1);
    if arguments.len() == 0 {
        println!("{}", HELP.trim());
        exit(0);
    }

    let mut verbose: bool = false;
    let mut depfile_arg: Option<String> = None;
    let mut dir_arg: Option<String> = None;

    for argument in arguments {

        if argument.starts_with("-") {
            if argument == "--help" || argument == "-h" {
                println!("{}", HELP.trim());
                exit(0);
            }
            if argument == "--verbose" || argument == "-v" {
                verbose = true;
                continue;
            }
            eprintln!("error: unknown flag: {}", argument);
            exit(1);
        }

        if depfile_arg.is_none() {
            depfile_arg = Some(argument);
            continue;
        }

        if dir_arg.is_none() {
            dir_arg = Some(argument);
            continue;
        }

        eprintln!("error: unknown argument: {}", argument);
        exit(1);

    }

    if depfile_arg.is_none() || dir_arg.is_none() {
        eprintln!("error: insufficient arguments");
        exit(1);
    }

    let dir_string: String = dir_arg.unwrap();
    let dir_path: &Path = Path::new(dir_string.as_str());
    if verbose {
        eprintln!("debug: directory: {}", dir_string);
    }

    if !dir_path.is_absolute() {
        eprintln!("error: directory must be absolute: {}", dir_path.display());
        exit(1);
    }

    let depfile_string: String = depfile_arg.unwrap();
    if verbose {
        eprintln!("debug: file: {}", depfile_string);
    }

    let depfile_read_result = read_to_string(&depfile_string);
    if let Err(err) = depfile_read_result {
        eprintln!("error: cannot read contents of file ({}): {}", &depfile_string, err);
        exit(1);
    }

    let depfile_contents: String = depfile_read_result.unwrap();
    if verbose {
        eprintln!("debug: file content size: {}", &depfile_contents.len());
    }

    let mut depfile_contents_iter = depfile_contents.split_whitespace();

    let depfile_target: Option<&str> = depfile_contents_iter.next();
    if depfile_target.is_none() {
        eprintln!("error: file ({}) has no content", &depfile_string);
        exit(1);
    }

    let depfile_target: &str = depfile_target.unwrap();
    let depfile_target: &str = depfile_target.trim_end_matches(":");  // "foo.c:" -> "foo.c"
    if verbose {
        eprintln!("debug: depfile target: {}", depfile_target);
    }

    let depfile_target_path: &Path = Path::new(depfile_target);
    if !depfile_target_path.is_absolute() {
        eprintln!("error: depfile target ({}) is not absolute, correct your compiler argument", depfile_target);
        exit(1);
    }

    let mut buffer: Vec<u8> = Vec::new();

    let new_target = format!("{}: \\\n", depfile_target);
    if verbose {
        println!("{}", new_target);
    }
    buffer.extend(new_target.as_bytes().iter());

    for depfile_part in depfile_contents_iter {

        if depfile_part == r#"\"# {
            continue;
        }

        let depfile_part_path: &Path = Path::new(depfile_part);
        if depfile_part_path.is_absolute() {
            let part: String = format!("    {} \\\n", depfile_part);
            if verbose {
                println!("{}", part);
            }
            buffer.extend(part.as_bytes().iter());
            continue;
        }

        let depfile_part_resolved = dir_path.join(depfile_part_path).canonicalize();
        match depfile_part_resolved {
            Ok(path) => {
                let part: String = format!("    {} \\\n", path.display());
                if verbose {
                    println!("{}", part);
                }
                buffer.extend(part.as_bytes().iter());
                continue;
            },
            Err(err) => {
                eprintln!("error: failed to resolve path ({}): {}", depfile_part, err);
                exit(1)
            }
        }

    }

    let _ = write(depfile_string, buffer).expect("error: failure to write back file");

}