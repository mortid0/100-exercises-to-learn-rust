// TODO: Convert the implementation to use bounded channels.
use crate::data::{Ticket, TicketDraft};
use crate::store::{TicketId, TicketStore};
use core::sync;
use std::io::Error;
use std::sync::mpsc::{Receiver, RecvError, SendError, SyncSender};

pub mod data;
pub mod store;

#[derive(Clone)]
pub struct TicketStoreClient {
    sender: SyncSender<Command>,
}

impl TicketStoreClient {
    pub fn new(sender: SyncSender<Command>) -> Self {
        Self { sender }
    }
    pub fn insert(&self, draft: TicketDraft) -> Result<TicketId, RecvError> {
        let (response_channel, response_receiver) = std::sync::mpsc::sync_channel(1);
        let res = self.sender.send(Command::Insert {
            draft,
            response_channel,
        });
        if let Err(SendError(_)) = res {
            return Err(RecvError);
        }
        response_receiver.recv()
    }

    pub fn get(&self, id: TicketId) -> Result<Option<Ticket>, RecvError> {
        let (response_channel, response_receiver) = std::sync::mpsc::sync_channel(1);
        let res = self.sender.send(Command::Get {
            id,
            response_channel,
        });
        if let Err(SendError(_)) = res {
            return Err(RecvError);
        }
        response_receiver.recv()
    }
}

pub fn launch(capacity: usize) -> TicketStoreClient {
    let (sender, receiver) = std::sync::mpsc::sync_channel(capacity);
    std::thread::spawn(move || server(receiver));
    TicketStoreClient::new(sender)
}

enum Command {
    Insert {
        draft: TicketDraft,
        response_channel: SyncSender<TicketId>,
    },
    Get {
        id: TicketId,
        response_channel: SyncSender<Option<Ticket>>,
    },
}

pub fn server(receiver: Receiver<Command>) {
    let mut store = TicketStore::new();
    loop {
        match receiver.recv() {
            Ok(Command::Insert {
                draft,
                response_channel,
            }) => {
                let id = store.add_ticket(draft);
                response_channel.send(id).unwrap();
            }
            Ok(Command::Get {
                id,
                response_channel,
            }) => {
                let ticket = store.get(id);
                response_channel.send(ticket.cloned()).unwrap();
            }
            Err(_) => {
                // There are no more senders, so we can safely break
                // and shut down the server.
                break;
            }
        }
    }
}
