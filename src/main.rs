use std::process;
use std::env;
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
    BadShell(String),
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
            BadShell(cmd) => write!(f, "bad shell: `{}`", cmd),
        }
    }
}


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// A command to run in a coprocess
    #[arg(short, long)]
    coproc: Vec<String>,

    /// A stream template
    #[arg(short, long)]
    stream: Vec<String>,
}

#[derive(Debug)]
struct Shell {
    program: String,
    args: Vec<String>,
}

impl Shell {

    fn from_str(cmd_str: &str) -> Result<Shell, XcoprError> {
        let mut iter = cmd_str.split_whitespace();
        let program = iter.next().take().ok_or_else(|| {
            XcoprError::BadShell(cmd_str.to_string())
        })?;
        let args = iter.map(|s| s.to_string()).collect();
        Ok(Shell {
            program: program.to_string(),
            args,
        })
    }

    fn from_env(var_name: &str, default: &str) -> Result<Shell, XcoprError> {
        env::var(var_name)
            .or(Ok(default.to_string()))
            .map(|s| Shell::from_str(&s))?
    }

    fn full_cmd_str(&self, cmd: &str) -> String {
        format!("{} {} '{}'", self.program, self.args.join(" "), cmd)
    }

    fn spawn(&self, cmd_str: &str, stdin: Stdio, stdout: Stdio) -> Result<Child, XcoprError> {
        std::process::Command::new(&self.program)
            .args(&self.args)
            .arg(cmd_str)
            .stdin(stdin)
            .stdout(stdout)
            .spawn()
            .map_err(|e| XcoprError::SpawnFailed {
                command: self.full_cmd_str(cmd_str),
                source: e,
            })
    }
}

fn run(args: Args) -> Result<(), XcoprError> {
    let mut children = Vec::new();
    let mut next_stdin: Stdio = Stdio::inherit();
    let shell = Shell::from_env("XCOPR_SHELL", "sh -euc")?;

    // Handle all but the last command
    for cmd_str in &args.coproc[..args.coproc.len().saturating_sub(1)] {
        let mut child = shell.spawn(cmd_str, next_stdin, Stdio::piped())?;

        let stdout = child.stdout.take().ok_or_else(|| {
            XcoprError::StdoutNotCaptured(cmd_str.to_string())
        })?;

        next_stdin = Stdio::from(stdout);
        children.push((cmd_str, child));
    }

    // Handle last command, which inherits stdout from xcopr
    if let Some(cmd_str) = args.coproc.last() {
        let child = shell.spawn(cmd_str, next_stdin, Stdio::inherit())?;
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
