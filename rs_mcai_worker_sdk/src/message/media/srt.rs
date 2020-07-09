
use bytes::Bytes;
use crate::error::MessageError;
use futures_util::sink::SinkExt;
use srt::tokio::SrtSocket;
use srt::SrtSocketBuilder;
use std::{
  cell::RefCell,
  rc::Rc,
  time::Instant,
};
use tokio::runtime::Runtime;

pub struct SrtStream {
  socket: Rc<RefCell<SrtSocket>>,
  runtime: Runtime,
}

impl SrtStream {
  pub fn is_srt_stream(url: &str) -> bool {
    url.starts_with("srt://")
  }

  pub fn open_connection(url: &str) -> Result<SrtStream, MessageError> {
    let mut runtime = Runtime::new().unwrap();

    let socket = runtime.block_on(async {
        if url.starts_with("srt://:") {
          let port = url.replace("srt://:", "").parse::<u16>().unwrap();
          SrtSocketBuilder::new_listen()
            .local_port(port)
            .connect()
            .await
            .unwrap()
        } else {
          let url = url.replace("srt://", "");

          SrtSocketBuilder::new_connect(url).connect().await.unwrap()
        }
      });

    let socket = Rc::new(RefCell::new(socket));

    info!("SRT connected");
    Ok(SrtStream { socket, runtime })
  }

  pub fn send(&mut self, data: Bytes) {
    let socket = self.socket.clone();
    self.runtime.block_on(async {
        if let Err(reason) = socket
          .borrow_mut()
          .send((Instant::now(), data))
          .await
        {
          error!("unable to send message, reason: {}", reason);
        }
      });
  }
}