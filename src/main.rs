#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

#[macro_use]
extern crate clap;
extern crate dialoguer;
extern crate termcolor;
#[macro_use]
extern crate text_io;

use std::io;
use std::io::Error;
use std::process::Command;

use clap::{App, Arg, ArgMatches};
use dialoguer::Select;
use termcolor::ColorChoice;
use termcolor::StandardStream;

use file_handler::{ConfigManagement, FileHandler, FileManagement};
use git::git::git_commit_and_push;
use printer::{Print, Printer};
use reader::{Read, Reader};
use types::CliFlag::{ClearEditor, ClearRepo, View};
use types::ConfigType::{Editor, Repo};

mod file_handler;
mod git;
mod printer;
mod reader;
mod types;
mod utils;

fn main() {
    let cli_flags: ArgMatches = App::new("eureka")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Input and store your ideas without leaving the terminal")
        .arg(
            Arg::with_name(ClearRepo.value())
                .long(ClearRepo.value())
                .help("Clear the stored path to your idea repo"),
        )
        .arg(
            Arg::with_name(ClearEditor.value())
                .long(ClearEditor.value())
                .help("Clear the stored path to your idea editor"),
        )
        .arg(
            Arg::with_name(View.value())
                .long(View.value())
                .help("View your ideas using less"),
        )
        .get_matches();

    let stdio = io::stdin();
    let input = stdio.lock();
    let output = StandardStream::stdout(ColorChoice::AlwaysAnsi);

    let mut p = Printer { writer: output };
    let mut r = Reader { reader: input };
    let fh = FileHandler {};

    if cli_flags.is_present(ClearRepo.value()) {
        fh.file_rm(Repo).expect("Could not remove repo config file");
    }

    if cli_flags.is_present(ClearEditor.value()) {
        fh.file_rm(Editor)
            .expect("Could not remove editor config file");
    }

    if cli_flags.is_present(ClearRepo.value()) || cli_flags.is_present(ClearEditor.value()) {
        ::std::process::exit(0);
    }

    if cli_flags.is_present("view") {
        match fh.config_read(Repo) {
            Ok(repo_path) => match open_pager_less(repo_path) {
                Ok(_) => {
                    ::std::process::exit(0);
                }
                Err(e) => panic!(e),
            },
            Err(_) => panic!("No path to repository found"),
        }
    }

    // If first time setup is carried out do not
    // require an idea to be written down.
    let mut is_first_time: bool = false;

    let repo_path: String = match fh.config_read(Repo) {
        Ok(file_path) => file_path,
        Err(_) => {
            is_first_time = true;
            p.print_fts_banner();
            if !fh.config_dir_exists() {
                fh.config_dir_create()
                    .expect("Unable to create dir to store config");
            }

            p.print_input_header("Absolute path to your idea repo");
            p.flush().unwrap();
            let input_path: String = read!();
            let copy_input_path: String = input_path.clone();

            match fh.config_write(Repo, input_path) {
                Ok(_) => copy_input_path,
                Err(e) => panic!("Unable to write your repo path to disk: {}", e),
            }
        }
    };

    let editor_path: String = match fh.config_read(Editor) {
        Ok(file_path) => file_path,
        Err(_) => {
            let selections = &[
                "vim (/usr/bin/vi)",
                "nano (/usr/bin/nano)",
                "Other (provide path to binary)",
            ];

            p.print_editor_selection_header();
            let index = Select::new()
                .default(0)
                .items(selections)
                .interact()
                .unwrap();

            let input_path: String = match index {
                0 => s("/usr/bin/vi"),
                1 => s("/usr/bin/nano"),
                2 => {
                    p.print_input_header("Path to editor binary");
                    p.flush().unwrap();
                    let editor_bin_path: String = read!();
                    editor_bin_path
                }
                _ => {
                    // TODO(simeg): Do not fall back, ask user again for options
                    // TODO(simeg): How can the user even end up here?
                    p.println("Invalid option, falling back to vim");
                    s("/usr/bin/vi")
                }
            };

            if !fh.file_exists(&input_path) {
                panic!("Invalid editor path");
            }

            let copy_input_path: String = input_path.clone();
            match fh.config_write(Editor, input_path) {
                Ok(_) => copy_input_path,
                Err(e) => panic!("Unable to write your editor path to disk: {}", e),
            }
        }
    };

    if !is_first_time {
        p.print_input_header(">> Idea summary");
        let commit_msg: String = r.read();
        let readme_path: String = format!("{}/README.md", repo_path);

        match open_editor(&editor_path, &readme_path) {
            Ok(_) => {
                let _ = git_commit_and_push(&repo_path, commit_msg);
            }
            Err(e) => panic!("Could not open editor at path {}: {}", editor_path, e),
        };
    } else {
        p.println("First time setup complete. Happy ideation!");
    }
}

fn open_editor(bin_path: &String, file_path: &String) -> Result<(), Error> {
    match Command::new(bin_path).arg(file_path).status() {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!(
                "Error: Unable to open file [{}] with editor binary at [{}]: {}",
                file_path, bin_path, e
            );
            Err(e)
        }
    }
}

fn open_pager_less(repo_config_file: String) -> Result<(), Error> {
    let readme_path = &(repo_config_file + "/README.md");
    match Command::new(less()).arg(readme_path).status() {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!(
                "Error: Could not open idea file with less at [{}]: {}",
                readme_path, e
            );
            Err(e)
        }
    }
}

fn less() -> String {
    if utils::utils::is_available("less") {
        String::from("less")
    } else {
        panic!("Cannot locate executable - less - on your system")
    }
}

/*
 * Helpers
*/

fn s(string: &str) -> String {
    string.to_owned()
}
