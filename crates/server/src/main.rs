fn main() {
    println!("Hello, server!");
    use common::net::*;

    let mut host = Host::bind("127.0.0.1:29757").unwrap();

    loop {
        if let Some(peer) = host.accept() {
            println!("client {} joined", peer);
        }

        if let Some(peer) = host.drop() {
            println!("client {} dropped", peer);
            break;
        }
        
        if let Some(msg) = host.recv().pop() {
            dbg!(msg);
        }
    };
    
}

