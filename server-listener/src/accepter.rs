use futures::sink::SinkExt;
use futures::{Sink, Stream, StreamExt};
use parking_lot::Mutex;
use server_base::tokio;
use server_base::{ConnContext, HandyMutex, RecvMessage};
use std::sync::Arc;
use std::time::{Duration, Instant};
use stream_cancel::{Trigger, Valve, Valved};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub struct Accepter {
    context: Arc<ConnContext>,
    incoming_time: Arc<Mutex<Instant>>,
}

impl Accepter {
    pub fn new(context: ConnContext) -> Self {
        Self {
            context: Arc::new(context),
            incoming_time: Arc::new(Mutex::new(Instant::now())),
        }
    }

    fn start_timer(&self, trigger: Arc<Mutex<Option<Trigger>>>) {
        let incoming_time = self.incoming_time.clone();
        let timeout_secs = self.context.timeout;
        tokio::spawn(async move {
            let mut elapsed = 0;
            loop {
                if elapsed < timeout_secs {
                    let delay = tokio::time::sleep(Duration::from_secs(timeout_secs - elapsed));
                    delay.await;
                } else {
                    break;
                }
                elapsed = incoming_time.l().elapsed().as_secs();
            }
            trigger.l().take()
        });
    }

    pub fn accept_stream<R, W, T, E>(
        self,
        reader: R,
        mut writer: W,
        recv_msg: Box<dyn RecvMessage<Item = T>>,
        receiver: UnboundedReceiver<T>,
    ) where
        R: Stream<Item = Result<T, E>> + Unpin + Send + 'static,
        W: Sink<T, Error = E> + Unpin + Send + 'static,
        E: std::fmt::Debug + Send,
        T: Send + 'static,
    {
        self.context.on_conn_create();
        let (trigger, valve) = Valve::new();
        let trigger = Arc::new(Mutex::new(Some(trigger)));
        self.start_timer(trigger.clone());
        let receiver_stream = UnboundedReceiverStream::new(receiver);
        let mut receiver = valve.wrap(receiver_stream);
        let reader = valve.wrap(reader);
        tokio::spawn(async move {
            self.do_read(reader, recv_msg, trigger).await;
        });

        tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                if let Err(e) = writer.send(msg).await {
                    log::warn!("send err: {:?}, write channel finished!", e);
                    return;
                }
            }
        });
    }

    async fn do_read<R, T, E>(
        self,
        mut reader: Valved<R>,
        recv_msg: Box<dyn RecvMessage<Item = T>>,
        trigger: Arc<Mutex<Option<Trigger>>>,
    ) where
        R: Stream<Item = Result<T, E>> + Unpin + Send + 'static,
        E: std::fmt::Debug + Send,
        T: Send + 'static,
    {
        while let Some(result) = reader.next().await {
            let item = match result {
                Ok(item) => item,
                Err(e) => {
                    log::warn!("read err: {:?}", e);
                    break;
                }
            };
            match recv_msg.recv(item) {
                Ok(Some(message)) => {
                    self.context.accept_message(message);
                }
                Ok(None) => {}
                Err(e) => {
                    log::warn!("read error {:?}", e);
                    break;
                }
            }
            // update incoming time
            if !self.context.should_timeout() {
                let mut time = self.incoming_time.l();
                *time = Instant::now();
            }
        }
        if let Some(trigger) = trigger.l().take() {
            trigger.cancel();
        }
    }
}

impl Drop for Accepter {
    fn drop(&mut self) {
        self.context.on_conn_destroy();
    }
}
