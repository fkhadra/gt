use std::{
    borrow::Cow,
    fs::canonicalize,
    path::{self, Path, PathBuf, absolute},
    process::{Command, exit},
};

use anyhow::{Result, anyhow, bail};
use cliclack::{MultiSelect, input, intro, log, multiselect, note, outro, select};

#[derive(Clone, PartialEq, Eq)]
enum Action {
    AddWorktree,
    DeleteWorktree,
}

fn main() -> Result<()> {
    intro("I see you are lazy as ever habibi 🙈")?;

    let selected = select("What do you want to do?")
        .item(Action::AddWorktree, "🌴 Create a new worktree", "")
        .item(Action::DeleteWorktree, "🧹 Clean worktrees", "")
        .interact()?;

    let res = match selected {
        Action::AddWorktree => add_worktree_cmd(),
        Action::DeleteWorktree => delete_worktree_cmd(),
    };

    if let Err(e) = res {
        log::error(e)?;
    }

    Ok(())
}

fn add_worktree_cmd() -> Result<()> {
    let existing = list_worktree()?
        .iter()
        .map(|v| format!("{} {}", v.path, v.branch))
        .collect::<Vec<String>>()
        .join("\n");

    note("Existing worktree", existing)?;

    let worktree_name: String = input("What's the name of your worktree?")
        .placeholder("some-awesome-feature")
        .validate(validate_empty)
        .interact()?;

    let wt_path = add_worktree(&worktree_name)?;

    outro(format!(
        "✅ Worktree successfully created at {}",
        wt_path.to_string_lossy()
    ))?;

    Ok(())
}

fn delete_worktree_cmd() -> Result<()> {
    let existing = list_worktree()?;
    let mut multi_select: MultiSelect<String> =
        multiselect("Select one or more worktree to remove");

    for wt in existing {
        multi_select = multi_select.item(wt.path.clone(), wt.branch, wt.path);
    }

    let selected = multi_select.interact()?;

    for v in selected {
        remove_worktree(&v)?;
    }

    outro("✅ Feels good to tidy up")?;

    Ok(())
}

fn validate_empty(value: &String) -> Result<(), &'static str> {
    if value.is_empty() {
        Err("Value is required!")
    } else {
        Ok(())
    }
}

#[derive(Debug)]
struct Worktree {
    path: String,
    _head: String,
    branch: String,
}

fn list_worktree() -> Result<Vec<Worktree>> {
    let cmd = Command::new("git").args(["worktree", "list"]).output()?;
    let output = String::from_utf8(cmd.stdout)?;

    let data: Vec<Worktree> = output
        .lines()
        .map(|v| {
            let mut parts = v.split_whitespace().map(String::from);

            Worktree {
                path: parts.next().unwrap_or_default(),
                _head: parts.next().unwrap_or_default(),
                branch: parts.next().unwrap_or_default(),
            }
        })
        .collect();

    Ok(data)
}

fn add_worktree(name: &str) -> Result<PathBuf> {
    let wt_path = resolve_path(name);
    let abs_path = absolute(wt_path.as_ref())?;

    let cmd = Command::new("git")
        .args(["worktree", "add", wt_path.as_ref()])
        .output()?;

    match cmd.status.success() {
        true => Ok(canonicalize(abs_path)?),
        false => {
            let output = String::from_utf8(cmd.stderr)?;
            Err(anyhow!(output))
        }
    }
}

fn remove_worktree(path: &str) -> Result<()> {
    let cmd = Command::new("git")
        .args(["worktree", "remove", path])
        .output()?;

    match cmd.status.success() {
        true => Ok(()),
        false => {
            let output = String::from_utf8(cmd.stderr)?;
            Err(anyhow!(output))
        }
    }
}

fn resolve_path(name: &str) -> Cow<'_, str> {
    if Path::new(name).components().count() == 1 {
        Cow::Owned(format!("../{name}"))
    } else {
        Cow::Borrowed(name)
    }
}
