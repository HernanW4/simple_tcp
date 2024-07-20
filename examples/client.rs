use simple_tcp::client::Client;

fn main() {
    env_logger::init();
    let mut client = Client::new("127.0.0.1:42069".to_string()).unwrap();

    client.run().expect("Could not connect to server");
}
