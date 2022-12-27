use std::time::Duration;

use futures::StreamExt;
use kodec::binary::Codec;
use mezzenger_tcp::Transport;
use tokio::{net::TcpStream, spawn, time::sleep};
use zzrpc::{consumer::Configuration, Consume};

use common::api::{Api, Consumer};

#[tokio::main]
async fn main() {
    let stream = TcpStream::connect("127.0.0.1:8080")
        .await
        .expect("failed to connect to server");
    let address = stream.local_addr().expect("failed to get address");
    let transport = Transport::new(stream, Codec::default());

    let consumer = Consumer::consume(transport, Configuration::default());

    consumer.hello_world().await.expect("failed to call method");

    let result = consumer
        .add_numbers(2, 3)
        .await
        .expect("failed to call method");
    println!("2 + 3 = {result}");

    let result = consumer
        .concatenate_strings("Hello ".to_string(), "World".to_string())
        .await
        .expect("failed to call method");
    println!("'Hello ' + 'World' = '{result}'");

    let mut messages = consumer
        .messages()
        .await
        .expect("failed get message stream");

    consumer
        .message(format!("{address} - Message 1"))
        .await
        .expect("failed to call method");

    consumer
        .message(format!("{address} - Message 2"))
        .await
        .expect("failed to call method");

    consumer
        .message(format!("{address} - Message 3"))
        .await
        .expect("failed to call method");

    let aborter = messages.aborter();
    spawn(async move {
        // abort stream after 30 seconds
        sleep(Duration::from_secs(30)).await;
        aborter.abort();
    });

    while let Some(message) = messages.next().await {
        println!("Received message: {message}");
    }
}
