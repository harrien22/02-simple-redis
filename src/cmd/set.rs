use super::{extract_args, validate_command, CommandExecutor, Sadd, Sismember};
use crate::{cmd::CommandError, RespArray, RespFrame};

impl CommandExecutor for Sadd {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        RespFrame::Integer(backend.sadd(self.key, self.members))
    }
}

impl CommandExecutor for Sismember {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        RespFrame::Integer(backend.sismember(&self.key, &self.member))
    }
}

impl TryFrom<RespArray> for Sadd {
    type Error = CommandError;
    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["sadd"], None)?;

        let mut args = extract_args(value, 1)?.into_iter();

        let key = match args.next() {
            Some(RespFrame::BulkString(key)) => String::from_utf8(key.0.expect("Invalid key"))?,
            _ => return Err(CommandError::InvalidArgument("Invalid key".to_string())),
        };

        let mut members = vec![];
        for v in args {
            match v {
                RespFrame::BulkString(member) => {
                    members.push(String::from_utf8(member.0.expect("Invalid member"))?);
                }
                _ => return Err(CommandError::InvalidArgument("Invalid member".to_string())),
            }
        }

        Ok(Sadd { key, members })
    }
}

impl TryFrom<RespArray> for Sismember {
    type Error = CommandError;
    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["sismember"], Some(2))?;

        let mut args = extract_args(value, 1)?.into_iter();
        match (args.next(), args.next()) {
            (Some(RespFrame::BulkString(key)), Some(RespFrame::BulkString(member))) => {
                Ok(Sismember {
                    key: String::from_utf8(key.0.expect("Invalid key"))?,
                    member: String::from_utf8(member.0.expect("Invalid member"))?,
                })
            }
            _ => Err(CommandError::InvalidArgument(
                "Invalid key or member".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Backend, RespDecode};
    use anyhow::Result;
    use bytes::BytesMut;

    #[test]
    fn test_sadd_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*4\r\n$4\r\nsadd\r\n$5\r\nmyset\r\n$5\r\nhello\r\n$5\r\nworld\r\n");

        let frame = RespArray::decode(&mut buf)?;

        let result: Sadd = frame.try_into()?;
        assert_eq!(result.key, "myset");
        assert_eq!(result.members, vec!["hello".to_owned(), "world".to_owned()]);

        Ok(())
    }

    #[test]
    fn test_sismember_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*3\r\n$9\r\nsismember\r\n$5\r\nmyset\r\n$5\r\nhello\r\n");

        let frame = RespArray::decode(&mut buf)?;

        let result: Sismember = frame.try_into()?;
        assert_eq!(result.key, "myset");
        assert_eq!(result.member, "hello");

        Ok(())
    }

    #[test]
    fn test_sadd_sismember_command() -> Result<()> {
        let backend = Backend::new();
        let cmd = Sadd {
            key: "myset".to_string(),
            members: vec!["hello".to_owned(), "world".to_owned()],
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, RespFrame::Integer(2));

        let cmd = Sismember {
            key: "myset".to_string(),
            member: "hello".to_string(),
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, RespFrame::Integer(1));

        Ok(())
    }
}
