use annotate_snippets::{Annotation, Group, Level, Renderer, Snippet};
use clap::{Command, arg};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;
use yutu::config::Config;
use yutu::lints::INFOS;
use yutu::parser::Diagnostic;
use yutu::{check_source, read_source};

fn cli() -> Command {
    Command::new("yutu")
        .about("Modern Lua linter")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("check")
                .about("Checks the code for errors and warnings")
                .arg(arg!(-v --version <VERSION> "sets the Lua version").required(false))
                .arg(
                    arg!(<PATH> ... "file to check")
                        .value_parser(clap::value_parser!(PathBuf))
                        .required(false),
                ),
        )
        .subcommand(Command::new("init").about("Creates a new yutu.toml configuration"))
        .subcommand(Command::new("list").about("Lists all lints as markdown"))
}

fn output_diags(diags: Vec<Diagnostic<'_>>) -> (String, bool, usize, usize) {
    let renderer = Renderer::styled();
    let mut output = String::new();
    let mut no_error = true;
    let mut error_count = 0;
    let mut warning_count = 0;
    for diag in diags {
        if diag.error {
            error_count += 1;
        } else {
            warning_count += 1;
        }
        let message = renderer.render(&diag.groups);
        no_error &= !diag.error;
        output.push_str(&message);
        output.push_str("\n\n");
    }
    (output, no_error, error_count, warning_count)
}

fn collect_source_files(path: PathBuf, paths: &mut Vec<PathBuf>) {
    fn rec(path: PathBuf, paths: &mut Vec<PathBuf>) {
        if path.is_dir() {
            match fs::read_dir(&path) {
                Ok(dir) => {
                    for entry in dir {
                        match entry {
                            Ok(entry) => rec(entry.path(), paths),
                            Err(err) => {
                                anstream::print!(
                                    "{}",
                                    file_error(path.display().to_string(), err.to_string())
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    anstream::print!(
                        "{}",
                        file_error(path.display().to_string(), err.to_string())
                    );
                }
            }
        } else if let Some(extension) = path.extension()
            && extension == "lua"
        {
            paths.push(path);
        }
    }
    rec(path.to_path_buf(), paths)
}

fn check(path: PathBuf, config: &Config) -> (String, bool, usize, usize) {
    let path_string = path.display().to_string();
    match read_source(&path) {
        Ok(source) => {
            let diags = check_source(&path_string, &source, config);
            output_diags(diags)
        }
        Err(err) => (file_error(path_string, err.to_string()), false, 1, 0),
    }
}

fn file_error(path: String, msg: String) -> String {
    let group = Group::with_title(Level::ERROR.primary_title(msg))
        .element(Snippet::<Annotation>::source("").path(path));
    let renderer = Renderer::styled();
    renderer.render(&[group]) + "\n\n"
}

fn fatal_error(msg: String) -> ! {
    let group = Group::with_title(Level::ERROR.primary_title(msg));
    let renderer = Renderer::styled();
    anstream::print!("{}\n\n", renderer.render(&[group]));
    std::process::exit(1)
}

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("check", sub_matches)) => {
            let lua_minor_version =
                sub_matches
                    .get_one::<String>("version")
                    .map(|version| match version.as_str() {
                        "5.5" => 5,
                        "5.4" => 4,
                        _ => fatal_error("unsupported Lua version".to_string()),
                    });

            let input_paths = sub_matches
                .get_many::<PathBuf>("PATH")
                .into_iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>();
            let mut paths = vec![];
            for input_path in input_paths.into_iter() {
                collect_source_files(input_path, &mut paths);
            }
            let config = match Config::new(lua_minor_version) {
                Ok(config) => config,
                Err(msg) => fatal_error(msg),
            };
            let diags: Vec<_> = paths
                .into_par_iter()
                .map(|path| check(path, &config))
                .collect();
            let mut all_no_error = true;
            let mut global_error_count = 0;
            let mut global_warning_count = 0;
            let file_count = diags.len();
            for (message, no_error, error_count, warning_count) in diags {
                global_error_count += error_count;
                global_warning_count += warning_count;
                all_no_error &= no_error;
                anstream::print!("{message}");
            }
            let summary = format!(
                "found {global_error_count} error{} and {global_warning_count} warning{} in {file_count} file{}",
                if global_error_count != 1 { "s" } else { "" },
                if global_warning_count != 1 { "s" } else { "" },
                if file_count != 1 { "s" } else { "" },
            );
            let level = if global_error_count > 0 {
                Level::ERROR
            } else if global_warning_count > 0 {
                Level::WARNING
            } else {
                Level::NOTE
            }
            .with_name("summary");
            let group = Group::with_title(level.primary_title(summary));
            let renderer = Renderer::styled();
            anstream::print!("{}\n\n", renderer.render(&[group]));
            if !all_no_error {
                std::process::exit(1);
            }
        }
        Some(("init", _sub_matches)) => match std::fs::exists("yutu.toml") {
            Ok(false) => {
                if let Err(err) = std::fs::write("yutu.toml", Config::default().to_string()) {
                    fatal_error(err.to_string())
                }
            }
            Ok(true) => {}
            Err(err) => fatal_error(err.to_string()),
        },
        Some(("list", _sub_matches)) => {
            println!("# Lints\n");
            let mut sorted_infos = INFOS;
            sorted_infos.sort_by(|a, b| a.code.cmp(b.code));

            println!("| code | default level | category | message |");
            println!("|---|---|---|---|");
            for info in sorted_infos.iter() {
                println!("{}", info.to_markdown_table_line());
            }
            println!();
            for info in sorted_infos {
                println!("{}", info.to_markdown());
            }
        }
        _ => cli().print_help().unwrap(),
    }
}
