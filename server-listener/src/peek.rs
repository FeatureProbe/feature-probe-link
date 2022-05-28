use anyhow::Result;
use server_base::tokio;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};

pub(crate) struct PeekStream<S: AsyncRead + AsyncWrite + Unpin> {
    stream: S,
    peek_buf: Option<Vec<u8>>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> PeekStream<S> {
    pub fn new(stream: S) -> Self {
        PeekStream {
            stream,
            peek_buf: None,
        }
    }

    pub async fn peek(&mut self, length: u8) -> Result<Vec<u8>> {
        let mut buf = [0, length];
        let n = self.stream.read(&mut buf).await?;
        let buf = buf[0..n].to_vec();
        self.peek_buf = Some(buf.clone());
        Ok(buf)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for PeekStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let me = self.get_mut();
        if let Some(peek_buf) = me.peek_buf.take() {
            buf.put_slice(&peek_buf);
        }
        let stream = &mut me.stream;
        Pin::new(stream).poll_read(cx, buf)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for PeekStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let me = self.get_mut();
        let stream = &mut me.stream;
        Pin::new(stream).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let me = self.get_mut();
        let stream = &mut me.stream;
        Pin::new(stream).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let me = self.get_mut();
        let stream = &mut me.stream;
        Pin::new(stream).poll_shutdown(cx)
    }
}
