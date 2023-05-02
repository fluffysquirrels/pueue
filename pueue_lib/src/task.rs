use std::{collections::HashMap, path::PathBuf};

use chrono::prelude::*;
use serde_derive::{Deserialize, Serialize};
use strum_macros::Display;

use crate::state::PUEUE_DEFAULT_GROUP;

/// This enum represents the status of the internal task handling of Pueue.
/// They basically represent the internal task life-cycle.
#[derive(PartialEq, Eq, Clone, Debug, Display, Serialize, Deserialize)]
pub enum TaskStatus {
    /// The task is queued and waiting for a free slot
    Queued,
    /// The task has been manually stashed. It won't be executed until it's manually enqueued
    Stashed { enqueue_at: Option<DateTime<Local>> },
    /// The task is started and running
    Running,
    /// A previously running task has been paused
    Paused,
    /// Task finished. The actual result of the task is handled by the [TaskResult] enum.
    Done(TaskResult),
    /// Used while the command of a task is edited (to prevent starting the task)
    Locked,
}

/// This enum represents the exit status of an actually spawned program.
/// It's only used, once a task finished or failed in some kind of way.
#[derive(PartialEq, Eq, Clone, Debug, Display, Serialize, Deserialize)]
pub enum TaskResult {
    /// Task exited with 0
    Success,
    /// The task failed in some other kind of way (error code != 0)
    Failed(i32),
    /// The task couldn't be spawned. Probably a typo in the command
    FailedToSpawn(String),
    /// Task has been actively killed by either the user or the daemon on shutdown
    Killed,
    /// Some kind of IO error. This should barely ever happen. Please check the daemon logs.
    Errored,
    /// A dependency of the task failed.
    DependencyFailed,
}

/// Representation of a task.
/// start will be set the second the task starts processing.
/// `result`, `output` and `end` won't be initialized, until the task has finished.
#[derive(PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct Task {
    pub id: usize,
    #[serde(default = "Local::now")]
    pub created_at: DateTime<Local>,
    #[serde(default = "Default::default")]
    pub enqueued_at: Option<DateTime<Local>>,
    pub original_command: String,
    pub command: String,
    pub path: PathBuf,
    pub envs: HashMap<String, String>,
    pub group: String,
    pub dependencies: Vec<usize>,
    #[serde(default = "Default::default")]
    pub priority: Option<usize>,
    pub label: Option<String>,
    pub status: TaskStatus,
    /// This field is only used when editing the path/command of a task.
    /// It's necessary, since we enter the `Locked` state during editing.
    /// However, we have to go back to the previous state after we finished editing.
    ///
    /// TODO: Refactor this into a `TaskStatus::Locked{previous_status: TaskStatus}`.
    pub prev_status: TaskStatus,
    pub start: Option<DateTime<Local>>,
    pub end: Option<DateTime<Local>>,
}

impl Task {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        original_command: String,
        path: PathBuf,
        envs: HashMap<String, String>,
        group: String,
        starting_status: TaskStatus,
        dependencies: Vec<usize>,
        priority: Option<usize>,
        label: Option<String>,
    ) -> Task {
        Task {
            id: 0,
            created_at: Local::now(),
            enqueued_at: None,
            original_command: original_command.clone(),
            command: original_command,
            path,
            envs,
            group,
            dependencies,
            priority,
            label,
            status: starting_status.clone(),
            prev_status: starting_status,
            start: None,
            end: None,
        }
    }

    /// A convenience function used to duplicate a task.
    pub fn from_task(task: &Task) -> Task {
        Task {
            id: 0,
            created_at: Local::now(),
            enqueued_at: None,
            original_command: task.original_command.clone(),
            command: task.command.clone(),
            path: task.path.clone(),
            envs: task.envs.clone(),
            group: task.group.clone(),
            dependencies: Vec::new(),
            priority: None,
            label: task.label.clone(),
            status: TaskStatus::Queued,
            prev_status: TaskStatus::Queued,
            start: None,
            end: None,
        }
    }

    /// Whether the task is having a running process managed by the TaskHandler
    pub fn is_running(&self) -> bool {
        matches!(self.status, TaskStatus::Running | TaskStatus::Paused)
    }

    /// Whether the task's process finished.
    pub fn is_done(&self) -> bool {
        matches!(self.status, TaskStatus::Done(_))
    }

    /// Check if the task errored. \
    /// It either:
    /// 1. Finished successfully
    /// 2. Didn't finish yet.
    pub fn failed(&self) -> bool {
        match &self.status {
            TaskStatus::Done(result) => !matches!(result, TaskResult::Success),
            _ => false,
        }
    }

    pub fn is_queued(&self) -> bool {
        matches!(self.status, TaskStatus::Queued | TaskStatus::Stashed { .. })
    }

    /// Small convenience function to set the task's group to the default group.
    pub fn set_default_group(&mut self) {
        self.group = String::from(PUEUE_DEFAULT_GROUP);
    }

    pub fn is_in_default_group(&self) -> bool {
        self.group.eq(PUEUE_DEFAULT_GROUP)
    }
}

/// We use a custom `Debug` implementation for [Task], as the `envs` field just has too much
/// info in it and makes the log output much too verbose.
///
/// Furthermore, there might be secrets in the environment, resulting in a possible leak if
/// users copy-paste their log output for debugging.
impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("original_command", &self.original_command)
            .field("command", &self.command)
            .field("path", &self.path)
            .field("envs", &"hidden")
            .field("group", &self.group)
            .field("dependencies", &self.dependencies)
            .field("label", &self.label)
            .field("status", &self.status)
            .field("prev_status", &self.prev_status)
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}
