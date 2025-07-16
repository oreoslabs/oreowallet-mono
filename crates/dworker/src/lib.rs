use std::collections::HashSet;
use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::anyhow;
use futures::{SinkExt, StreamExt};
use ironfish_rust::{IncomingViewKey, MerkleNote, OutgoingViewKey};
use networking::socket_message::codec::{
    DMessage, DMessageCodec, DRequest, DResponse, RegisterWorker, SingleRequest,
};
use rayon::prelude::*;
use rayon::{iter::IntoParallelIterator, ThreadPool};
use tokio::time::timeout;
use tokio::{
    io::split,
    net::TcpStream,
    sync::{mpsc, oneshot},
    time::sleep,
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{debug, error};

pub async fn decrypt(worker_pool: Arc<ThreadPool>, request: DRequest) -> DResponse {
    let DRequest {
        id,
        address,
        incoming_view_key,
        outgoing_view_key,
        decrypt_for_spender,
        data,
    } = request;
    let mut target_hash = vec![];
    let in_vk = IncomingViewKey::from_hex(&incoming_view_key);
    let out_vk = OutgoingViewKey::from_hex(&outgoing_view_key);
    if in_vk.is_err() || out_vk.is_err() {
        return DResponse {
            id,
            data: target_hash,
            address,
        };
    }
    let in_vk = in_vk.unwrap();
    let out_vk = out_vk.unwrap();
    target_hash = worker_pool.install(move || {
        let decrypted: HashSet<Option<String>> = data
            .into_par_iter()
            .map(|data| {
                let SingleRequest {
                    serialized_note,
                    tx_hash,
                } = data;
                for note in serialized_note {
                    let raw = hex::decode(note);
                    match raw {
                        Ok(raw) => {
                            let note_enc = MerkleNote::read(&raw[..]);
                            if let Ok(note_enc) = note_enc {
                                if let Ok(received_note) = note_enc.decrypt_note_for_owner(&in_vk) {
                                    if received_note.value() != 0 {
                                        return Some(tx_hash);
                                    }
                                }

                                if decrypt_for_spender {
                                    if let Ok(spend_note) =
                                        note_enc.decrypt_note_for_spender(&out_vk)
                                    {
                                        if spend_note.value() != 0 {
                                            return Some(tx_hash);
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
                return None;
            })
            .collect();
        decrypted.into_iter().flatten().collect()
    });
    DResponse {
        id,
        data: target_hash,
        address,
    }
}

pub async fn handle_connection(
    worker_pool: Arc<ThreadPool>,
    stream: TcpStream,
    worker_name: String,
) -> anyhow::Result<()> {
    debug!("connected to scheduler");
    let (r, w) = split(stream);
    let mut socket_w_handler = FramedWrite::new(w, DMessageCodec::default());
    let mut socket_r_handler = FramedRead::new(r, DMessageCodec::default());
    let (tx, mut rx) = mpsc::channel::<DMessage>(1024);

    // send to scheduler loop
    let (router, handler) = oneshot::channel();
    let write_channel_handler = tokio::spawn(async move {
        let _ = router.send(());
        while let Some(message) = rx.recv().await {
            debug!("write message to scheduler {:?}", message);
            match message {
                DMessage::DResponse(response) => {
                    if let Err(e) = socket_w_handler.send(DMessage::DResponse(response)).await {
                        error!("failed to send DResponse message, {:?}", e);
                        return Err(anyhow!("failed to send DResponse message"));
                    }
                }
                DMessage::RegisterWorker(register) => {
                    if let Err(e) = socket_w_handler
                        .send(DMessage::RegisterWorker(register))
                        .await
                    {
                        error!("failed to send RegisterWorker message, {:?}", e);
                        return Err(anyhow!("failed to send RegisterWorker message"));
                    }
                }
                _ => {
                    error!("invalid message to send");
                    return Err(anyhow!("invalid message to send"));
                }
            }
        }
        Ok(())
    });
    let _ = handler.await;

    // receive task handler loop
    let task_tx = tx.clone();
    let (router, handler) = oneshot::channel();
    let read_channel_handler = tokio::spawn(async move {
        let _ = router.send(());
        while let Some(Ok(message)) = socket_r_handler.next().await {
            match message {
                DMessage::DRequest(request) => {
                    debug!("new task from scheduler: {}", request.id.clone());
                    let response = decrypt(worker_pool.clone(), request).await;
                    if let Err(e) = task_tx.send(DMessage::DResponse(response)).await {
                        error!("failed to send response to write channel, {}", e);
                        return Err(anyhow!("failed to send response to write channel"));
                    }
                }
                _ => {
                    error!("invalid message");
                    return Err(anyhow!("invalid message"));
                }
            }
        }
        Ok(())
    });
    let _ = handler.await;

    let heart_beat_tx = tx.clone();
    let (router, handler) = oneshot::channel();
    tokio::spawn(async move {
        let _ = router.send(());
        loop {
            let _ = heart_beat_tx
                .send(DMessage::RegisterWorker(RegisterWorker {
                    name: worker_name.clone(),
                }))
                .await
                .unwrap();
            sleep(Duration::from_secs(30)).await;
        }
    });
    let _ = handler.await;

    loop {
        tokio::select! {
            _ = read_channel_handler => {
                error!("read channel handler exited");
                return Err(anyhow!("read channel handler exited"));
            }
            _ = write_channel_handler => {
                error!("write channel handler exited");
                return Err(anyhow!("write channel handler exited"));
            }
        }
    }
}

pub async fn reconnect_tcp(addr: SocketAddr) -> anyhow::Result<TcpStream> {
    match timeout(Duration::from_secs(3), TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => Ok(stream),
        _ => Err(anyhow!("failed to connect to dservice")),
    }
}

pub async fn start_worker(addr: SocketAddr, name: String) -> anyhow::Result<()> {
    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .build()
        .unwrap();
    let worker = Arc::new(thread_pool);
    let (router, handler) = oneshot::channel();
    tokio::spawn(async move {
        let _ = router.send(());
        loop {
            let stream = loop {
                if let Ok(stream) = reconnect_tcp(addr.clone()).await {
                    break stream;
                }
                error!("can't connect to dservice, reconnect after 30 seconds");
                sleep(Duration::from_secs(30)).await;
            };

            if handle_connection(worker.clone(), stream, name.clone())
                .await
                .is_err()
            {
                error!("failed to init networking");
                break;
            }
        }
    });
    let _ = handler.await;
    std::future::pending::<()>().await;
    Ok(())
}
