mod accepter;
mod codec;
mod listener;
mod peek;
mod quic;
mod tcp;
mod tls;
mod ws;

pub use quic::listen_quic;
// use snafu::Snafu;
pub use tls::listen_tls;

use server_base::tokio;
use stream_cancel::Valve;

// #[derive(Debug, Snafu)]
// pub enum Error {
//     #[snafu(display("Io Error {err}"))]
//     IoError { err: std::io::Error },

//     #[snafu(display("EncodeError {err}"))]
//     EncodeError { err: EncodeError },

//     #[snafu(display("DecodeError {err}"))]
//     DecodeError { err: DecodeError },

//     #[snafu(display("Private Keys Error {msg}"))]
//     PrivateKeyError { msg: String },
// }

// impl From<std::io::Error> for Error {
//     fn from(err: std::io::Error) -> Self {
//         Error::IoError { err }
//     }
// }

// impl From<EncodeError> for Error {
//     fn from(err: EncodeError) -> Self {
//         Error::EncodeError { err }
//     }
// }

// impl From<DecodeError> for Error {
//     fn from(err: DecodeError) -> Self {
//         Error::DecodeError { err }
//     }
// }

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
