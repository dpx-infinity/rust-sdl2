use std::io;
use std::io::IoResult;
use get_error;
use libc::{c_void, c_int, size_t};

#[allow(non_camel_case_types)]
pub mod ll {
    use libc::{c_uchar, uint32_t, c_char, FILE, c_void};
    use libc::{c_int, int64_t, size_t};

    struct SDL_RWops_Anon {
        data: [c_uchar, ..24],
    }

    pub type SDL_bool = c_int;

    pub static RW_SEEK_SET: c_int = 0;
    pub static RW_SEEK_CUR: c_int = 1;
    pub static RW_SEEK_END: c_int = 2;

    pub struct SDL_RWops {
        pub size:  extern "C" fn(context: *SDL_RWops) -> int64_t,
        pub seek:  extern "C" fn(context: *SDL_RWops, offset: int64_t, whence: c_int) -> int64_t,
        pub read:  extern "C" fn(context: *SDL_RWops, ptr: *c_void,
                                 size: size_t, maxnum: size_t) -> size_t,
        pub write: extern "C" fn(context: *SDL_RWops, ptr: *c_void,
                                 size: size_t, maxnum: size_t) -> size_t,
        pub close: extern "C" fn(context: *SDL_RWops) -> c_int,
        pub _type: uint32_t,
        hidden: SDL_RWops_Anon
    }

    extern "C" {
        pub fn SDL_RWFromFile(file: *c_char, mode: *c_char) -> *SDL_RWops;
        pub fn SDL_RWFromFP(fp: *FILE, autoclose: SDL_bool) -> *SDL_RWops;
        pub fn SDL_RWFromMem(mem: *c_void, size: c_int) -> *SDL_RWops;
        pub fn SDL_RWFromConstMem(mem: *c_void, size: c_int) -> *SDL_RWops;

        pub fn SDL_AllocRW() -> *SDL_RWops;
        pub fn SDL_FreeRW(area: *SDL_RWops);
    }
}

#[deriving(Eq)] #[allow(raw_pointer_deriving)]
pub struct RWops {
    raw: *ll::SDL_RWops,
    close_on_drop: bool
}

impl_raw_accessors!(RWops, *ll::SDL_RWops)
impl_owned_accessors!(RWops, close_on_drop)

/// A structure that provides an abstract interface to stream I/O.
impl RWops {
    pub fn from_file(path: &Path, mode: &str) -> Result<RWops, ~str> {
        let raw = unsafe {
            ll::SDL_RWFromFile(path.to_c_str().unwrap(), mode.to_c_str().unwrap())
        };
        if raw.is_null() { Err(get_error()) }
        else { Ok(RWops{raw: raw, close_on_drop: true}) }
    }

    pub fn from_bytes(buf: &[u8]) -> Result<RWops, ~str> {
        let raw = unsafe {
            ll::SDL_RWFromConstMem(buf.as_ptr() as *c_void, buf.len() as c_int)
        };
        if raw.is_null() { Err(get_error()) }
        else { Ok(RWops{raw: raw, close_on_drop: false}) }
    }
}

impl Drop for RWops {
    fn drop(&mut self) {
        // TODO: handle close error
        if self.close_on_drop {
            let ret = unsafe { ((*self.raw).close)(self.raw) };
            if ret != 0 {
                println!("error {} when closing RWopt {:?}", get_error(), self);
            }
        }
    }
}

impl Reader for RWops {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
        let out_len = buf.len() as size_t;
        // FIXME: it's better to use as_mut_ptr().
        // number of objects read, or 0 at error or end of file.
        let ret = unsafe {
            ((*self.raw).read)(self.raw, buf.as_ptr() as *c_void, 1, out_len)
        };
        if ret == 0 {
            Err(io::standard_error(io::EndOfFile))
        } else {
            Ok(ret as uint)
        }
    }
}

impl Writer for RWops {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        let in_len = buf.len() as size_t;
        let ret = unsafe {
            ((*self.raw).write)(self.raw, buf.as_ptr() as *c_void, 1, in_len)
        };
        if ret == 0 {
            Err(io::standard_error(io::EndOfFile))
        } else if ret != in_len {
            // FIXME: what error should we return here?
            Err(io::standard_error(io::EndOfFile))
        } else {
            Ok(())
        }
    }
}

impl Seek for RWops {
    fn tell(&self) -> IoResult<u64> {
        let ret = unsafe {
            ((*self.raw).seek)(self.raw, 0, ll::RW_SEEK_CUR)
        };
        if ret == -1 {
            Err(io::IoError::last_error())
        } else {
            Ok(ret as u64)
        }
    }

    fn seek(&mut self, pos: i64, style: io::SeekStyle) -> IoResult<()> {
        // whence code is different from SeekStyle
        let whence = match style {
            io::SeekSet => ll::RW_SEEK_SET,
            io::SeekEnd => ll::RW_SEEK_END,
            io::SeekCur => ll::RW_SEEK_CUR
        };
        let ret = unsafe {
            ((*self.raw).seek)(self.raw, pos, whence)
        };
        if ret == -1 {
            Err(io::IoError::last_error())
        } else {
            Ok(())
        }
    }
}

impl Container for RWops {
    fn len(&self) -> uint {
        unsafe {
            ((*self.raw).size)(self.raw) as uint
        }
    }
}
