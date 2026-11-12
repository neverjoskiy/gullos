use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::builtins::{run_builtin, BuiltinOutcome, ShellState};
use crate::history::History;
use crate::lexer::{
    parse_command_line, ChainConnector, Pipeline, RedirKind, Redirection, SimpleCommand,
};

pub enum ExecControl {
    Continue(i32),
    Exit(i32),
}

pub fn execute_line(
    line: &str,
    state: &mut ShellState,
    history: &History,
) -> ExecControl {
    let expanded_line = expand_leading_aliases(line, state);
    let chains = match parse_command_line(&expanded_line, state.last_status) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("\x1b[38;2;243;139;168mgull-sh:\x1b[0m {}", err);
            state.last_status = 2;
            return ExecControl::Continue(2);
        }
    };

    let mut status = state.last_status;
    for chain in chains {
        let should_run = match chain.connector {
            ChainConnector::Always => true,
            ChainConnector::And => status == 0,
            ChainConnector::Or => status != 0,
        };
        if !should_run {
            continue;
        }

        match execute_pipeline(&chain.pipeline, state, history) {
            ExecControl::Continue(code) => {
                status = code;
                state.last_status = code;
            }
            ExecControl::Exit(code) => return ExecControl::Exit(code),
        }
    }

    ExecControl::Continue(status)
}

fn expand_leading_aliases(line: &str, state: &ShellState) -> String {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return line.to_string();
    }
    let first_end = trimmed
        .find(|c: char| c.is_whitespace() || matches!(c, '|' | ';' | '&'))
        .unwrap_or(trimmed.len());
    let first_word = &trimmed[..first_end];
    if let Some(replacement) = state.aliases.get(first_word) {
        format!("{}{}", replacement, &trimmed[first_end..])
    } else {
        line.to_string()
    }
}

fn execute_pipeline(
    pipeline: &Pipeline,
    state: &mut ShellState,
    history: &History,
) -> ExecControl {
    if pipeline.commands.is_empty() {
        return ExecControl::Continue(0);
    }

    // Single command with no pipe: run shell-mutating assignments & builtins in-process
    if pipeline.commands.len() == 1 && !pipeline.background {
        let cmd = &pipeline.commands[0];
        if cmd.argv.is_empty() {
            for (k, v) in &cmd.assignments {
                env::set_var(k, v);
            }
            return ExecControl::Continue(0);
        }

        if cmd.redirections.is_empty() {
            if let Some(outcome) = run_builtin(&cmd.argv, state, history) {
                return match outcome {
                    BuiltinOutcome::Executed(code) => ExecControl::Continue(code),
                    BuiltinOutcome::ExitShell(code) => ExecControl::Exit(code),
                    BuiltinOutcome::SourceFile(path) => execute_script_file(&path, state, history),
                };
            }
        }
    }

    // Multi-stage OS pipeline or external command execution
    let num_cmds = pipeline.commands.len();
    let mut prev_stdout: Option<Stdio> = None;
    let mut children: Vec<Child> = Vec::new();
    let mut last_inline_status: Option<i32> = None;

    for (idx, simple_cmd) in pipeline.commands.iter().enumerate() {
        let is_last = idx + 1 == num_cmds;
        if simple_cmd.argv.is_empty() {
            continue;
        }

        let program = &simple_cmd.argv[0];
        let args = &simple_cmd.argv[1..];

        let mut command = Command::new(program);
        command.args(args);
        for (k, v) in &simple_cmd.assignments {
            command.env(k, v);
        }

        #[cfg(unix)]
        unsafe {
            use std::os::unix::process::CommandExt;
            command.pre_exec(|| {
                libc::signal(libc::SIGINT, libc::SIG_DFL);
                libc::signal(libc::SIGQUIT, libc::SIG_DFL);
                Ok(())
            });
        }

        // Configure stdin
        if let Some(stdin_pipe) = prev_stdout.take() {
            command.stdin(stdin_pipe);
        }

        // Configure stdout for pipeline stage
        if !is_last {
            command.stdout(Stdio::piped());
        }

        // Apply file redirections (<, >, >>)
        if let Err(err) = apply_redirections(&mut command, &simple_cmd.redirections) {
            eprintln!("\x1b[38;2;243;139;168mgull-sh:\x1b[0m {}", err);
            return ExecControl::Continue(1);
        }

        match command.spawn() {
            Ok(mut child) => {
                if !is_last {
                    prev_stdout = child.stdout.take().map(Stdio::from);
                }
                children.push(child);
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    eprintln!(
                        "\x1b[38;2;243;139;168mgull-sh:\x1b[0m command not found: \x1b[1m{}\x1b[0m",
                        program
                    );
                    last_inline_status = Some(127);
                } else {
                    eprintln!(
                        "\x1b[38;2;243;139;168mgull-sh:\x1b[0m {}: {}",
                        program, err
                    );
                    last_inline_status = Some(126);
                }
            }
        }
    }

    if pipeline.background {
        if let Some(last) = children.last() {
            println!("[bg] pid {}", last.id());
        }
        return ExecControl::Continue(0);
    }

    let mut final_code = last_inline_status.unwrap_or(0);
    for mut child in children {
        match child.wait() {
            Ok(exit_status) => {
                if last_inline_status.is_none() {
                    final_code = exit_status.code().unwrap_or(130);
                }
            }
            Err(_) => {
                final_code = 1;
            }
        }
    }

    ExecControl::Continue(final_code)
}

fn apply_redirections(command: &mut Command, redirs: &[Redirection]) -> Result<(), String> {
    for redir in redirs {
        match redir.kind {
            RedirKind::Input => {
                let file = File::open(&redir.target)
                    .map_err(|e| format!("{}: {}", redir.target, e))?;
                command.stdin(Stdio::from(file));
            }
            RedirKind::OutputTruncate => {
                let file = File::create(&redir.target)
                    .map_err(|e| format!("{}: {}", redir.target, e))?;
                command.stdout(Stdio::from(file));
            }
            RedirKind::OutputAppend => {
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&redir.target)
                    .map_err(|e| format!("{}: {}", redir.target, e))?;
                command.stdout(Stdio::from(file));
            }
        }
    }
    Ok(())
}

pub fn execute_script_file(
    path: &Path,
    state: &mut ShellState,
    history: &History,
) -> ExecControl {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(err) => {
            eprintln!("gull-sh: {}: {}", path.display(), err);
            return ExecControl::Continue(1);
        }
    };

    let reader = BufReader::new(file);
    let mut status = 0;
    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        match execute_line(trimmed, state, history) {
            ExecControl::Continue(code) => status = code,
            ExecControl::Exit(code) => return ExecControl::Exit(code),
        }
    }
    ExecControl::Continue(status)
}

#[allow(dead_code)]
fn _assert_send(_c: &SimpleCommand) {}
