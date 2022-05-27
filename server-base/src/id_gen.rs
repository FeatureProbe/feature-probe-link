use crate::Protocol;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Clone)]
pub struct IdGen {
    inner: Arc<Inner>,
}
struct Inner {
    pub node_id: String,
    pub cur_seq: Mutex<u64>,
    pub cur_ts: Mutex<u64>,
}

impl Inner {
    pub fn generate(&self, proto: Protocol) -> String {
        let ts = crate::now_ts_milli();
        let mut cur_seq = self.cur_seq.lock();
        let mut cur_ts = self.cur_ts.lock();
        if *cur_ts == ts {
            *cur_seq += 1;
        } else {
            *cur_seq = 0;
            *cur_ts = ts;
        }

        format!("{}_{}_{}_{}", self.node_id, ts, *cur_seq, proto)
    }
}

impl IdGen {
    pub fn new(node_id: String) -> Self {
        Self {
            inner: Arc::new(Inner {
                node_id,
                cur_seq: Mutex::new(0),
                cur_ts: Mutex::new(0),
            }),
        }
    }

    pub fn node_id(conn_id: &str) -> Option<String> {
        conn_id.split('_').next().map(|node_id| node_id.to_owned())
    }

    pub fn conn_id(&self, proto: Protocol) -> String {
        self.inner.generate(proto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_extract() {
        let node_id = String::from("this-is-fake-node-id");
        let id_generator = IdGen::new(node_id.clone());
        let generated_conn_id = id_generator.conn_id(Protocol::Quic);
        let extract_node_id = IdGen::node_id(&generated_conn_id);

        assert_eq!(Some(node_id), extract_node_id);
    }
}
