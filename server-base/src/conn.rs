use crate::context::ConnContext;
use crate::Protocol;
use common_proto::proto::Message;
use std::sync::Arc;

#[derive(Clone)]
pub struct Conn {
    pub inner: Arc<ConnContext>,
}

impl Conn {
    pub fn id(&self) -> &String {
        &self.inner.conn_id
    }

    pub fn protocol(&self) -> &Protocol {
        &self.inner.proto
    }

    pub fn create_time(&self) -> u64 {
        self.inner.create_time
    }

    #[allow(clippy::result_unit_err)]
    pub fn push(&self, message: Message) -> Result<(), ()> {
        self.inner.sender.send(message)
    }
}

impl PartialEq for Conn {
    fn eq(&self, other: &Self) -> bool {
        self.inner.conn_id == other.inner.conn_id
    }
}

impl Eq for Conn {}

impl std::fmt::Debug for Conn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Conn {{ id: {} ,proto: {} }}",
            self.inner.conn_id, self.inner.proto
        )
    }
}

impl std::hash::Hash for Conn {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.conn_id.hash(state);
    }
}

impl From<Arc<ConnContext>> for Conn {
    fn from(context: Arc<ConnContext>) -> Self {
        Self { inner: context }
    }
}
