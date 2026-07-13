use std::{
    borrow::Cow,
    fs::canonicalize,
    path::{Path, PathBuf, absolute},
    process::Command,
};

use anyhow::{Result, anyhow};
use arboard::Clipboard;
use cliclack::{MultiSelect, confirm, input, intro, log, multiselect, note, outro, select};
use console::style;

#[derive(Clone, PartialEq, Eq)]
enum Action {
    AddWorktree,
    DeleteWorktree,
}

fn main() -> Result<()> {
    intro(
        style("I see you are lazy as ever habibi 🙈")
            .on_blue()
            .black(),
    )?;

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
    print_existing_worktrees("Existing worktrees")?;

    let worktree_name: String = input("What's the name of your worktree?")
        .placeholder("some-awesome-feature")
        .validate(validate_empty)
        .interact()?;

    let wt_path = add_worktree(&worktree_name)?;

    let copied_to_clipboard = Clipboard::new()
        .and_then(|mut v| v.set_text(wt_path.to_string_lossy()))
        .is_ok();

    let path = wt_path.to_string_lossy();
    let message = if copied_to_clipboard {
        format!("✅ Worktree successfully created. Path copied to clipboard 📋 {path}")
    } else {
        format!("✅ Worktree successfully created at {path}")
    };

    outro(message)?;

    Ok(())
}

fn print_existing_worktrees(title: &str) -> Result<()> {
    let existing = list_worktree()?
        .iter()
        .map(|v| format!("{} {}", v.path, v.branch))
        .collect::<Vec<String>>()
        .join("\n");

    note(title, existing)?;

    Ok(())
}

fn delete_worktree_cmd() -> Result<()> {
    let existing = list_worktree()?;
    let mut multi_select: MultiSelect<Worktree> =
        multiselect("Select one or more worktree to remove");

    for wt in existing {
        multi_select = multi_select.item(wt.clone(), wt.branch, wt.path);
    }

    let selected = multi_select.interact()?;

    for v in selected {
        remove_worktree(&v)?;
    }

    print_existing_worktrees("Remaining worktrees")?;

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

#[derive(Debug, Clone, Eq)]
struct Worktree {
    path: String,
    _head: String,
    branch: String,
}

impl PartialEq for Worktree {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

fn list_worktree() -> Result<Vec<Worktree>> {
    let cmd = Command::new("git").args(["worktree", "list"]).output()?;
    let output = String::from_utf8(cmd.stdout)?;

    let data: Vec<Worktree> = output
        .lines()
        .map(|v| {
            let mut parts = v.split_whitespace().map(String::from);
            let path = parts.next().unwrap_or_default();

            Worktree {
                path,
                _head: parts.next().unwrap_or_default(),
                branch: parts.next().unwrap_or_default(),
            }
        })
        .collect();

    Ok(data)
}

// best effort
fn is_worktree_clean(path: &str) -> bool {
    if let Ok(cmd) = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
    {
        return cmd.stdout.is_empty();
    }

    true
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

fn remove_worktree(worktree: &Worktree) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["worktree", "remove", &worktree.path]);

    if !is_worktree_clean(&worktree.path) {
        if !confirm(format!(
            "Worktree {} is not clean, delete anyway?",
            worktree.branch
        ))
        .interact()?
        {
            return Ok(());
        }

        cmd.arg("-f");
    }

    let res = cmd.output()?;

    if res.status.success() {
        Ok(())
    } else {
        let output = String::from_utf8(res.stderr)?;
        Err(anyhow!(output))
    }
}

fn resolve_path(name: &str) -> Cow<'_, str> {
    if Path::new(name).components().count() == 1 {
        Cow::Owned(format!("../{name}"))
    } else {
        Cow::Borrowed(name)
    }
}
