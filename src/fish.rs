use std::{env, path::PathBuf};

use color_eyre::eyre::Result;

const J_FILENAME: &str = "j.fish";
const J_FUNCTION: &str = r#"
# j
function j
    set path (vpm -p search -l 1 (echo $argv))
    if set -q path[1]
        cd $path
    else
        echo "No project was found for query: $argv"
    end
end
"#;

const PJ_FILENAME: &str = "pj.fish";
const PJ_FUNCTION: &str = r#"
# pj
function pj
    set path (vpm path $argv[1])
    if set -q path[1]
        cd $path
    end
end
"#;

fn bind_function(filename: &str, function: &str) -> Result<()> {
    let function_path = PathBuf::from(env::var("HOME")?)
        .join(".config")
        .join("fish")
        .join("functions")
        .join(filename);
    std::fs::write(function_path, function)?;
    Ok(())
}

pub fn init() -> Result<()> {
    bind_function(J_FILENAME, J_FUNCTION)?;
    bind_function(PJ_FILENAME, PJ_FUNCTION)?;
    Ok(())
}
