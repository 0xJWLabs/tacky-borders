use std::{
    ffi::{OsStr, OsString},
    io,
    process::{Command, Stdio},
};

use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Open path with the default application without blocking.
pub fn that(path: impl AsRef<OsStr>) -> io::Result<()> {
    let mut last_err = None;
    for mut cmd in commands(path) {
        match cmd.status_without_output() {
            Ok(status) => {
                return Ok(status).into_result(&cmd);
            }
            Err(err) => last_err = Some(err),
        }
    }
    Err(last_err.expect("no launcher worked, at least one error"))
}

pub fn commands<T: AsRef<OsStr>>(path: T) -> Vec<Command> {
    let mut cmd = Command::new("cmd");
    cmd.arg("/c")
        .arg("start")
        .raw_arg("\"\"")
        .raw_arg(wrap_in_quotes(path))
        .creation_flags(CREATE_NO_WINDOW);
    vec![cmd]
}

fn wrap_in_quotes<T: AsRef<OsStr>>(path: T) -> OsString {
    let mut result = OsString::from("\"");
    result.push(path);
    result.push("\"");

    result
}

trait IntoResult<T> {
    fn into_result(self, cmd: &Command) -> T;
}

impl IntoResult<io::Result<()>> for io::Result<std::process::ExitStatus> {
    fn into_result(self, cmd: &Command) -> io::Result<()> {
        match self {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Launcher {cmd:?} failed with {:?}", status),
            )),
            Err(err) => Err(err),
        }
    }
}

trait CommandExts {
    fn status_without_output(&mut self) -> io::Result<std::process::ExitStatus>;
}

impl CommandExts for Command {
    fn status_without_output(&mut self) -> io::Result<std::process::ExitStatus> {
        self.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    }
}
