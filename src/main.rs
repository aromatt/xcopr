use std::process;
use std::process::Stdio;
use std::fmt;
use std::io;
use clap::Parser;

#[derive(Debug)]
pub enum XcoprError {
    SubprocessSpawnFailed {
        command: String,
        source: io::Error,
    },
    SubprocessWaitFailed {
        command: String,
        source: io::Error,
    },

    SubprocessExitError {
        command: String,
        status: std::process::ExitStatus,
    },
    SubprocessStdoutNotCaptured(String),
    MissingArgs(&'static str),
}

impl fmt::Display for XcoprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use XcoprError::*;
        match self {
            SubprocessSpawnFailed { command, source } => {
                write!(f, "failed to start subprocess `{}`: {}", command, source)
            }
            SubprocessWaitFailed { command, source } => {
                write!(f, "failed to start subprocess `{}`: {}", command, source)
            }
            SubprocessExitError { command, status } => {
                write!(f, "subprocess `{}` failed with exit status {}", command, status)
            }
            MissingArgs(arg) => write!(f, "missing required argument: {}", arg),
            SubprocessStdoutNotCaptured(cmd) => write!(f, "stdout not captured for `{}`", cmd),
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

    for (i, cmd_str) in args.coproc.iter().enumerate() {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-eu").arg("-c").arg(cmd_str);

        cmd.stdin(next_stdin);

        let is_last = i == args.coproc.len() - 1;

        if is_last {
            cmd.stdout(Stdio::inherit());
        } else {
            cmd.stdout(Stdio::piped());
        }

        let mut child = cmd.spawn().map_err(|e|
            XcoprError::SubprocessSpawnFailed {
                command: cmd_str.clone(),
                source: e,
            }
        )?;

        // For all but the last child, stdout must be
        next_stdin = if is_last {
            Stdio::null()
        } else {
            match child.stdout.take() {
                Some(stdout) => Stdio::from(stdout),
                None => {
                    return Err(XcoprError::SubprocessStdoutNotCaptured(cmd_str.clone()));
                }
            }
        };

        children.push((cmd_str, child));
    }

    for (cmd_str, mut child) in children {
        let status = child.wait().map_err(|e| XcoprError::SubprocessWaitFailed {
            command: cmd_str.clone(),
            source: e,
        })?;
        if !status.success() {
            return Err(XcoprError::SubprocessExitError {
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
