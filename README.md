# Vector's Project Manager

Small CLI tool to manage my projects folder.

## Installation

Install with `cargo`:

```bash
cargo install vector-project-manager
```

## Usage

```bash
vpm [COMMAND]
```

For a list of commands, run `vpm --help`.

## Shells integrations

Example shortcuts using `vpm`.

### zsh

```shell
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
```

### fish

Fish functions can be installed with `vpm init fish`.
> ⚠️ This will overwrite your `~/.config/fish/functions/j.fish` and `~/.config/fish/functions/pj.fish` files.

```shell
# cd into a project by its ID.
# Usage: pj <ID>
# Example: pj 1
function pj
    set path (vpm path $argv[1])
    if set -q path[1]
        cd $path
    end
end

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
```
