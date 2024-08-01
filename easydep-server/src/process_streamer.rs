/*
 * This file is part of easydep, licensed under the MIT License (MIT).
 *
 * Copyright (c) 2024 easybill GmbH
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use std::io::Error;

use anyhow::{anyhow, Context};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::LinesStream;
use tokio_stream::StreamExt;
use tonic::Status;

use crate::easydep::{Action, ActionStatus, ExecutedActionEntry, LogEntry, LogType};

/// A streamer that streams `ExecutedActionEntry`s to a gRPC client from a spawned child process.
pub(crate) struct ProcessStreamer {
    action: Action,
    release_id: u64,
    child_process: Child,
    sender: Sender<Result<ExecutedActionEntry, Status>>,
}

impl ProcessStreamer {
    /// Creates a new process streamer instance for the given child process.
    ///
    /// # Arguments
    /// * `action` - The action that is represented by the given process.
    /// * `release_id` - The id of the release that is being executed.
    /// * `child_process` - The process to stream the log output of.
    /// * `sender` - The sender into which the constructed action entries will be sent.
    pub(crate) fn new(
        action: Action,
        release_id: u64,
        child_process: Child,
        sender: Sender<Result<ExecutedActionEntry, Status>>,
    ) -> Self {
        ProcessStreamer {
            action,
            release_id,
            child_process,
            sender,
        }
    }

    /// Waits for the underlying child process to complete and streams the log output of it into the underlying sender.
    /// This method returns an error if some error occurs or the underlying process does not finish successfully.
    pub(crate) async fn await_child_and_stream(&mut self) -> anyhow::Result<()> {
        self.sender
            .send(Self::construct_executed_action_entry(
                self.release_id,
                self.action,
                ActionStatus::Started,
                None,
            ))
            .await?;

        let stdout = self
            .child_process
            .stdout
            .take()
            .context("Child process has no stdout available")?;
        let stderr = self
            .child_process
            .stderr
            .take()
            .context("Child process has no stderr available")?;

        let stdout_stream = LinesStream::new(BufReader::new(stdout).lines())
            .map(|entry| Self::construct_log_entry(entry, LogType::Stdout));
        let stderr_stream = LinesStream::new(BufReader::new(stderr).lines())
            .map(|entry| Self::construct_log_entry(entry, LogType::Stderr));

        let action = self.action;
        let release_id = self.release_id;
        let mut combined_stream = stdout_stream.merge(stderr_stream).map(move |log_entry| {
            Self::construct_executed_action_entry(
                release_id,
                action,
                ActionStatus::Running,
                Some(log_entry),
            )
        });

        let sender = self.sender.clone();
        let stream_task = tokio::spawn(async move {
            while let Some(entry) = combined_stream.next().await {
                if sender.send(entry).await.is_err() {
                    return;
                }
            }
        });

        let (_, process_result) = tokio::join!(stream_task, self.child_process.wait());
        match process_result {
            Ok(exit_status) => {
                let log_entry = Self::construct_log_entry(
                    Ok(format!("Process finished with {}", exit_status)),
                    LogType::Stdout,
                );
                let action_status = if exit_status.success() {
                    ActionStatus::CompletedSuccess
                } else {
                    ActionStatus::CompletedFailure
                };
                let action_entry = Self::construct_executed_action_entry(
                    self.release_id,
                    self.action,
                    action_status,
                    Some(log_entry),
                );
                self.sender.send(action_entry).await?;

                if exit_status.success() {
                    Ok(())
                } else {
                    Err(anyhow!(
                        "process did not complete with an successful exit status"
                    ))
                }
            }
            Err(error) => {
                let action_entry = Self::construct_executed_action_entry(
                    self.release_id,
                    self.action,
                    ActionStatus::CompletedFailure,
                    Some(Err(anyhow!(
                        "Error awaiting process for current action: {}",
                        error
                    ))),
                );
                self.sender.send(action_entry).await?;
                Err(error.into())
            }
        }
    }

    /// Constructs a new log entry from the given captured log line, returning
    /// back the error if the log line was not captured successfully.
    ///
    /// # Arguments
    /// * `captured_log_line` - The log line that was potentially captured, could also be an error.
    /// * `stream_type` - The log stream type from which the log line was captured.
    fn construct_log_entry(
        captured_log_line: Result<String, Error>,
        stream_type: LogType,
    ) -> anyhow::Result<LogEntry> {
        captured_log_line
            .map(|line| LogEntry {
                stream_type: stream_type as i32,
                content: line,
            })
            .map_err(Into::into)
    }

    /// Constructs a new executed action entry based on the given properties.
    ///
    /// # Arguments
    /// * `release_id` - The id of the release being executed.
    /// * `current_action` - The action that is currently being executed.
    /// * `status` - The status of the action being executed.
    /// * `log_entry` - The log entry that was captured, can be None if no log line is associated.
    fn construct_executed_action_entry(
        release_id: u64,
        current_action: Action,
        status: ActionStatus,
        log_entry: Option<anyhow::Result<LogEntry>>,
    ) -> Result<ExecutedActionEntry, Status> {
        match log_entry {
            None => {
                let action_entry = ExecutedActionEntry {
                    release_id,
                    current_action: current_action.into(),
                    action_status: status.into(),
                    action_log_entry: None,
                };
                Ok(action_entry)
            }
            Some(entry) => entry
                .map(|log_entry| ExecutedActionEntry {
                    release_id,
                    current_action: current_action.into(),
                    action_status: status.into(),
                    action_log_entry: Some(log_entry),
                })
                .map_err(|err| Status::internal(format!("{:?}", err))),
        }
    }
}
