use futures::{pin_mut, Stream};
use kodec::binary::Codec;
use mezzenger_tcp::Transport;
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    select, spawn,
    sync::{broadcast, RwLock},
};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use zzrpc::{
    producer::{Configuration, Produce},
    Produce,
};

use common::api::{impl_produce, Request, Response}; // or simply: use common::api::*;

#[derive(Debug)]
struct State {
    sender: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    let (sender, _) = broadcast::channel(16);
    let state = Arc::new(RwLock::new(State { sender }));

    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("failed to bind tcp listener to specified address");
    let break_signal = tokio::signal::ctrl_c();
    pin_mut!(break_signal);

    println!("Server started.");

    loop {
        select! {
            listener_result = listener.accept() => {
                let (stream, _address) = listener_result.expect("failed to connect client");
                let state = state.clone();
                spawn(async move {
                    let transport = Transport::new(stream, Codec::default());
                    let producer = Producer { state };
                    producer.produce(transport, Configuration::default());
                });
            },
            break_result = &mut break_signal => {
                break_result.expect("failed to listen for break signal event");
                break;
            }
        }
    }

    println!("\nShutting down...");
}

#[derive(Debug, Produce)]
struct Producer {
    state: Arc<RwLock<State>>,
}

impl Producer {
    /// Print "Hello World!" message on the server.
    async fn hello_world(&self) {
        println!("Hello World!");
    }

    /// Add two integers together and return result.
    async fn add_numbers(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    /// Concatenate two strings and return resulting string.
    async fn concatenate_strings(&self, a: String, b: String) -> String {
        format!("{a}{b}")
    }

    /// Send (string) message to server.
    async fn message(&self, message: String) {
        println!("Message received: {message}");
        let _ = self.state.read().await.sender.send(message);
    }

    /// Stream of messages.
    async fn messages(&self) -> impl Stream<Item = String> {
        BroadcastStream::new(self.state.read().await.sender.subscribe()).filter_map(Result::ok)
    }
}
