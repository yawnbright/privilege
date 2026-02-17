use std::ffi::OsStr;
use std::io;
use std::mem;
use std::os::raw::c_ushort;
use std::os::windows::ffi::OsStrExt;
use std::process::ExitStatus;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Com::{
    CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, TerminateProcess, WaitForSingleObject,
};
use windows_sys::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_NORMAL};

use crate::runas::Command;

pub struct ChildInner {
    handle: HANDLE,
}

impl ChildInner {
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        unsafe {
            WaitForSingleObject(self.handle, 0xFFFFFFFF);
            let mut code: u32 = 0;
            if GetExitCodeProcess(self.handle, &mut code) == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(mem::transmute(code))
        }
    }

    pub fn kill(&mut self) -> io::Result<()> {
        unsafe {
            if TerminateProcess(self.handle, 1) == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }

    pub fn id(&self) -> u32 {
        0
    }
}

impl Drop for ChildInner {
    fn drop(&mut self) {
        unsafe {
            if self.handle != INVALID_HANDLE_VALUE && self.handle != 0 {
                CloseHandle(self.handle);
            }
        }
    }
}

unsafe fn win_spawn(
    cmd: *const c_ushort,
    args: *const c_ushort,
    show: bool,
) -> io::Result<ChildInner> {
    let mut sei: SHELLEXECUTEINFOW = mem::zeroed();
    let verb = "runas\0".encode_utf16().collect::<Vec<u16>>();
    CoInitializeEx(
        ptr::null(),
        COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
    );

    sei.cbSize = mem::size_of::<SHELLEXECUTEINFOW>() as _;
    sei.lpVerb = verb.as_ptr();
    sei.lpFile = cmd;
    sei.lpParameters = args;
    sei.fMask = SEE_MASK_NOASYNC | SEE_MASK_NOCLOSEPROCESS;
    sei.nShow = if show { SW_NORMAL } else { SW_HIDE } as _;

    if ShellExecuteExW(&mut sei) == 0 {
        return Err(io::Error::last_os_error());
    }

    if sei.hProcess == 0 || sei.hProcess == INVALID_HANDLE_VALUE {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "No process handle returned",
        ));
    }

    Ok(ChildInner {
        handle: sei.hProcess,
    })
}

pub fn runas_spawn(cmd: &Command) -> io::Result<crate::runas::Child> {
    let mut params = String::new();
    for arg in cmd.args.iter() {
        let arg = arg.to_string_lossy();
        params.push(' ');
        if arg.len() == 0 {
            params.push_str("\"\"");
        } else if arg.find(&[' ', '\t', '"'][..]).is_none() {
            params.push_str(&arg);
        } else {
            params.push('"');
            for c in arg.chars() {
                match c {
                    '\\' => params.push_str("\\\\"),
                    '"' => params.push_str("\\\""),
                    c => params.push(c),
                }
            }
            params.push('"');
        }
    }

    let file = OsStr::new(&cmd.command)
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let params = OsStr::new(&params)
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();

    unsafe { win_spawn(file.as_ptr(), params.as_ptr(), !cmd.hide).map(crate::runas::Child::new) }
}
