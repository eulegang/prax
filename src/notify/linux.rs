use std::{
    ffi::{c_int, OsStr},
    io,
    mem::size_of,
    os::{fd::FromRawFd, unix::ffi::OsStrExt},
    path::{Path, PathBuf},
};

use tokio::{fs::File, io::AsyncReadExt};

extern "C" {
    fn inotify_init1(flag: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, buf: *const u8, mask: u32) -> c_int;
    fn inotify_rm_watch(fd: c_int, wd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

pub struct INotify {
    fd: c_int,
    file: File,
}

#[derive(Clone, Copy)]
pub struct Watch {
    wd: c_int,
}

#[derive(Clone, Copy)]
pub struct Mask(u32);

#[derive(Debug)]
pub struct Event {
    pub watch: Watch,
    pub mask: Mask,
    pub cookie: u32,
    pub path: PathBuf,
}

#[repr(C)]
pub struct EventHeader {
    wd: c_int,
    mask: u32,
    cookie: u32,
    len: u32,
}

impl INotify {
    pub fn new() -> io::Result<Self> {
        let fd = unsafe { inotify_init1(0) };

        if fd == -1 {
            return Err(io::Error::from_raw_os_error(fd));
        }

        let file = unsafe { File::from_raw_fd(fd) };

        Ok(Self { fd, file })
    }

    pub fn add(&mut self, path: &Path, mask: Mask) -> io::Result<Watch> {
        log::debug!("adding watch {} {:?}", path.display(), mask);
        let path: &OsStr = path.as_ref();
        let res = unsafe { inotify_add_watch(self.fd, path.as_bytes().as_ptr(), mask.0) };
        if res == -1 {
            return Err(io::Error::from_raw_os_error(res));
        }

        Ok(Watch { wd: res })
    }

    pub fn rm(&mut self, watch: Watch) -> io::Result<()> {
        let res = unsafe { inotify_rm_watch(self.fd, watch.wd) };
        if res == -1 {
            return Err(io::Error::from_raw_os_error(res));
        }

        Ok(())
    }

    pub async fn watch(&mut self) -> io::Result<Event> {
        const SIZE: usize = size_of::<EventHeader>();
        let mut buffer = [0u8; SIZE];

        let mut amt = 0;
        while amt < SIZE {
            amt += self.file.read(&mut buffer[amt..SIZE]).await?;
        }

        let header: EventHeader = unsafe { std::mem::transmute(buffer) };
        let total = header.len as usize;
        let mut buffer = [0u8; 0x1000];

        let mut amt: usize = 0;
        while amt < total {
            amt += self.file.read(&mut buffer[amt..total]).await?;
        }

        let os = OsStr::from_bytes(&buffer[0..total]);
        let path = PathBuf::from(os);

        Ok(Event {
            watch: Watch { wd: header.wd },
            mask: Mask(header.mask),
            cookie: header.cookie,
            path,
        })
    }

    pub async fn close(self) -> io::Result<()> {
        std::mem::forget(self.file);
        let res = unsafe { close(self.fd) };

        if res == -1 {
            return Err(io::Error::from_raw_os_error(res));
        }

        Ok(())
    }
}

impl Mask {
    /// File Accessed
    pub const IN_ACCESS: Mask = Mask(0x00000001);

    /// File modified
    pub const IN_MODIFY: Mask = Mask(0x00000002);
    /// Metadata changed
    pub const IN_ATTRIB: Mask = Mask(0x00000004);
    /// Writtable file was closed
    pub const IN_CLOSE_WRITE: Mask = Mask(0x00000008);
    /// Unwrittable file closed
    pub const IN_CLOSE_NOWRITE: Mask = Mask(0x00000010);
    /// File was opened
    pub const IN_OPEN: Mask = Mask(0x00000020);
    /// File was moved from X */
    pub const IN_MOVED_FROM: Mask = Mask(0x00000040);
    /// File was moved to Y */
    pub const IN_MOVED_TO: Mask = Mask(0x00000080);
    /// Subfile was created */
    pub const IN_CREATE: Mask = Mask(0x00000100);
    /// Subfile was deleted */
    pub const IN_DELETE: Mask = Mask(0x00000200);
    /// Self was deleted */
    pub const IN_DELETE_SELF: Mask = Mask(0x00000400);
    /// Self was moved */
    pub const IN_MOVE_SELF: Mask = Mask(0x00000800);

    /// Backing fs was unmounted */
    pub const IN_UNMOUNT: Mask = Mask(0x00002000);
    /// Event queued overflowed */
    pub const IN_Q_OVERFLOW: Mask = Mask(0x00004000);
    /// File was ignored */
    pub const IN_IGNORED: Mask = Mask(0x00008000);

    /// Close
    pub const IN_CLOSE: Mask = Mask(Self::IN_CLOSE_WRITE.0 | Self::IN_CLOSE_NOWRITE.0);

    /// Moves
    pub const IN_MOVE: Mask = Mask(Self::IN_MOVED_TO.0 | Self::IN_MOVED_FROM.0);

    /// only watch the path if it is a directory */
    pub const IN_ONLYDIR: Mask = Mask(0x01000000);
    /// don't follow a sym link */
    pub const IN_DONT_FOLLOW: Mask = Mask(0x02000000);
    /// exclude events on unlinked objects */
    pub const IN_EXCL_UNLINK: Mask = Mask(0x04000000);
    /// only create watches */
    pub const IN_MASK_CREATE: Mask = Mask(0x10000000);
    /// add to the mask of an already existing watch */
    pub const IN_MASK_ADD: Mask = Mask(0x20000000);
    /// event occurred against dir */
    pub const IN_ISDIR: Mask = Mask(0x40000000);
    /// only send event once */
    pub const IN_ONESHOT: Mask = Mask(0x80000000);

    pub fn contains(self, other: Mask) -> bool {
        (self & other) == other
    }
}

impl PartialEq for Mask {
    fn eq(&self, other: &Self) -> bool {
        const ALL: u32 = 0xF700EFFF;

        (self.0 & ALL) == (other.0 & ALL)
    }
}

impl std::ops::BitAnd<Mask> for Mask {
    type Output = Mask;

    fn bitand(self, rhs: Mask) -> Self::Output {
        Mask(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for Mask {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0
    }
}

impl std::ops::BitOr<Mask> for Mask {
    type Output = Mask;

    fn bitor(self, rhs: Mask) -> Self::Output {
        Mask(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Mask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Watch").field(&self.wd).finish()?;
        Ok(())
    }
}

const CHECK: &[(Mask, &str)] = &[
    (Mask::IN_ACCESS, "IN_ACCESS"),
    (Mask::IN_MODIFY, "IN_MODIFY"),
    (Mask::IN_ATTRIB, "IN_ATTRIB"),
    (Mask::IN_CLOSE_WRITE, "IN_CLOSE_WRITE"),
    (Mask::IN_CLOSE_NOWRITE, "IN_CLOSE_NOWRITE"),
    (Mask::IN_OPEN, "IN_OPEN"),
    (Mask::IN_MOVED_FROM, "IN_MOVED_FROM"),
    (Mask::IN_MOVED_TO, "IN_MOVED_TO"),
    (Mask::IN_CREATE, "IN_CREATE"),
    (Mask::IN_DELETE, "IN_DELETE"),
    (Mask::IN_DELETE_SELF, "IN_DELETE_SELF"),
    (Mask::IN_MOVE_SELF, "IN_MOVE_SELF"),
    (Mask::IN_UNMOUNT, "IN_UNMOUNT"),
    (Mask::IN_Q_OVERFLOW, "IN_Q_OVERFLOW"),
    (Mask::IN_IGNORED, "IN_IGNORED"),
    (Mask::IN_ONLYDIR, "IN_ONLYDIR"),
    (Mask::IN_DONT_FOLLOW, "IN_DONT_FOLLOW"),
    (Mask::IN_EXCL_UNLINK, "IN_EXCL_UNLINK"),
    (Mask::IN_MASK_CREATE, "IN_MASK_CREATE"),
    (Mask::IN_MASK_ADD, "IN_MASK_ADD"),
    (Mask::IN_ISDIR, "IN_ISDIR"),
    (Mask::IN_ONESHOT, "IN_ONESHOT"),
];

impl std::fmt::Debug for Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;

        write!(f, "({:X}) ", self.0)?;

        for (mask, repr) in CHECK {
            if (*self & *mask).0 != 0 {
                if !first {
                    write!(f, " | ")?;
                } else {
                    first = false;
                }

                write!(f, "{}", repr)?;
            }
        }

        Ok(())
    }
}
