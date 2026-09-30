// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! The thin `optimize` front end over the Python optimization entry point.
//!
//! The command locates a Python interpreter, invokes the single optimization entry point
//! (`nautilus_trader.optimization.config`) over the configuration file, and lets the child's
//! machine-readable JSON document go straight to this process's standard output. It contains no
//! optimization logic: no search, no objective, and no aggregation. The Python entry point is the
//! same code a notebook calls.
//!
//! The interpreter contract is, in order:
//!
//! 1. `--python`, when given.
//! 2. `NAUTILUS_PYTHON`, when set and non-empty.
//! 3. `VIRTUAL_ENV\Scripts\python.exe` on Windows or `VIRTUAL_ENV/bin/python` elsewhere, when
//!    `VIRTUAL_ENV` is set.
//! 4. `python3` on `PATH`, then `python` on `PATH`.
//!
//! The first candidate that resolves to an existing file wins. When none resolves, the command
//! fails and names every candidate it tried. A candidate is treated as a path when it contains a
//! separator, and otherwise as a command name looked up on `PATH` (with a `.exe` suffix tried on
//! Windows).

use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, bail};

use crate::opt::OptimizeOpt;

/// The environment variable that names the Python interpreter to invoke.
pub(crate) const PYTHON_ENV: &str = "NAUTILUS_PYTHON";

/// The Python module that implements the optimization configuration entry point.
pub(crate) const ENTRY_POINT: &str = "nautilus_trader.optimization.config";

/// Runs the optimization over the configuration file by invoking the Python entry point.
///
/// # Errors
///
/// Returns an error when no Python interpreter resolves, when the interpreter cannot be spawned,
/// or when the entry point exits non-zero (a configuration or run failure, reported through the
/// child's own JSON document).
pub(crate) fn run_optimize(args: &OptimizeOpt) -> anyhow::Result<()> {
    crate::catalog::silence_console_logging();
    let interpreter = resolve_interpreter(args.python.as_deref())?;
    let status = Command::new(&interpreter)
        .arg("-m")
        .arg(ENTRY_POINT)
        .arg(&args.config)
        .status()
        .with_context(|| {
            format!(
                "failed to run Python interpreter '{}'",
                interpreter.display()
            )
        })?;
    if !status.success() {
        bail!("optimization failed (exit status {status})");
    }
    Ok(())
}

/// Resolves the Python interpreter to invoke from the documented candidate order.
///
/// # Errors
///
/// Returns an error naming every candidate when none of them resolves to a file.
pub(crate) fn resolve_interpreter(explicit: Option<&str>) -> anyhow::Result<PathBuf> {
    let candidates = interpreter_candidates(
        explicit,
        env::var(PYTHON_ENV).ok().as_deref(),
        env::var("VIRTUAL_ENV").ok().as_deref(),
    );
    let path_var = env::var_os("PATH");
    for candidate in &candidates {
        if let Some(resolved) = resolve_candidate(candidate, path_var.as_deref()) {
            return Ok(resolved);
        }
    }
    bail!(
        "no Python interpreter found; pass --python, set {PYTHON_ENV}, or put python3 on PATH \
         (tried: {})",
        candidates.join(", ")
    )
}

/// Builds the ordered interpreter candidates, without touching the filesystem.
fn interpreter_candidates(
    explicit: Option<&str>,
    env_override: Option<&str>,
    virtual_env: Option<&str>,
) -> Vec<String> {
    let mut candidates = Vec::new();
    push_candidate(&mut candidates, explicit);
    push_candidate(&mut candidates, env_override);
    if let Some(virtual_env) = virtual_env.filter(|value| !value.is_empty()) {
        push_candidate(&mut candidates, Some(&virtual_env_python(virtual_env)));
    }
    push_candidate(&mut candidates, Some("python3"));
    push_candidate(&mut candidates, Some("python"));
    candidates
}

/// Pushes a non-empty candidate when it is not already present.
fn push_candidate(candidates: &mut Vec<String>, candidate: Option<&str>) {
    if let Some(candidate) = candidate.filter(|value| !value.is_empty())
        && !candidates.iter().any(|existing| existing == candidate)
    {
        candidates.push(candidate.to_string());
    }
}

/// Returns the interpreter path inside a virtual environment root.
fn virtual_env_python(virtual_env: &str) -> String {
    if cfg!(windows) {
        format!("{virtual_env}\\Scripts\\python.exe")
    } else {
        format!("{virtual_env}/bin/python")
    }
}

/// Resolves one candidate to a file, treating a path-like candidate as a path and otherwise
/// looking the name up on `PATH`.
fn resolve_candidate(candidate: &str, path_var: Option<&OsStr>) -> Option<PathBuf> {
    let path = Path::new(candidate);
    if candidate.contains('/') || candidate.contains('\\') {
        return path.is_file().then_some(path.to_path_buf());
    }
    find_in_path(candidate, path_var)
}

/// Looks a command name up on the given `PATH` value.
fn find_in_path(candidate: &str, path_var: Option<&OsStr>) -> Option<PathBuf> {
    let entries = path_var?;
    for directory in env::split_paths(entries) {
        let direct = directory.join(candidate);
        if direct.is_file() {
            return Some(direct);
        }
        if cfg!(windows) {
            let with_extension = directory.join(format!("{candidate}.exe"));
            if with_extension.is_file() {
                return Some(with_extension);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn interpreter_candidates_follow_the_documented_order() {
        let candidates = interpreter_candidates(
            Some("/opt/explicit/python"),
            Some("/opt/environment/python"),
            Some("/opt/venv"),
        );

        assert_eq!(
            candidates[0..3],
            [
                "/opt/explicit/python".to_string(),
                "/opt/environment/python".to_string(),
                virtual_env_python("/opt/venv"),
            ]
        );
        assert_eq!(
            candidates[3..],
            ["python3".to_string(), "python".to_string()]
        );
    }

    #[test]
    fn interpreter_candidates_skip_empty_and_duplicate_values() {
        let candidates = interpreter_candidates(None, Some(""), Some(""));

        assert_eq!(candidates, vec!["python3", "python"]);
    }

    #[test]
    fn resolve_candidate_prefers_the_first_path_entry_that_exists() {
        let root = env::temp_dir().join("nautilus-cli-optimize-test");
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        let script = if cfg!(windows) {
            "python.exe"
        } else {
            "python"
        };
        fs::write(second.join(script), b"").unwrap();
        let path_var = env::join_paths([&first, &second]).unwrap();

        let resolved = find_in_path("python", Some(path_var.as_os_str()));

        assert_eq!(resolved, Some(second.join(script)));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn resolve_candidate_fails_when_no_entry_matches() {
        assert_eq!(
            find_in_path("python", Some(OsStr::new("/nonexistent-nautilus-path"))),
            None
        );
    }

    #[test]
    fn resolve_candidate_treats_a_path_as_a_path() {
        let missing = env::temp_dir().join("nautilus-cli-missing-interpreter");
        let candidate = missing.to_string_lossy().to_string();

        assert_eq!(resolve_candidate(&candidate, None), None);
    }
}
