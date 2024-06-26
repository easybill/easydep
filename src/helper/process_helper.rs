use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context};
use tokio::task::JoinSet;

use crate::helper::process_helper::StreamEntry::{Stderr, Stdout};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StreamEntry {
    Stdout(String),
    Stderr(String),
    Exit,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandResult {
    pub status: ExitStatus,
    pub command_line: String,
    pub stream_output: Vec<StreamEntry>,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandResultCollection {
    pub failed_command: bool,
    pub results: Vec<CommandResult>,
}

pub(crate) fn pretty_print_output(output: &CommandResult) -> Vec<String> {
    // pretty print the command line
    let mut target = Vec::<String>::new();
    let exit_code = output.status.code().unwrap_or(-1);
    target.push(format!(
        "----- {} (status: {}) -----",
        &output.command_line, exit_code
    ));

    // push all output lines
    let output_entries = &output.stream_output;
    for entry in output_entries {
        match entry {
            Stdout(line) => {
                let formatted_line = format!("[stdout]: {}", line);
                target.push(formatted_line);
            }
            Stderr(line) => {
                let formatted_line = format!("[stderr]: {}", line);
                target.push(formatted_line);
            }
            _ => {}
        }
    }

    // finish off with a last delimiter line
    target.push("-----".to_string());
    target
}

pub(crate) async fn run_command(
    mut command: Command,
) -> anyhow::Result<CommandResult, anyhow::Error> {
    // ensure that the process pipes all outputs to this process
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    // spawn and run the process
    let full_command_line = format!("{:?}", &command);
    let process = command.spawn()?;
    wait_for_process(process, full_command_line).await
}

async fn wait_for_process(
    mut process: Child,
    command_line: String,
) -> anyhow::Result<CommandResult, anyhow::Error> {
    let (sender, receiver) = channel();
    let target = Arc::new(Mutex::new(Vec::<StreamEntry>::new()));

    let mut join_set = JoinSet::new();

    // read the full stdout
    let stdout_sender = sender.clone();
    let stdout = process
        .stdout
        .take()
        .context("Unable to get process stdout")?;
    read_stream_output(stdout, stdout_sender, &mut join_set, Stdout);

    // read the full stderr
    let stderr_sender = sender.clone();
    let stderr = process
        .stderr
        .take()
        .context("Unable to get process stderr")?;
    read_stream_output(stderr, stderr_sender, &mut join_set, Stderr);

    // spawn the thread that receives the lines
    let entry_target = Arc::clone(&target);
    join_set.spawn(async move {
        while let Ok(entry) = receiver.recv() {
            // exit the loop when receiving the exit signal
            if entry == StreamEntry::Exit {
                break;
            }

            if let Ok(mut guard) = entry_target.lock() {
                guard.push(entry);
            }
        }
    });

    // await the process exit and notify the receiver
    let exit_sender = sender.clone();
    let process_exit_code = process.wait()?;
    exit_sender
        .send(StreamEntry::Exit)
        .context("Unable to notify about process exit")?;

    // wait for all futures to complete
    while join_set.join_next().await.is_some() {}

    // unwrap the log lines & return the final result
    return match target.lock() {
        Ok(guard) => {
            let output = guard.clone();
            Ok(CommandResult {
                command_line,
                status: process_exit_code,
                stream_output: output,
            })
        }
        Err(_) => Err(anyhow!("Unable to acquire lock on stream output")),
    };
}

fn read_stream_output<R: Read + Send + 'static, F: Fn(String) -> StreamEntry + Send + 'static>(
    stream: R,
    target: Sender<StreamEntry>,
    tracker: &mut JoinSet<()>,
    line_factory: F,
) {
    tracker.spawn(async move {
        let stream_reader = BufReader::new(stream);
        for line in stream_reader.lines().map_while(Result::ok) {
            if target.send(line_factory(line)).is_err() {
                break;
            };
        }
    });
}
