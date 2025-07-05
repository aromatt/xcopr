use std::process;
use std::process::Stdio;
use std::fmt;
use std::io;
use clap::Parser;

#[derive(Debug)]
pub enum XcoprError {
    SubprocSpawnFailed {
        command: String,
        source: io::Error,
    },
    SubprocWaitFailed {
        command: String,
        source: io::Error,
    },

    SubprocExitError {
        command: String,
        status: std::process::ExitStatus,
    },
    SubprocStdoutNotCaptured(String),
    MissingArgs(&'static str),
}

impl fmt::Display for XcoprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use XcoprError::*;
        match self {
            SubprocSpawnFailed { command, source } => {
                write!(f, "failed to start subprocess `{}`: {}", command, source)
            }
            SubprocWaitFailed { command, source } => {
                write!(f, "failed to start subprocess `{}`: {}", command, source)
            }
            SubprocExitError { command, status } => {
                write!(f, "subprocess `{}` failed with exit status {}", command, status)
            }
            MissingArgs(arg) => write!(f, "missing required argument: {}", arg),
            SubprocStdoutNotCaptured(cmd) => write!(f, "stdout not captured for `{}`", cmd),
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

fn run(args: Args) -> Result<(), XcoprError> {
    let mut children = Vec::new();
    let mut next_stdin: Stdio = Stdio::inherit();

    for cmd_str in &args.coproc[..args.coproc.len().saturating_sub(1)] {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-eu").arg("-c").arg(cmd_str);
        cmd.stdin(next_stdin);

        cmd.stdout(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e|
            XcoprError::SubprocSpawnFailed {
                command: cmd_str.clone(),
                source: e,
            }
        )?;
        next_stdin = match child.stdout.take() {
            Some(stdout) => Stdio::from(stdout),
            None => {
                return Err(XcoprError::SubprocStdoutNotCaptured(cmd_str.clone()));
            }
        };
        children.push((cmd_str, child));
    }

    // Handle last command, which inherits stdout from xcopr
    let cmd_str = &args.coproc[args.coproc.len()-1];
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-eu").arg("-c").arg(cmd_str);
    cmd.stdin(next_stdin);
    cmd.stdout(Stdio::inherit());
    let child = cmd.spawn().map_err(|e|
        XcoprError::SubprocSpawnFailed {
            command: cmd_str.clone(),
            source: e,
        }
    )?;
    children.push((cmd_str, child));

    // Wait for all children to exit
    for (cmd_str, mut child) in children {
        let status = child.wait().map_err(|e| XcoprError::SubprocWaitFailed {
            command: cmd_str.clone(),
            source: e,
        })?;
        if !status.success() {
            return Err(XcoprError::SubprocExitError {
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
