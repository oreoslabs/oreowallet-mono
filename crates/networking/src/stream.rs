use std::io::Read;

use oreo_errors::OreoError;
use serde::de::DeserializeOwned;
use ureq::Response;

use crate::rpc_abi::RpcResponseStream;

pub trait RequestExt {
  fn collect_stream<T: DeserializeOwned>(self) -> Result<Vec<T>, OreoError>;
}

impl RequestExt for Response {
  fn collect_stream<T: DeserializeOwned>(self) -> Result<Vec<T>, OreoError> {
      let reader = self.into_reader();
      let mut buffered = std::io::BufReader::new(reader);
      let mut items = Vec::new();
      let mut response_str = String::new();
      buffered.read_to_string(&mut response_str).map_err(|e| OreoError::InternalRpcError(e.to_string()))?;
      let lines = response_str.split('\x0c').collect::<Vec<&str>>();
      
      // Get rid of status code
      for line in lines[0..lines.len()-1].into_iter() {
        let line = *line; // Dereference to get &str
          if !line.trim().is_empty() {
              let item: RpcResponseStream<T> = serde_json::from_str(line)
                  .map_err(|e| OreoError::InternalRpcError(e.to_string()))?;
              items.push(item.data);
          }
      }
      Ok(items)
  }
}