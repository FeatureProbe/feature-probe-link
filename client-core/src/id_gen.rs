use lazy_static::lazy_static;
use parking_lot::Mutex;

lazy_static! {
    pub static ref ID_GEN: IdGen = IdGen::new();
}

#[derive(Default)]
pub struct IdGen {
    inner: Mutex<IdGenInner>,
}

#[derive(Default)]
struct IdGenInner {
    cur_seq: u64,
    cur_ts: u64,
}

impl IdGen {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(IdGenInner::default()),
        }
    }

    pub fn generate(&self, addr: &str, protocol: &str) -> String {
        let ts = crate::now_ts();
        let mut inner = self.inner.lock();
        inner.cur_seq += 1;
        inner.cur_ts = ts;

        format!("{}_{}_{}_{}", protocol, addr, ts, inner.cur_seq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_extract() {
        let addr = "1.2.3.4";
        let id_gen = IdGen::new();
        let unique_id = id_gen.generate(addr, "tcp");
        println!("gen id {}", unique_id);
    }
}
