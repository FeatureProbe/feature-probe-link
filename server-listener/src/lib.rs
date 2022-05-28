mod accepter;
mod codec;
mod listener;
mod peek;
mod quic;
mod tcp;
mod tls;
mod ws;

pub use quic::listen_quic;
pub use tls::listen_tls;

use server_base::tokio;
use stream_cancel::Valve;

fn user_port_valve() -> Valve {
    let (trigger, valve) = Valve::new();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let should_stop = server_base::USER_PORT_LISTEN.read();
            if *should_stop {
                drop(trigger);
                break;
            }
        }
    });
    valve
}
