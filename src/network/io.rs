use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use serde::{Serialize, de::DeserializeOwned};
use anyhow::Result;
use bincode;

pub async fn send_message<T, S>(stream: &mut S, msg: &T) -> Result<()>
where
    T: Serialize,
    S: AsyncWrite + Unpin
{
    let data = bincode::serialize(msg)?;
    let len = (data.len() as u32).to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&data).await?;
    Ok(())
}

pub async fn recv_message<T, S>(stream: &mut S) -> Result<T>
where 
    T: DeserializeOwned,
    S: AsyncRead + Unpin
{
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);

    let mut buf = vec![0u8; len as usize];
    stream.read_exact(&mut buf).await?;
    Ok(bincode::deserialize(&buf)?)
}
