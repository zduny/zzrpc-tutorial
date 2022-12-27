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
