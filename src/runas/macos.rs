use std::env;
use std::ffi::{CString, OsString};
use std::io;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::ptr;

use libc::{fcntl, fileno, kill, EINTR, F_GETOWN, SIGKILL};
use security_framework_sys::authorization::{
    errAuthorizationSuccess, kAuthorizationFlagDefaults, kAuthorizationFlagDestroyRights,
    AuthorizationCreate, AuthorizationExecuteWithPrivileges, AuthorizationFree, AuthorizationRef,
};

use crate::runas::Command;

pub(crate) const ENV_PATH: &str = "PATH";

fn get_exe_path<P: AsRef<Path>>(exe_name: P) -> Option<PathBuf> {
    let exe_name = exe_name.as_ref();
    if exe_name.has_root() {
        return Some(exe_name.into());
    }

    env::var_os(ENV_PATH).and_then(|paths| {
        env::split_paths(&paths)
            .filter_map(|dir| {
                let full_path = dir.join(exe_name);
                if full_path.is_file() {
                    Some(full_path)
                } else {
                    None
                }
            })
            .next()
    })
}

macro_rules! make_cstring {
    ($s:expr) => {
        match CString::new($s.as_bytes()) {
            Ok(s) => s,
            Err(_) => {
                return Err(io::Error::new(io::ErrorKind::Other, "null byte in string"));
            }
        }
    };
}

pub struct ChildInner {
    pid: i32,
    auth_ref: AuthorizationRef,
}

impl ChildInner {
    pub fn wait(&self) -> io::Result<ExitStatus> {
        let mut status = 0;
        loop {
            let r = unsafe { libc::waitpid(self.pid, &mut status, 0) };
            if r == -1 && io::Error::last_os_error().raw_os_error() == Some(EINTR) {
                continue;
            } else {
                break;
            }
        }
        unsafe {
            AuthorizationFree(self.auth_ref, kAuthorizationFlagDestroyRights);
        }
        Ok(unsafe { mem::transmute::<i32, ExitStatus>(status) })
    }

    pub fn kill(&self) -> io::Result<()> {
        let result = unsafe { kill(self.pid, SIGKILL) };
        if result == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn id(&self) -> u32 {
        self.pid as u32
    }
}

impl Drop for ChildInner {
    fn drop(&mut self) {
        unsafe {
            AuthorizationFree(self.auth_ref, kAuthorizationFlagDestroyRights);
        }
    }
}

unsafe fn gui_spawn(prog: *const i8, argv: *const *const i8) -> io::Result<ChildInner> {
    let mut authref: AuthorizationRef = ptr::null_mut();
    let mut pipe: *mut libc::FILE = ptr::null_mut();

    if AuthorizationCreate(
        ptr::null(),
        ptr::null(),
        kAuthorizationFlagDefaults,
        &mut authref,
    ) != errAuthorizationSuccess
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "AuthorizationCreate failed",
        ));
    }
    if AuthorizationExecuteWithPrivileges(
        authref,
        prog,
        kAuthorizationFlagDefaults,
        argv as *const *mut _,
        &mut pipe,
    ) != errAuthorizationSuccess
    {
        AuthorizationFree(authref, kAuthorizationFlagDestroyRights);
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "AuthorizationExecuteWithPrivileges failed",
        ));
    }

    let pid = fcntl(fileno(pipe), F_GETOWN, 0);
    if pid == -1 {
        AuthorizationFree(authref, kAuthorizationFlagDestroyRights);
        return Err(io::Error::last_os_error());
    }

    Ok(ChildInner {
        pid,
        auth_ref: authref,
    })
}

pub fn runas_spawn(cmd: &Command) -> io::Result<crate::runas::Child> {
    let exe: OsString = match get_exe_path(&cmd.command) {
        Some(exe) => exe.into(),
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Command `{}` not found", cmd.command.to_string_lossy()),
            ));
        }
    };
    let prog = make_cstring!(exe);
    let mut args = vec![];
    for arg in cmd.args.iter() {
        args.push(make_cstring!(arg))
    }
    let mut argv: Vec<_> = args.iter().map(|x| x.as_ptr()).collect();
    argv.push(ptr::null());

    unsafe { gui_spawn(prog.as_ptr(), argv.as_ptr()).map(crate::runas::Child::new) }
}
