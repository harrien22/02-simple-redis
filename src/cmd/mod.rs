mod echo;
mod hmap;
mod map;
mod set;

use crate::{Backend, RespArray, RespError, RespFrame, SimpleString};
use enum_dispatch::enum_dispatch;
use lazy_static::lazy_static;
use thiserror::Error;

// you could also use once_cell instead of lazy_static
lazy_static! {
    static ref RESP_OK: RespFrame = SimpleString::new("OK").into();
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("{0}")]
    RespError(#[from] RespError),
    #[error("Utf8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

#[enum_dispatch]
pub trait CommandExecutor {
    fn execute(self, backend: &Backend) -> RespFrame;
}

#[enum_dispatch(CommandExecutor)]
#[derive(Debug)]
pub enum Command {
    Get(Get),
    Set(Set),
    HGet(HGet),
    HSet(HSet),
    HMGet(HMGet),
    HGetAll(HGetAll),
    Echo(Echo),
    Sadd(Sadd),
    Sismember(Sismember),

    // unrecognized command
    Unrecognized(Unrecognized),
}

#[derive(Debug)]
pub struct Sadd {
    key: String,
    members: Vec<String>,
}

#[derive(Debug)]
pub struct Sismember {
    key: String,
    member: String,
}

#[derive(Debug)]
pub struct Get {
    key: String,
}

#[derive(Debug)]
pub struct Set {
    key: String,
    value: RespFrame,
}

#[derive(Debug)]
pub struct HGet {
    key: String,
    field: String,
}

#[derive(Debug)]
pub struct HSet {
    key: String,
    field: String,
    value: RespFrame,
}

#[derive(Debug)]
pub struct HMGet {
    key: String,
    fields: Vec<String>,
}

#[derive(Debug)]
pub struct HGetAll {
    key: String,
    sort: bool,
}

#[derive(Debug)]
pub struct Echo {
    message: String,
}

#[derive(Debug)]
pub struct Unrecognized;

impl TryFrom<RespFrame> for Command {
    type Error = CommandError;
    fn try_from(v: RespFrame) -> Result<Self, Self::Error> {
        match v {
            RespFrame::Array(array) => array.try_into(),
            _ => Err(CommandError::InvalidCommand(
                "Command must be an Array".to_string(),
            )),
        }
    }
}

impl TryFrom<RespArray> for Command {
    type Error = CommandError;
    fn try_from(v: RespArray) -> Result<Self, Self::Error> {
        match &v.0 {
            Some(frames) => match frames.first() {
                Some(RespFrame::BulkString(ref cmd)) => match cmd.as_ref() {
                    b"get" => Ok(Get::try_from(v)?.into()),
                    b"set" => Ok(Set::try_from(v)?.into()),
                    b"hget" => Ok(HGet::try_from(v)?.into()),
                    b"hset" => Ok(HSet::try_from(v)?.into()),
                    b"hmget" => Ok(HMGet::try_from(v)?.into()),
                    b"hgetall" => Ok(HGetAll::try_from(v)?.into()),
                    b"echo" => Ok(Echo::try_from(v)?.into()),
                    b"sadd" => Ok(Sadd::try_from(v)?.into()),
                    b"sismember" => Ok(Sismember::try_from(v)?.into()),
                    _ => Ok(Unrecognized.into()),
                },
                _ => Err(CommandError::InvalidCommand(
                    "Command must have a BulkString as the first argument".to_string(),
                )),
            },
            None => Err(CommandError::InvalidCommand(
                "Command must have a BulkString as the first argument".to_string(),
            )),
        }
        // match v.first() {
        //     Some(RespFrame::BulkString(ref cmd)) => match cmd.as_ref() {
        //         b"get" => Ok(Get::try_from(v)?.into()),
        //         b"set" => Ok(Set::try_from(v)?.into()),
        //         b"hget" => Ok(HGet::try_from(v)?.into()),
        //         b"hset" => Ok(HSet::try_from(v)?.into()),
        //         b"hgetall" => Ok(HGetAll::try_from(v)?.into()),
        //         _ => Ok(Unrecognized.into()),
        //     },
        //     _ => Err(CommandError::InvalidCommand(
        //         "Command must have a BulkString as the first argument".to_string(),
        //     )),
        // }
    }
}

impl CommandExecutor for Unrecognized {
    fn execute(self, _: &Backend) -> RespFrame {
        RESP_OK.clone()
    }
}

fn validate_command(
    value: &RespArray,
    names: &[&'static str],
    n_args: Option<usize>,
) -> Result<(), CommandError> {
    if value.is_none() {
        return Err(CommandError::InvalidCommand(
            "Command must have a BulkString as the first argument".to_string(),
        ));
    }

    let value = value.as_ref().unwrap();

    if let Some(n_args) = n_args {
        if value.len() != n_args + names.len() {
            return Err(CommandError::InvalidArgument(format!(
                "{} command must have exactly {} argument",
                names.join(" "),
                n_args
            )));
        }
    }

    for (i, name) in names.iter().enumerate() {
        match value[i] {
            RespFrame::BulkString(ref cmd) => {
                if cmd.as_ref().to_ascii_lowercase() != name.as_bytes() {
                    return Err(CommandError::InvalidCommand(format!(
                        "Invalid command: expected {}, got {}",
                        name,
                        String::from_utf8_lossy(cmd.as_ref())
                    )));
                }
            }
            _ => {
                return Err(CommandError::InvalidCommand(
                    "Command must have a BulkString as the first argument".to_string(),
                ))
            }
        }
    }
    Ok(())
}

fn extract_args(value: RespArray, start: usize) -> Result<Vec<RespFrame>, CommandError> {
    match value.0 {
        Some(frames) => Ok(frames.into_iter().skip(start).collect::<Vec<RespFrame>>()),
        None => Err(CommandError::InvalidCommand(
            "Command must have a BulkString as the first argument".to_string(),
        )),
    }

    // Ok(value.0.into_iter().skip(start).collect::<Vec<RespFrame>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RespDecode, RespNull};
    use anyhow::Result;
    use bytes::BytesMut;

    #[test]
    fn test_command() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nget\r\n$5\r\nhello\r\n");

        let frame = RespArray::decode(&mut buf)?;

        let cmd: Command = frame.try_into()?;

        let backend = Backend::new();

        let ret = cmd.execute(&backend);
        assert_eq!(ret, RespFrame::Null(RespNull));

        Ok(())
    }
}
