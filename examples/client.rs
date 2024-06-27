use simple_tcp::Client;

fn main() {
    let mut client = Client::new("127.0.0.1:42069".to_string());

    println!("Before connecting");
    client
        .connect_to_server()
        .expect("Could not connect to server");
}
