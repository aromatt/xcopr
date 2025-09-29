use std::process;
use std::env;
use std::process::{Child, Command, Stdio};
use std::fmt;
use std::io::{self, BufRead, Write};
use std::collections::HashMap;
use std::thread;

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
    MissingStreamTemplate, // TODO
    BadShell(String),
    InvalidStreamRef(String),
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
            // TODO
            MissingStreamTemplate => write!(f, "missing stream template"),
            StdoutNotCaptured(cmd) => write!(f, "stdout not captured for `{}`", cmd),
            BadShell(cmd) => write!(f, "bad shell: `{}`", cmd),
            InvalidStreamRef(num_str) => {
                write!(f, "invalid stream reference: `{}`", num_str)
            }
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

/// The shell and initial arguments that will be used to execute the coprocesses.
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

    /// Executes cmd_str using this shell, connecting the provided stdin and stdout.
    fn spawn(&self, cmd_str: &str, stdin: Stdio, stdout: Stdio) -> Result<Child, XcoprError> {
        Command::new(&self.program)
            .args(&self.args)
            .arg(cmd_str)
            .stdin(stdin)
            .stdout(stdout)
            .spawn()
            .map_err(|e| XcoprError::SpawnFailed {
                command: format!("{} {} '{}'", self.program, self.args.join(" "), cmd_str),
                source: e,
            })
    }
}

enum Segment {
    Literal(String),
    StreamRef(usize),
}

// TODO: support embedded coprocesses
fn parse_template(tmpl: &str) -> Result<Vec<Segment>, XcoprError> {
    let mut segments = Vec::new();
    let mut chars = tmpl.chars().peekable();
    let mut cur_lit = String::new();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            // Flush any literal accumulated so far
            if !cur_lit.is_empty() {
                segments.push(Segment::Literal(std::mem::take(&mut cur_lit)));
            }
            // Parse the number after %. Peek and collect each char until we reach a non-digit
            let mut num_str = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    num_str.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            let idx = num_str.parse::<usize>().map_err(|_| {
                XcoprError::InvalidStreamRef(num_str)
            })?;
            segments.push(Segment::StreamRef(idx));
        } else {
            cur_lit.push(ch);
        }
    }

    if !cur_lit.is_empty() {
        segments.push(Segment::Literal(cur_lit));
    }

    Ok(segments)
}

/// Generates a sequence of strings that can reference (and be referenced by) other streams.
struct StreamTemplate {
    template: String,
    segments: Vec<Segment>,
}

impl StreamTemplate {
    fn parse(template: &str) -> Result<StreamTemplate, XcoprError> {
        let segments = parse_template(template)
            .map_err(|e| {
                eprintln!("xcopr: error parsing stream template `{}`: {}", template, e);
                e
            })?;
        Ok(StreamTemplate {
            template: template.to_string(),
            segments,
        })
    }

    /// Renders the template for a line given a cross-section of values from referenced streams.
    // TODO: I don't think we need to return a string here. We should just write
    //       all the segments directly to this template's output stream or stdout.
    //
    fn render(&self, streams: &[&str]) -> String {
        let mut out = String::with_capacity(64);
        for seg in &self.segments {
            match seg {
                Segment::Literal(s) => out.push_str(&s),
                Segment::StreamRef(i) => out.push_str(streams[*i]),
            }
        }
        out
    }

}

fn run(args: Args) -> Result<(), XcoprError> {
    let mut children = Vec::new();
    let mut next_stdin: Stdio = Stdio::inherit();
    let shell = Shell::from_env("XCOPR_SHELL", "sh -euc")?;

    // TODO: for now, assume there's exactly one stream template and it's the final stream
    if args.stream.len() != 1 {
        return Err(XcoprError::MissingStreamTemplate)
    }
    let stream_template = StreamTemplate::parse(&args.stream[0]);

    // This holds references to the output streams of the coprocesses. These are the streams that
    // can be referenced by index (e.g. %1, %2, etc) in the stream template. %0 is the original
    // stdin.
    let mut cmd_streams = Vec::new();

    // Set up all coprocesses
    for cmd_str in &args.coproc {
        let mut child = shell.spawn(cmd_str, next_stdin, Stdio::piped())?;

        let stdout = child.stdout.take().ok_or_else(|| {
            XcoprError::StdoutNotCaptured(cmd_str.to_string())
        })?;

        next_stdin = Stdio::from(stdout);
        children.push((cmd_str, child));
    }

    //// Set up all but the last command.
    //for cmd_str in &args.coproc[..args.coproc.len().saturating_sub(1)] {
    //    let mut child = shell.spawn(cmd_str, next_stdin, Stdio::piped())?;

    //    let stdout = child.stdout.take().ok_or_else(|| {
    //        XcoprError::StdoutNotCaptured(cmd_str.to_string())
    //    })?;

    //    next_stdin = Stdio::from(stdout);
    //    children.push((cmd_str, child));
    //}

    //// Set up last command, which inherits stdout from xcopr
    //if let Some(cmd_str) = args.coproc.last() {
    //    let child = shell.spawn(cmd_str, next_stdin, Stdio::inherit())?;
    //    children.push((cmd_str, child));
    //}

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
