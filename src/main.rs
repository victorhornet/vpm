use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand};
use color_eyre::eyre::{anyhow, Result};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use std::{
    collections::BTreeMap,
    env,
    fmt::Display,
    fs,
    process::{Command, Stdio},
};
mod fish;
mod tui;

#[derive(Debug, Clone)]
pub struct Project {
    pub id: usize,
    pub name: String,
    pub date: NaiveDate,
    args: Option<Args>,
}

impl Project {
    pub fn new(id: usize, name: impl Into<String>, date: NaiveDate) -> Self {
        Self {
            id,
            name: name.into(),
            date,
            args: None,
        }
    }
    pub fn get_path(&self) -> String {
        format!("{}/{}", env::var("PROJECT_HOME").unwrap(), self.full_name())
    }
    pub fn with_args(mut self, args: &Args) -> Self {
        self.args = Some(args.to_owned());
        self
    }
    pub fn full_name(&self) -> String {
        format!(
            "p{:02X}-{}-{}",
            self.id,
            self.name,
            self.date.format("%Y-%m-%d")
        )
    }
}

impl Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.args {
            Some(args) => {
                if args.path {
                    return write!(f, "{}", self.get_path());
                }
                if args.id {
                    write!(f, "{:02} ", self.id)?;
                }
                if args.date {
                    write!(f, "{} ", self.date)?;
                }
                if args.full_name {
                    write!(f, "{} ", self.full_name())?;
                } else {
                    write!(f, "{} ", self.name)?;
                }
                Ok(())
            }
            None => write!(f, "{}", self.full_name()),
        }
    }
}

/// Simple program to manage projects
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, help = "Print the id of the projects")]
    id: bool,
    #[arg(short, long, help = "Print the path of the projects")]
    path: bool,
    #[arg(short, long, help = "Print the date of the projects")]
    date: bool,
    #[arg(short, long, help = "Print the full name of the project directories")]
    full_name: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
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
        #[arg(
            short,
            long,
            default_value = "0",
            help = "Limit the number of results, 0 for no limit"
        )]
        limit: usize,
    },
    #[command(about = "Init shell bindings")]
    Init {
        #[command(subcommand)]
        shell: InitShells,
    },
}

#[derive(Debug, Clone, Subcommand, Default)]
enum InitShells {
    #[default]
    #[command(about = "Init fish shell. This will create two functions: j and pj.")]
    Fish,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let path_str = match env::var("PROJECT_HOME") {
        Ok(path) => path,
        Err(_) => {
            return Err(anyhow!(
                "You must set the $PROJECT_HOME variable to the root of your projects folder!"
            ));
        }
    };

    let projects = read_files(path_str, &args);
    match args.command {
        Some(Commands::List) => {
            projects.values().for_each(|project| {
                println!("{}", project);
            });
        }
        Some(Commands::New { name }) => {
            let id = projects.last_key_value().unwrap().0 + 1;
            let date = Local::now().date_naive();
            let name = format_name(&name).unwrap();
            let project = Project::new(id, name, date);
            Command::new("mkdir")
                .arg(project.get_path())
                .output()
                .unwrap();
            println!("Created project {} with id {}", &project, id);
        }
        Some(Commands::Rename { id, name }) => {
            let project = projects.get(&id).unwrap();
            let new_name = format_name(&name).unwrap();
            let new_project = Project::new(id, new_name, project.date);
            Command::new("mv")
                .arg(project.get_path())
                .arg(new_project.get_path())
                .output()
                .unwrap();
            println!("Renamed project: {}", &new_project);
        }
        Some(Commands::Path { id }) => {
            let project = projects
                .get(&id)
                .ok_or(anyhow!("Project {id} not found!"))?;
            println!("{}", project.get_path());
        }
        Some(Commands::Code { id }) => {
            let project = projects
                .get(&id)
                .ok_or(anyhow!("Project {id} not found!"))?;
            let path = project.get_path();
            Command::new("code")
                .arg(path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
        }
        Some(Commands::Search { pattern, limit }) => {
            let matcher = SkimMatcherV2::default();
            projects
                .values()
                .filter_map(|project| {
                    let score = matcher.fuzzy_match(&project.to_string(), &pattern);
                    score.map(|score| (project, score))
                })
                .sorted_by(|(_, score1), (_, score2)| score2.cmp(score1))
                .take(if limit > 0 { limit } else { usize::MAX })
                .for_each(|(project, _)| {
                    println!("{project}");
                });
        }
        Some(Commands::Init { shell }) => init_shell(shell)?,
        #[allow(unreachable_patterns)]
        Some(c) => {
            unimplemented!("{:?}", c);
        }
        None => {
            tui::start(projects).unwrap();
        }
    }
    Ok(())
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

fn read_files(path: impl Into<String>, args: &Args) -> BTreeMap<usize, Project> {
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
            (id, Project::new(id, name, date).with_args(args))
        })
        .collect()
}

fn init_shell(shell: InitShells) -> Result<()> {
    match shell {
        InitShells::Fish => fish::init(),
        #[allow(unreachable_patterns)]
        sh => unimplemented!("init_shell({sh:?})"),
    }
}
