use chrono::{DateTime, Local, NaiveDate, NaiveTime};
use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::eyre::{anyhow, Result};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use std::{
    collections::BTreeMap,
    env,
    fmt::Display,
    fs,
    path::Path,
    process::{Command, Stdio},
    str::FromStr,
};
mod shells;
mod tui;

#[derive(Default, Clone, Debug, Copy)]
pub enum Status {
    #[default]
    Active,
    Paused,
    Archived,
}

impl FromStr for Status {
    type Err = color_eyre::eyre::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Status::Active),
            "paused" => Ok(Status::Paused),
            "archived" => Ok(Status::Archived),
            _ => Err(anyhow!("Invalid status")),
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Active => write!(f, "Active"),
            Status::Paused => write!(f, "Paused"),
            Status::Archived => write!(f, "Archived"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Project {
    pub id: usize,
    pub name: String,
    pub date: NaiveDate,
    pub last_accessed: DateTime<Local>,
    pub status: Status,
    args: Option<Args>,
}

impl Project {
    pub fn new(
        id: usize,
        name: impl Into<String>,
        date: NaiveDate,
        last_accessed: DateTime<Local>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            date,
            last_accessed,
            status: Status::default(),
            args: None,
        }
    }
    pub fn get_path(&self) -> String {
        format!(
            "{}/{}/{}",
            env::var("PROJECT_HOME").unwrap(),
            self.status,
            self.full_name()
        )
    }
    pub fn with_args(mut self, args: &Args) -> Self {
        self.args = Some(args.to_owned());
        self
    }
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
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
                if args.accessed {
                    write!(f, "({}) ", self.last_accessed)?;
                }
                if args.full_name {
                    write!(f, "{} ", self.full_name())?;
                } else if !args.no_name {
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
    #[arg(short, long, help = "Print the full name of the projects")]
    full_name: bool,
    #[arg(short, long, help = "Don't print the name of the projects")]
    no_name: bool,
    #[arg(short, long, help = "Print the time the projects were last accessed")]
    accessed: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    #[command(about = "List all projects")]
    List {
        #[arg(short, long, help = "Sort", default_value = "id")]
        sort: Sort,
        #[arg(short, long, help = "Reverse the sort")]
        reverse: bool,
        #[arg(
            short,
            long,
            default_value = "0",
            help = "Limit the number of results, 0 for no limit"
        )]
        limit: usize,
    },
    #[command(about = "Create a new project")]
    New {
        #[clap(help = "Name of the project")]
        name: String,
        #[arg(short, long, help = "Template to use")]
        template: Option<String>,
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
    #[command(about = "Init shell bindings. This will create two functions: j and pj.")]
    Init {
        #[command(subcommand)]
        shell: InitShells,
    },
    #[command(about = "Create a new template from a project")]
    Template {
        #[clap(help = "ID of the project")]
        id: usize,
        #[clap(help = "Name of the template")]
        name: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum TemplateCommands {
    #[command(about = "List all templates")]
    List,
    #[command(about = "Create a new template")]
    New {
        #[clap(help = "Name of the template")]
        name: String,
        #[clap(help = "ID of the project")]
        id: usize,
    },
    #[command(about = "Delete a template")]
    Delete {
        #[clap(help = "Name of the template")]
        name: String,
    },
}

#[derive(Debug, Clone, Default, ValueEnum)]
enum Sort {
    #[default]
    Id,
    Name,
    #[clap(alias = "date")]
    Created,
    Accessed,
}

#[derive(Debug, Clone, Subcommand, Default)]
enum InitShells {
    #[default]
    #[command(about = "Init fish shell")]
    Fish,
    #[command(about = "Init zsh shell")]
    Zsh,
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

    let projects = read_files(&path_str, &args);
    match args.command {
        Some(Commands::List {
            sort,
            reverse,
            limit,
        }) => {
            projects
                .values()
                .sorted_by(|a, b| {
                    let ordering = match sort {
                        Sort::Id => a.id.cmp(&b.id),
                        Sort::Name => a.name.cmp(&b.name),
                        Sort::Created => a.date.cmp(&b.date),
                        Sort::Accessed => a.last_accessed.cmp(&b.last_accessed),
                    };
                    if reverse {
                        ordering.reverse()
                    } else {
                        ordering
                    }
                })
                .take(if limit > 0 { limit } else { usize::MAX })
                .for_each(|project| {
                    println!("{}", project);
                });
        }
        Some(Commands::New {
            ref name,
            ref template,
        }) => {
            let id = projects.last_key_value().unwrap().0 + 1;
            let date = Local::now().date_naive();
            let name = format_name(name).unwrap();
            let project = Project::new(id, name, date, Local::now()).with_args(&args);
            match template {
                Some(template) => {
                    let template_path = Path::new(&path_str).join("templates").join(&template);
                    if !template_path.exists() {
                        return Err(anyhow!("Template does not exist!"));
                    }
                    Command::new("cp")
                        .arg("-r")
                        .arg(template_path)
                        .arg(project.get_path())
                        .output()
                        .unwrap();
                }
                None => {
                    Command::new("mkdir")
                        .arg(project.get_path())
                        .output()
                        .unwrap();
                }
            }
            println!("{}", &project);
        }
        Some(Commands::Rename { id, name }) => {
            let project = projects.get(&id).unwrap();
            let new_name = format_name(&name).unwrap();
            let new_project = Project::new(id, new_name, project.date, Local::now());
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
        Some(Commands::Template { name, id }) => {
            let project = projects.get(&id).unwrap();
            let project_path = project.get_path();
            let templates_root = Path::new(&path_str).join("templates");
            if !templates_root.exists() {
                Command::new("mkdir").arg(&templates_root).output().unwrap();
            }
            let template_path = templates_root.join(&name);
            Command::new("cp")
                .arg("-r")
                .arg(project_path)
                .arg(template_path)
                .output()
                .unwrap();
        }
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
        .filter_map(|res| res.ok())
        .filter(|entry| {
            entry.file_type().is_ok_and(|ftype| ftype.is_dir())
                && entry
                    .file_name()
                    .into_string()
                    .is_ok_and(|name| Status::from_str(name.as_str()).is_ok())
        })
        .map(|dir| {
            let status = Status::from_str(dir.file_name().into_string().unwrap().as_str()).unwrap();
            let path = dir.path();
            fs::read_dir(path)
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
                    let project = project.unwrap();
                    let project_vec: Vec<String> = project
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
                    let modified: DateTime<Local> = project
                        .metadata()
                        .unwrap()
                        .accessed()
                        .map(|time| time.into())
                        .unwrap_or(
                            date.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                                .and_local_timezone(Local)
                                .unwrap(),
                        );
                    (
                        id,
                        Project::new(id, name, date, modified)
                            .with_args(args)
                            .with_status(status),
                    )
                })
                .collect_vec()
        })
        .concat()
        .into_iter()
        .collect()
}

fn init_shell(shell: InitShells) -> Result<()> {
    match shell {
        InitShells::Fish => shells::init_fish(),
        InitShells::Zsh => shells::init_zsh(),
        #[allow(unreachable_patterns)]
        sh => unimplemented!("init_shell({sh:?})"),
    }
}
