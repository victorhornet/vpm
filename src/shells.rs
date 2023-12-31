use color_eyre::eyre::Result;
use std::fs::File;
use std::io::ErrorKind;
use std::{env, io::Write, path::PathBuf};

const FISH_PJ_FILENAME: &str = ".config/fish/functions/pj.fish";
const FISH_PJ_FUNCTION: &str = r#"
# cd into a project by its ID.
# Usage: pj <ID>
# Example: pj 1
function pj
    set path (vpm path $argv[1])
    if set -q path[1]
        cd $path
    end
end"#;
const FISH_J_FILENAME: &str = ".config/fish/functions/j.fish";
const FISH_J_FUNCTION: &str = r#"
# Fuzzy search for a project and cd into it.
# Usage: j <QUERY>
# Example: j some-proj
function j
    set path (vpm -p search -l 1 (echo $argv))
    if set -q path[1]
        cd $path
    else
        echo "No project was found for query: $argv"
    end
end
"#;
pub fn init_fish() -> Result<()> {
    bind_functions(FISH_J_FILENAME, FISH_J_FUNCTION)?;
    bind_functions(FISH_PJ_FILENAME, FISH_PJ_FUNCTION)?;
    Ok(())
}

const ZSH_FILENAME: &str = ".zshrc";
const ZSH_FUNCTIONS: &str = r#"
# cd into a project by its ID.
# Usage: pj <ID>
# Example: pj 1
pj() {
    project_path=$(vpm path $1)
    if [ -z "$project_path" ]; then
        return 1
    fi
    cd $project_path
}

# Fuzzy search for a project and cd into it.
# Usage: j <QUERY>
# Example: j some-proj
j() {
    project_path=$(vpm -p search -l 1 $1)
    if [ -z "$project_path" ]; then
        echo "No project found"
        return 1
    fi
    cd $project_path
}
"#;
pub fn init_zsh() -> Result<()> {
    bind_functions(ZSH_FILENAME, ZSH_FUNCTIONS)?;
    Ok(())
}

fn bind_functions(filename: &str, functions: &str) -> Result<()> {
    let function_path = PathBuf::from(env::var("HOME")?).join(filename);
    print!(
        "This will create or open the file at {:?} and append the functions to it.\nDo you want to continue [y/N]? ",
        function_path
    );
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim().to_lowercase() != "y" {
        println!("Aborting...");
        return Ok(());
    }
    let mut file = match File::options().append(true).open(&function_path) {
        Ok(file) => file,
        Err(err) => match err.kind() {
            ErrorKind::NotFound => File::create(&function_path)?,
            _ => return Err(err.into()),
        },
    };
    file.write_all(functions.as_bytes())?;
    println!("Done!");
    Ok(())
}
