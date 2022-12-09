fn main() {
    println!("Hello, server!");

    use net::*;

    let mut socket = Socket::open(SocketType::Datagram).unwrap();

    socket.bind("0.0.0.0:29753").unwrap();

    loop {}
}
