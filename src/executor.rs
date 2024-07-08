use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct LinuxProcessConfiguration {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) envs: HashMap<String, String>,
    pub(crate) working_dir: Option<PathBuf>,
    pub(crate) redirect_stdout: bool,
    pub(crate) redirect_stdin: bool,
    pub(crate) redirect_stderr: bool,
    pub(crate) disable_extra_reads: bool,
    pub(crate) user_id: Option<u32>,
    pub(crate) group_id: Option<u32>,
    pub(crate) process_group_id: Option<u32>,
}

impl LinuxProcessConfiguration {
    pub fn new(program: impl Into<String>) -> LinuxProcessConfiguration {
        LinuxProcessConfiguration {
            program: program.into(),
            args: Vec::new(),
            envs: HashMap::new(),
            working_dir: None,
            redirect_stdout: false,
            redirect_stdin: false,
            redirect_stderr: false,
            disable_extra_reads: false,
            user_id: None,
            group_id: None,
            process_group_id: None,
        }
    }

    pub fn arg(&mut self, argument: impl Into<String>) -> &mut Self {
        self.args.push(argument.into());
        self
    }

    pub fn args(&mut self, arguments: &mut Vec<String>) -> &mut Self {
        self.args.append(arguments);
        self
    }

    pub fn env(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.envs.insert(key.into(), value.into());
        self
    }

    pub fn envs(&mut self, environment: HashMap<String, String>) -> &mut Self {
        self.envs.extend(environment);
        self
    }

    pub fn clear_env(&mut self) -> &mut Self {
        self.envs.clear();
        self
    }

    pub fn working_dir(&mut self, working_dir: impl Into<PathBuf>) -> &mut Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    pub fn redirect_stdout(&mut self) -> &mut Self {
        self.redirect_stdout = true;
        self
    }

    pub fn redirect_stdin(&mut self) -> &mut Self {
        self.redirect_stdin = true;
        self
    }

    pub fn redirect_stderr(&mut self) -> &mut Self {
        self.redirect_stderr = true;
        self
    }

    pub fn disable_extra_reads(&mut self) -> &mut Self {
        self.disable_extra_reads = true;
        self
    }

    pub fn user_id(&mut self, user_id: u32) -> &mut Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn group_id(&mut self, group_id: u32) -> &mut Self {
        self.group_id = Some(group_id);
        self
    }

    pub fn process_group_id(&mut self, process_group_id: u32) -> &mut Self {
        self.process_group_id = Some(process_group_id);
        self
    }
}

#[derive(Debug)]
pub enum LinuxProcessError {
    UnsupportedOperation,
    ProcessIdNotFound,
    StdinNotPiped,
    StdoutNotPiped,
    StderrNotPiped,
    StreamPipedButNotFound,
    IO(std::io::Error),
    Other(Box<dyn std::error::Error>),
}

#[derive(Debug, Clone)]
pub struct LinuxProcessOutput {
    pub stdout: Option<Vec<u8>>,
    pub stderr: Option<Vec<u8>>,
    pub stdout_extended: HashMap<u32, Vec<u8>>,
    pub status_code: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct LinuxProcessPartialOutput {
    pub stdout: Option<Vec<u8>>,
    pub stderr: Option<Vec<u8>>,
    pub stdout_extended: HashMap<u32, Vec<u8>>,
}

#[async_trait]
pub trait LinuxProcess: Sized {
    fn id(&self) -> Option<u32>;

    async fn write_to_stdin(&mut self, data: &[u8]) -> Result<usize, LinuxProcessError>;

    async fn close_stdin(&mut self) -> Result<(), LinuxProcessError>;

    fn get_partial_output(&self) -> Result<LinuxProcessPartialOutput, LinuxProcessError>;

    async fn await_exit(&mut self) -> Result<Option<i64>, LinuxProcessError>;

    async fn await_exit_with_output(mut self) -> Result<LinuxProcessOutput, LinuxProcessError>;

    async fn begin_kill(&mut self) -> Result<(), LinuxProcessError>;

    async fn kill(&mut self) -> Result<Option<i64>, LinuxProcessError> {
        self.begin_kill().await?;
        self.await_exit().await
    }

    async fn kill_with_output(mut self) -> Result<LinuxProcessOutput, LinuxProcessError> {
        self.begin_kill().await?;
        self.await_exit_with_output().await
    }
}

#[async_trait]
pub trait LinuxExecutor {
    async fn begin_execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<impl LinuxProcess, LinuxProcessError>;

    async fn execute(
        &self,
        process_configuration: &LinuxProcessConfiguration,
    ) -> Result<LinuxProcessOutput, LinuxProcessError>;
}
