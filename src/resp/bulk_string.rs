use std::ops::Deref;

use bytes::{Buf, BytesMut};

use crate::{RespDecode, RespEncode, RespError};

use super::{check_null_bulkstring, parse_length, CRLF_LEN};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct BulkString(pub(crate) Option<Vec<u8>>);

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
// pub struct RespNullBulkString;

// - bulk string: "$<length>\r\n<data>\r\n"
impl RespEncode for BulkString {
    fn encode(self) -> Vec<u8> {
        match self.0 {
            Some(data) => {
                let mut buf = Vec::with_capacity(data.len() + 16);
                buf.extend_from_slice(&format!("${}\r\n", data.len()).into_bytes());
                buf.extend_from_slice(&data);
                buf.extend_from_slice(b"\r\n");
                buf
            }
            None => b"$-1\r\n".to_vec(),
        }
    }
}

impl RespDecode for BulkString {
    const PREFIX: &'static str = "$";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        if check_null_bulkstring(buf)? {
            return Ok(BulkString(None));
        }

        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let remained = &buf[end + CRLF_LEN..];
        if remained.len() < len + CRLF_LEN {
            return Err(RespError::NotComplete);
        }

        buf.advance(end + CRLF_LEN);

        let data = buf.split_to(len + CRLF_LEN);
        Ok(BulkString(Some(data[..len].to_vec())))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN + len + CRLF_LEN)
    }
}

// - null bulk string: "$-1\r\n"
// impl RespEncode for RespNullBulkString {
//     fn encode(self) -> Vec<u8> {
//         b"$-1\r\n".to_vec()
//     }
// }

// impl RespDecode for RespNullBulkString {
//     const PREFIX: &'static str = "$";
//     fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
//         extract_fixed_data(buf, "$-1\r\n", "NullBulkString")?;
//         Ok(RespNullBulkString)
//     }

//     fn expect_length(_buf: &[u8]) -> Result<usize, RespError> {
//         Ok(5)
//     }
// }

impl BulkString {
    pub fn new(s: Option<impl Into<Vec<u8>>>) -> Self {
        BulkString(s.map(|v| v.into()))
    }

    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }
}

impl AsRef<[u8]> for BulkString {
    fn as_ref(&self) -> &[u8] {
        match &self.0 {
            Some(data) => data.as_ref(),
            None => &[],
        }
    }
}

impl Deref for BulkString {
    type Target = Option<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Option<&str>> for BulkString {
    fn from(s: Option<&str>) -> Self {
        match s {
            Some(s) => BulkString::new(Some(s.as_bytes().to_vec())),
            None => BulkString(None),
        }
    }
}

impl From<&str> for BulkString {
    fn from(s: &str) -> Self {
        BulkString::new(Some(s.as_bytes().to_vec()))
    }
}

impl From<Option<String>> for BulkString {
    fn from(s: Option<String>) -> Self {
        match s {
            Some(s) => BulkString::new(Some(s.into_bytes())),
            None => BulkString(None),
        }
    }
}

impl From<String> for BulkString {
    fn from(s: String) -> Self {
        BulkString::new(Some(s.into_bytes()))
    }
}

impl From<Option<&[u8]>> for BulkString {
    fn from(s: Option<&[u8]>) -> Self {
        match s {
            Some(s) => BulkString::new(Some(s.to_vec())),
            None => BulkString(None),
        }
    }
}

impl From<&[u8]> for BulkString {
    fn from(s: &[u8]) -> Self {
        BulkString::new(Some(s.to_vec()))
    }
}

impl<const N: usize> From<Option<&[u8; N]>> for BulkString {
    fn from(s: Option<&[u8; N]>) -> Self {
        match s {
            Some(s) => BulkString::new(Some(s.to_vec())),
            None => BulkString(None),
        }
    }
}

impl<const N: usize> From<&[u8; N]> for BulkString {
    fn from(s: &[u8; N]) -> Self {
        BulkString::new(Some(s.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use crate::RespFrame;

    use super::*;
    use anyhow::Result;

    #[test]
    fn test_bulk_string_encode() {
        let frame: RespFrame = BulkString::new(Some(b"hello".to_vec())).into();
        assert_eq!(frame.encode(), b"$5\r\nhello\r\n");
    }

    // #[test]
    // fn test_null_bulk_string_encode() {
    //     let frame: RespFrame = RespNullBulkString.into();
    //     assert_eq!(frame.encode(), b"$-1\r\n");
    // }

    #[test]
    fn test_bulk_string_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"$5\r\nhello\r\n");

        let frame = BulkString::decode(&mut buf)?;
        assert_eq!(frame, BulkString(Some(b"hello".into())));

        buf.extend_from_slice(b"$5\r\nhello");
        let ret = BulkString::decode(&mut buf);
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);

        buf.extend_from_slice(b"\r\n");
        let frame = BulkString::decode(&mut buf)?;
        assert_eq!(frame, BulkString(Some(b"hello".into())));

        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"$-1\r\n");
        let frame = BulkString::decode(&mut buf)?;
        assert_eq!(frame, BulkString(None));

        Ok(())
    }

    // #[test]
    // fn test_null_bulk_string_decode() -> Result<()> {
    //     let mut buf = BytesMut::new();
    //     buf.extend_from_slice(b"$-1\r\n");

    //     let frame = RespNullBulkString::decode(&mut buf)?;
    //     assert_eq!(frame, RespNullBulkString);

    //     Ok(())
    // }
}
