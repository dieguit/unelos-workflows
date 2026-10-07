use camino::Utf8Path;
use serde::Serialize;
use std::process;
use unelos_core::{load_project, next, start, status, RuntimeError};

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result: Result<serde_json::Value, Box<dyn std::error::Error>> = match args.as_slice() {
        [command, project, workflow, name, request] if command == "start" => (|| {
            let loaded = load_project(Utf8Path::new(project), workflow)?;
            Ok(serde_json::to_value(start(&loaded, name, request)?)?)
        })(),
        [command, project] if command == "status" => json(status(Utf8Path::new(project))),
        [command, project] if command == "next" => json(next(Utf8Path::new(project))),
        _ => {
            eprintln!(
                "usage:\n  run_qa start PROJECT WORKFLOW NAME REQUEST\n  run_qa status PROJECT\n  run_qa next PROJECT"
            );
            process::exit(2);
        }
    };

    match result {
        Ok(value) => match serde_json::to_string_pretty(&value) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("{error}");
                process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

fn json<T: Serialize>(
    result: Result<T, RuntimeError>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::to_value(result?)?)
}
