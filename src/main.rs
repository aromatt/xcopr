use std::process;
use std::process::Child;
use std::process::Stdio;
use std::fmt;
use std::io;
use clap::Parser;

#[derive(Debug)]
pub enum XcoprError {
    SpawnFailed {
        command: String,
        source: io::Error,
    },
    WaitFailed {
        command: String,
        source: io::Error,
    },
    ExitError {
        command: String,
        status: std::process::ExitStatus,
    },
    StdoutNotCaptured(String),
    MissingArgs(&'static str),
}

impl fmt::Display for XcoprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use XcoprError::*;
        match self {
            SpawnFailed { command, source } => {
                write!(f, "failed to start subprocess `{}`: {}", command, source)
            }
            WaitFailed { command, source } => {
                write!(f, "failed to start subprocess `{}`: {}", command, source)
            }
            ExitError { command, status } => {
                write!(f, "subprocess `{}` failed with exit status {}", command, status)
            }
            MissingArgs(arg) => write!(f, "missing required argument: {}", arg),
            StdoutNotCaptured(cmd) => write!(f, "stdout not captured for `{}`", cmd),
        }
    }
}


/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// A command to run in a coprocess
    #[arg(short, long)]
    coproc: Vec<String>,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    stream: u8,
}

fn spawn(cmd_str: &str, stdin: Stdio, stdout: Stdio) -> Result<Child, XcoprError> {
    std::process::Command::new("sh")
        .arg("-eu")
        .arg("-c")
        .arg(cmd_str)
        .stdin(stdin)
        .stdout(stdout)
        .spawn()
        .map_err(|e| XcoprError::SpawnFailed {
            command: cmd_str.to_string(),
            source: e,
        })
}

fn run(args: Args) -> Result<(), XcoprError> {
    let mut children = Vec::new();
    let mut next_stdin: Stdio = Stdio::inherit();

    // Handle all but the last command
    for cmd_str in &args.coproc[..args.coproc.len().saturating_sub(1)] {
        let mut child = spawn(cmd_str, next_stdin, Stdio::piped())?;

        let stdout = child.stdout.take().ok_or_else(|| {
            XcoprError::StdoutNotCaptured(cmd_str.to_string())
        })?;

        next_stdin = Stdio::from(stdout);
        children.push((cmd_str, child));
    }

    // Handle last command, which inherits stdout from xcopr
    if let Some(cmd_str) = args.coproc.last() {
        let child = spawn(cmd_str, next_stdin, Stdio::inherit())?;
        children.push((cmd_str, child));
    }

    // Wait for all procs to exit
    for (cmd_str, mut child) in children {
        let status = child.wait().map_err(|e| XcoprError::WaitFailed {
            command: cmd_str.clone(),
            source: e,
        })?;

        if !status.success() {
            return Err(XcoprError::ExitError {
                command: cmd_str.clone(),
                status,
            })
        }
    }

    Ok(())
}

fn main() {
    let args = Args::parse();
    match run(args) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("xcopr: {}", e);
            process::exit(1);
        }
    }
}
