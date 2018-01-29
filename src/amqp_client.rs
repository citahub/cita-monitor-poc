use amqp::{protocol, Basic, Channel, Consumer, Session, Table};
use std::sync::mpsc::Sender;
use std::thread;

pub struct Handler {
    tx: Sender<(String, Vec<u8>)>,
}

impl Handler {
    pub fn new(tx: Sender<(String, Vec<u8>)>) -> Self {
        Handler { tx: tx }
    }
}

impl Consumer for Handler {
    fn handle_delivery(
        &mut self,
        _channel: &mut Channel,
        deliver: protocol::basic::Deliver,
        _: protocol::basic::BasicProperties,
        body: Vec<u8>,
    ) {
        let _ = self.tx.send((deliver.routing_key, body));
    }
}

pub fn start_sub(amqp_url: &str, name: &str, keys: Vec<&str>, tx: Sender<(String, Vec<u8>)>) {
    let mut session = match Session::open_url(&amqp_url) {
        Ok(session) => session,
        Err(error) => panic!("failed to open url {} : {:?}", amqp_url, error),
    };

    let mut channel = session.open_channel(1).ok().expect("Can't open channel");
    let _ = channel.basic_prefetch(10);
    channel
        .exchange_declare(
            "cita",
            "topic",
            false,
            true,
            false,
            false,
            false,
            Table::new(),
        )
        .unwrap();

    //queue: &str, passive: bool, durable: bool, exclusive: bool, auto_delete: bool, nowait: bool, arguments: Table
    channel
        .queue_declare(name.clone(), false, true, false, true, false, Table::new())
        .unwrap();

    for key in keys {
        channel
            .queue_bind(name.clone(), "cita", key, false, Table::new())
            .unwrap();
    }
    let callback = Handler::new(tx);
    //queue: &str, consumer_tag: &str, no_local: bool, no_ack: bool, exclusive: bool, nowait: bool, arguments: Table
    channel
        .basic_consume(
            callback,
            name.clone(),
            "",
            false,
            true,
            false,
            false,
            Table::new(),
        )
        .unwrap();

    // thread recv msg from mq
    let _ = thread::Builder::new()
        .name("subscriber".to_string())
        .spawn(move || {
            channel.start_consuming();
            let _ = channel.close(200, "Bye");
        });
}
