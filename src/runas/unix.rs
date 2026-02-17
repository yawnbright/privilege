use std::io;
use std::process::{Child as StdChild, ExitStatus};

use crate::runas::Command;

pub(crate) const ENV_PATH: &str = "PATH";
const CMD_PKEXEC: &str = "pkexec";
const CMD_SUDO: &str = "sudo";

pub struct ChildInner {
    inner: StdChild,
    use_pkexec: bool,
}

impl ChildInner {
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        self.inner.wait()
    }

    pub fn kill(&mut self) -> io::Result<()> {
        self.inner.kill()
    }

    pub fn id(&self) -> u32 {
        self.inner.id()
    }
}

fn spawn_pkexec(cmd: &Command) -> io::Result<StdChild> {
    let mut c = std::process::Command::new(CMD_PKEXEC);
    c.arg("--").arg(&cmd.command).args(&cmd.args[..]);
    if cmd.hide {
        c.arg("--silent");
    }
    c.spawn()
}

fn spawn_sudo(cmd: &Command) -> io::Result<StdChild> {
    let mut c = std::process::Command::new(CMD_SUDO);
    c.arg("--").arg(&cmd.command).args(&cmd.args[..]);
    c.spawn()
}

pub fn runas_spawn(cmd: &Command) -> io::Result<crate::runas::Child> {
    if which::which(CMD_PKEXEC).is_ok() {
        spawn_pkexec(cmd).map(|inner| {
            crate::runas::Child::new(ChildInner {
                inner,
                use_pkexec: true,
            })
        })
    } else if which::which(CMD_SUDO).is_ok() {
        spawn_sudo(cmd).map(|inner| {
            crate::runas::Child::new(ChildInner {
                inner,
                use_pkexec: false,
            })
        })
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Neither `{}` nor `{}` found", CMD_PKEXEC, CMD_SUDO),
        ))
    }
}
