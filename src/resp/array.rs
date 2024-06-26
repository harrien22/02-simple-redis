use super::{calc_total_length, extract_fixed_data, parse_length, BUF_CAP, CRLF_LEN};
use crate::{RespDecode, RespEncode, RespError, RespFrame};
use bytes::{Buf, BytesMut};
use std::ops::Deref;

const NULL_ARRAY: &str = "*-1\r\n";

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct RespArray(pub(crate) Option<Vec<RespFrame>>);

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
// pub struct RespNullArray;

// - array: "*<number-of-elements>\r\n<element-1>...<element-n>"
impl RespEncode for RespArray {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);

        match self.0 {
            Some(frames) => {
                buf.extend_from_slice(&format!("*{}\r\n", frames.len()).into_bytes());
                for frame in frames {
                    buf.extend_from_slice(&frame.encode());
                }
            }
            None => buf.extend_from_slice(NULL_ARRAY.as_bytes()),
        }

        buf
    }
}

// - array: "*<number-of-elements>\r\n<element-1>...<element-n>"
// - "*2\r\n$3\r\nget\r\n$5\r\nhello\r\n"
// FIXME: need to handle incomplete
impl RespDecode for RespArray {
    const PREFIX: &'static str = "*";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        if buf.starts_with(NULL_ARRAY.as_bytes()) {
            extract_fixed_data(buf, NULL_ARRAY, "RespArray")?;
            return Ok(RespArray::new(None));
        }

        let (end, len) = parse_length(buf, Self::PREFIX)?;

        let total_len = calc_total_length(buf, end, len, Self::PREFIX)?;

        if buf.len() < total_len {
            return Err(RespError::NotComplete);
        }

        buf.advance(end + CRLF_LEN);

        let mut frames = Vec::with_capacity(len);
        for _ in 0..len {
            frames.push(RespFrame::decode(buf)?);
        }

        Ok(RespArray::new(Some(frames)))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        if buf.starts_with(NULL_ARRAY.as_bytes()) {
            return Ok(5);
        }
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        calc_total_length(buf, end, len, Self::PREFIX)
    }
}

// - null array: "*-1\r\n"
// impl RespEncode for RespNullArray {
//     fn encode(self) -> Vec<u8> {
//         b"*-1\r\n".to_vec()
//     }
// }

// impl RespDecode for RespNullArray {
//     const PREFIX: &'static str = "*";
//     fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
//         extract_fixed_data(buf, "*-1\r\n", "NullArray")?;
//         Ok(RespNullArray)
//     }

//     fn expect_length(_buf: &[u8]) -> Result<usize, RespError> {
//         Ok(4)
//     }
// }

impl RespArray {
    pub fn new(s: impl Into<Option<Vec<RespFrame>>>) -> Self {
        RespArray(s.into())
    }
}

impl Deref for RespArray {
    type Target = Option<Vec<RespFrame>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BulkString;
    use anyhow::Result;

    #[test]
    fn test_array_encode() {
        let frame: RespFrame = RespArray::new(Some(vec![
            BulkString(Some(b"set".into())).into(),
            BulkString(Some(b"hello".into())).into(),
            BulkString(Some(b"world".into())).into(),
        ]))
        .into();
        assert_eq!(
            &frame.encode(),
            b"*3\r\n$3\r\nset\r\n$5\r\nhello\r\n$5\r\nworld\r\n"
        );
    }

    #[test]
    fn test_null_array_encode() {
        let frame: RespFrame = RespArray::new(None).into();
        assert_eq!(frame.encode(), NULL_ARRAY.as_bytes());
    }

    #[test]
    fn test_null_array_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(NULL_ARRAY.as_bytes());

        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new(None));

        Ok(())
    }

    #[test]
    fn test_array_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n$5\r\nhello\r\n");

        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(
            frame,
            RespArray::new(Some(vec![b"set".into(), b"hello".into()]))
        );

        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n");
        let ret = RespArray::decode(&mut buf);
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);

        buf.extend_from_slice(b"$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(
            frame,
            RespArray::new(Some(vec![b"set".into(), b"hello".into()]))
        );

        Ok(())
    }
}
