use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use hyper::Body;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

use crate::utils::*;

pub type Client<T> = hyper::Client<T, Body>;

pub struct StartTaskInfo<T: ClientBounds> {
    pub site: Arc<String>,
    pub tx: Sender<SiteInfo>,
    pub client: Client<T>,
}

pub struct SiteInfo {
    pub site: Arc<String>,
    pub responder: oneshot::Sender<Response>,
}

impl Debug for SiteInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.site)
    }
}

#[derive(Clone, Debug)]
pub struct Response {
    pub to_process: bool,
}

impl Response {
    pub fn to_process(&self) -> bool {
        self.to_process
    }
}