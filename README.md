# zzrpc-tutorial

Tutorial example for [zzrpc](https://github.com/zduny/zzrpc).

# usage

Build:

```bash
cargo build
```

Start server:

```bash
cd server
cargo run
```

Open new terminal tab/window and start client:

```bash
cd client
cargo run
```

# tutorial

## Step 0 (optional) - `common` crate

Create separate `common` crate for definition of your api.

This step is optional, but doing so makes it easier to share code between client and server 
implementations, especially when they're targeting different platforms (for example: server 
targeting native platform and client targeting WebAssembly running in a browser).

```bash
cargo new --lib common
```

## Step 1 - dependencies

Add [`mezzenger`](https://crates.io/crates/mezzenger), [`serde`](https://crates.io/crates/serde) with `derive` feature enabled and finally [`zzrpc`](https://crates.io/crates/zzrpc) to `Cargo.toml`:

```toml
# ...

[dependencies]
# ...
mezzenger = "0.1.2"
serde = { version = "1.0.150", features = ["derive"] } 
zzrpc = "0.1.1"
```

For your server/clients you'll also need some [`kodec`](https://crates.io/crates/kodec), 
some `mezzenger` transport implementation 
(here: [mezzenger-tcp](https://crates.io/crates/mezzenger-tcp)), 
[`futures`](https://crates.io/crates/futures) and on native server/client 
[`tokio`](https://crates.io/crates/tokio):


```toml
# ...

[dependencies]
# ...
mezzenger = "0.1.2"
serde = { version = "1.0.150", features = ["derive"] } 
zzrpc = "0.1.1"
kodec = { version = "0.1.0", features = ["binary"] }
mezzenger-tcp = "0.1.1"
futures = "0.3.25"
tokio = { version = "1.23.0", features = ["full"] } # only when targeting native platforms
tokio-stream = { version = "0.1.11", features = ["sync"] } # optional but useful when creating stream responses 
```

If you followed **Step 0** then add `common` to your client/server crate dependencies:

```toml
# ...

[dependencies]
# ...
common = { path = "../common" }
mezzenger = "0.1.2"
serde = { version = "1.0.150", features = ["derive"] } 
zzrpc = "0.1.1"
kodec = { version = "0.1.0", features = ["binary"] }
mezzenger-tcp = "0.1.1"
futures = "0.3.25"
tokio = { version = "1.23.0", features = ["full"] } # only when targeting native platforms
tokio-stream = { version = "0.1.11", features = ["sync"] } # optional but useful when creating stream responses 
```

## Step 2 - api definition

Define your api:

```rust
use zzrpc::api;

/// Service API.
#[api]
pub trait Api {
    /// Print "Hello World!" message on the server.
    async fn hello_world(&self);

    /// Add two integers together and return result.
    async fn add_numbers(&self, a: i32, b: i32) -> i32;

    /// Concatenate two strings and return resulting string.
    async fn concatenate_strings(&self, a: String, b: String) -> String;

    /// Send (string) message to server.
    async fn message(&self, message: String);

    /// Stream of messages.
    async fn messages(&self) -> impl Stream<Item = String>;
}
```

Notice the `#[api]` macro attribute at the top.

Methods must be marked with `async`.<br>
There are two types of supported methods:
- regular "value" methods like `hello_world`, `add_numbers`, `concatenate_strings`, `message` above,
- streaming methods - where single request instructs producer to send multiple values (not necessarily 
at once) without consumer having to request them individually with separate requests. Return types of streaming request methods must follow form: `impl Stream<Item = [ITEM TYPE]>`.

## Step 3 - producing

Now, let's create a server for defined api.

### Step 3a - server

We'll use TCP for communication, first we have to accept TCP connections.<br>
Also we'll create common state shared state by our producers.

```rust
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    select, spawn,
    sync::RwLock,
};
use futures::{pin_mut, Stream};
use kodec::binary::Codec;
use mezzenger_tcp::Transport;

#[derive(Debug)]
struct State {
    // ... to do in next steps
}

#[tokio::main]
async fn main() {
    let state = Arc::new(RwLock::new(State {}));

    let listener = TcpListener::bind("127.0.0.1").await
        .expect("failed to bind tcp listener to specified address");
    let break_signal = tokio::signal::ctrl_c();
    pin_mut!(break_signal);

    loop {
        select! {
            listener_result = listener.accept() => {
                let (stream, _address) = listener_result.expect("failed to connect client");
                let state = state.clone();
                spawn(async move {
                    // ... to do in next steps
                });
            },
            break_result = &mut break_signal => {
                break_result.expect("failed to listen for break signal event");
                break;
            }
        }
    }
}
```

### Step 3b - producer

Now it's a good time to implement a producer for your api:

```rust
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use zzrpc::Produce;
use common::api::{impl_produce, Request, Response}; // or simply: use common::api::*;

// ...

#[derive(Debug, Produce)]
struct Producer {
    state: Arc<RwLock<State>>,
}

// Note we're not implementing any trait here - zzrpc is designed like that
// on purpose to avoid dealing with current Rust's async trait troubles.
//
// Instead simply copy your method signatures from the api's trait, add method 
// bodies and implement them - `Produce` derive macro will do the rest for you.
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
```

Remember to update `State` struct:

```rust
// ...
#[derive(Debug)]
struct State {
    sender: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    let (sender, _) = broadcast::channel(16);
    let state = Arc::new(RwLock::new(State { sender }));
// ...
```

### Step 3c - serving

Now we can serve our clients:

```rust
// ...
let (stream, _address) = listener_result.expect("failed to connect client");
let state = state.clone();
spawn(async move {
    // create mezzenger transport wrapping a TCP stream.
    let transport = Transport::new(stream, Codec::default());

    // create producer.
    let producer = Producer { state };

    // produce for the transport using default producer configuration.
    producer.produce(transport, Configuration::default());
});
// ...
```

## Step 4 - consuming

Let's write a client for our service:

```rust
use std::time::Duration;

use futures::StreamExt;
use kodec::binary::Codec;
use mezzenger_tcp::Transport;
use tokio::{net::TcpStream, spawn, time::sleep};
use zzrpc::{consumer::Configuration, Consume};

use common::api::{Api, Consumer};

#[tokio::main]
async fn main() {
    // first connect to the server.
    let stream = TcpStream::connect("127.0.0.1:8080")
        .await
        .expect("failed to connect to server");
    let address = stream.local_addr().expect("failed to get address");

    // wrap TCP stream in mezzenger transport.
    let transport = Transport::new(stream, Codec::default());

    // create consumer communicating over transport and using default configuration.
    let consumer = Consumer::consume(transport, Configuration::default());


    // now we can make out first request.
    consumer.hello_world().await.expect("failed to call method");

    // another request, this time with return value.
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

    // let's create a message stream.
    let mut messages = consumer
        .messages()
        .await
        .expect("failed get message stream");

    // send some messages to server.
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

    // loop over and print received messages.
    while let Some(message) = messages.next().await {
        println!("Received message: {message}");
    }
}
```

## Limitations

Have in mind following limitations of `zzrpc`'s `#[api]` macro:
- generic traits are not supported,
- generic methods are not supported,
- only one api is allowed per module 
  (but multiple apis in separate modules of the same crate are fine),
- only one producer implementation is allowed per module
  (again: if you need more you can simply define them in separate modules).
