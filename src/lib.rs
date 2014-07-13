#![crate_name = "pidfile"]
#![feature(macro_rules)]

extern crate libc;

use std::io::{FilePermission, IoResult, IoError, FileNotFound};
use std::io::fs;
use std::path::{BytesContainer, Path};
use libc::pid_t;
use file::File;

#[cfg(target_os = "macos")]
#[cfg(target_os = "ios")]
#[path = "ffi_darwin.rs"]
mod ffi;

#[cfg(unix)]
#[path = "file_posix.rs"]
mod file;

pub fn at<B: BytesContainer>(path: B) -> Request {
    Request {
        pid: pid(),
        path: Path::new(path),
        perm: FilePermission::from_bits(0o644)
            .expect("0o644 is not a valid file permission")
    }
}

pub struct Request {
    pid: pid_t,
    path: Path,
    perm: FilePermission
}

impl Request {
    pub fn lock(self) -> LockResult<Lock> {
        let res = File::open(&self.path, true, true, self.perm.bits());
        let mut f = try!(res.map_err(LockError::io_error));

        if !try!(f.lock().map_err(LockError::io_error)) {
            return Err(LockError::conflict());
        }

        try!(f.truncate().map_err(LockError::io_error));
        try!(f.write(self.pid).map_err(LockError::io_error));

        return Ok(Lock {
            pidfile: PidFile { pid: self.pid as uint },
            handle: f,
            path: self.path
        })
    }

    pub fn check(self) -> IoResult<bool> {
        let mut f = match File::open(&self.path, false, false, 0) {
            Ok(v) => v,
            Err(e) => {
                match e.kind {
                    FileNotFound => return Ok(true),
                    _ => return Err(e)
                }
            }
        };

        let pid = try!(f.check());

        Ok(pid == 0)
    }
}

/// Represents a pidfile that exists at the requested location and has an
/// active lock.
#[deriving(Clone)]
pub struct PidFile {
    pid: uint
}

impl PidFile {
    pub fn pid(&self) -> uint {
        self.pid
    }
}

pub struct Lock {
    pidfile: PidFile,
    handle: File,
    path: Path
}

impl Lock {
    pub fn pidfile(&self) -> PidFile {
        self.pidfile.clone()
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        // Some non-critical cleanup. We do not assume that the pidfile will
        // properly get cleaned up since this handler may not get executed.
        fs::unlink(&self.path);
    }
}

#[deriving(Show)]
pub struct LockError {
    conflict: bool,
    io: Option<IoError>,
}

impl LockError {
    fn conflict() -> LockError {
        LockError {
            conflict: true,
            io: None
        }
    }

    fn io_error(err: IoError) -> LockError {
        LockError {
            conflict: false,
            io: Some(err)
        }
    }
}

type LockResult<T> = Result<T, LockError>;

fn pid() -> pid_t {
    unsafe { libc::getpid() }
}