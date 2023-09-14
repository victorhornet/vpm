use std::collections::BTreeMap;
use std::env;
use std::fmt::Display;
use std::fs;
use std::process::Command;
use std::process::Stdio;

use chrono::Local;
use chrono::NaiveDate;

use clap::Parser;
use clap::Subcommand;

mod tui;

#[derive(Debug, Clone)]
pub struct Project {
    pub id: usize,
    pub name: String,
    pub date: NaiveDate,
}

impl Project {
    pub fn get_path(&self) -> String {
        format!("{}/{}", env::var("PROJECT_HOME").unwrap(), self)
    }
}

impl Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let date = self.date.format("%Y-%m-%d").to_string();
        write!(f, "p{:02X}-{}-{}", self.id, self.name, date)
    }
}

/// Simple program to manage projects
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "List all projects")]
    List,
    #[command(about = "Create a new project")]
    New {
        #[clap(help = "Name of the project")]
        name: String,
    },
    #[command(about = "Open a project in VSCode")]
    Code {
        #[clap(help = "Decimal ID of the project")]
        id: usize,
    },
    #[command(about = "Get the path of a project")]
    Path {
        #[clap(help = "Decimal ID of the project")]
        id: usize,
    },
    #[command(about = "Rename a project")]
    Rename {
        #[clap(help = "Decimal ID of the project")]
        id: usize,
        #[clap(help = "New name of the project")]
        name: String,
    },
    #[command(about = "Search for a project")]
    Search {
        #[clap(help = "Pattern to search for")]
        pattern: String,
    },
}

fn main() {
    let path_str = match env::var("PROJECT_HOME") {
        Ok(path) => path,
        Err(_) => {
            println!(
                "You must set the $PROJECT_HOME variable to the root of your projects folder!"
            );
            return;
        }
    };
    let projects = read_files(path_str);

    let args = Args::parse();
    match args.command {
        Some(Commands::List) => {
            projects.iter().for_each(|(id, project)| {
                println!("{id:3}: {project}");
            });
        }
        Some(Commands::New { name }) => {
            let id = projects.last_key_value().unwrap().0 + 1;
            let date = Local::now().date_naive();
            let name = format_name(&name).unwrap();
            let project = Project { id, name, date };
            Command::new("mkdir")
                .arg(project.get_path())
                .output()
                .unwrap();
            println!("Created project {} with id {}", &project, id);
        }
        Some(Commands::Rename { id, name }) => {
            let project = projects.get(&id).unwrap();
            let new_name = format_name(&name).unwrap();
            let new_project = Project {
                id,
                name: new_name,
                date: project.date,
            };
            Command::new("mv")
                .arg(project.get_path())
                .arg(new_project.get_path())
                .output()
                .unwrap();
            println!("Renamed project: {}", &new_project);
        }
        Some(Commands::Path { id }) => {
            let project = projects.get(&id).unwrap();
            println!("{}", project.get_path());
        }
        Some(Commands::Code { id }) => {
            let project = projects.get(&id).unwrap();
            let path = project.get_path();
            Command::new("code")
                .arg(path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
        }
        Some(Commands::Search { pattern }) => {
            projects
                .iter()
                .filter(|(_, project)| project.to_string().contains(&pattern))
                .for_each(|(id, project)| {
                    println!("{id:02}: {project}");
                });
        }

        #[allow(unreachable_patterns)]
        Some(c) => {
            unimplemented!("{:?}", c);
        }
        None => {
            tui::start(projects).unwrap();
        }
    }
}

fn format_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Name must not be empty!".to_string());
    }
    Ok(name
        .replace(|c: char| !c.is_ascii(), "")
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-"))
}

fn read_files(path: impl Into<String>) -> BTreeMap<usize, Project> {
    fs::read_dir(path.into())
        .unwrap()
        .filter(|project| {
            project
                .as_ref()
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .starts_with('p')
        })
        .map(|project| {
            let project_vec: Vec<String> = project
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .to_string()
                .split('-')
                .map(|s| s.to_string())
                .collect();
            let id = usize::from_str_radix(&project_vec[0][1..], 16).unwrap();
            let name = project_vec[1..project_vec.len() - 3].join("-");
            let date = NaiveDate::parse_from_str(
                project_vec[project_vec.len() - 3..=project_vec.len() - 1]
                    .join("-")
                    .as_str(),
                "%Y-%m-%d",
            )
            .expect("Could not parse date");
            (id, Project { id, name, date })
        })
        .collect()
}
