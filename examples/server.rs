use simple_tcp::server::Server;

fn main() {
    let mut server = Server::new("127.0.0.1:42069".to_string()).expect("Failed at creating server");

    match server.start_server() {
        Err(e) => {
            panic!("Error at starting server {e}")
        }
        _ => {}
    };
}
