use std::ffi::{OsStr, OsString};
use std::io;
use std::process::ExitStatus;

use crate::runas::{runas_spawn, ChildInner};

pub struct Child {
    inner: ChildInner,
}

impl Child {
    pub(crate) fn new(inner: ChildInner) -> Child {
        Child { inner }
    }

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

pub struct Command {
    pub(crate) command: OsString,
    pub(crate) args: Vec<OsString>,
    pub(crate) hide: bool,
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Command {
        Command {
            command: program.as_ref().to_os_string(),
            args: vec![],
            hide: false,
        }
    }

    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<S: AsRef<OsStr>>(&mut self, args: &[S]) -> &mut Command {
        for arg in args {
            self.arg(arg);
        }
        self
    }

    pub fn hide(&mut self, val: bool) -> &mut Command {
        self.hide = val;
        self
    }

    pub fn spawn(&mut self) -> io::Result<Child> {
        runas_spawn(self)
    }

    pub fn run(&mut self) -> io::Result<ExitStatus> {
        self.spawn()?.wait()
    }
}
