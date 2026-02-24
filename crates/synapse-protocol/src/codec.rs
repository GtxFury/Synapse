use anyhow::{bail, Result};
use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::message::Message;

/// 最大帧大小: 16 MB
const MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;

/// 长度前缀帧编解码器
///
/// 帧格式: `[u32 BE 长度][bincode 载荷]`
pub struct MessageCodec;

impl Decoder for MessageCodec {
    type Item = Message;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        // 至少需要 4 字节读取长度
        if src.len() < 4 {
            return Ok(None);
        }

        // 读取帧长度（不消费）
        let len = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        if len as u32 > MAX_FRAME_SIZE {
            bail!("frame too large: {} bytes (max {})", len, MAX_FRAME_SIZE);
        }

        // 等待完整帧
        if src.len() < 4 + len {
            src.reserve(4 + len - src.len());
            return Ok(None);
        }

        // 消费长度前缀
        src.advance(4);
        let payload = src.split_to(len);

        let msg: Message = bincode::deserialize(&payload)?;
        Ok(Some(msg))
    }
}

impl Encoder<Message> for MessageCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<()> {
        let payload = bincode::serialize(&item)?;
        let len = payload.len() as u32;

        if len > MAX_FRAME_SIZE {
            bail!("frame too large: {} bytes (max {})", len, MAX_FRAME_SIZE);
        }

        dst.reserve(4 + payload.len());
        dst.put_u32(len);
        dst.extend_from_slice(&payload);
        Ok(())
    }
}
