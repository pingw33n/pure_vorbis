use std::io;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Undecodable(&'static str),
    WrongPacketKind(&'static str),
    ExpectedEof(&'static str),
    Io(io::Error),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ErrorKind {
    Undecodable,
    WrongPacketKind,
    ExpectedEof,
    Io,
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        match self {
            &Error::Undecodable(_)      => ErrorKind::Undecodable,
            &Error::ExpectedEof(_)      => ErrorKind::ExpectedEof,
            &Error::WrongPacketKind(_)  => ErrorKind::WrongPacketKind,
            &Error::Io(_)               => ErrorKind::Io,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

pub trait ExpectEof<T> {
    fn expect_eof(self) -> Result<T>;
}

impl<T> ExpectEof<T> for Result<T> {
    fn expect_eof(self) -> Result<T> {
        match self {
            Err(Error::Io(e)) => Err(expect_eof(e)),
            v => v,
        }
    }
}

impl<T> ExpectEof<T> for io::Result<T> {
    fn expect_eof(self) -> Result<T> {
        match self {
            Err(e) => Err(expect_eof(e)),
            Ok(r) => Ok(r),
        }
    }
}

fn expect_eof(e: io::Error) -> Error {
    if e.kind() == io::ErrorKind::UnexpectedEof {
        Error::ExpectedEof("Expected EOF")
    } else {
        From::from(e)
    }
}