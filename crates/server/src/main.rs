fn main() {
    println!("Hello, server!");
    use common::net::*;

    let mut host = Host::bind("0.0.0.0:29757").unwrap();

    let mut peers = vec![];

    let mut instant = std::time::Instant::now();

    loop {
        if let Some(peer) = host.accept() {
            peers.push(peer);
            println!("client {} joined", peer);
        }

        if let Some(peer) = host.drop() {
            peers.push(peer);
            println!("client {} dropped", peer);
            break;
        }

        if std::time::Instant::now().duration_since(instant).as_secs_f32() > 1.0 {
            for &peer in &peers {
                host.send(peer, Delivery::Reliable, Message::HelloWorld(420)).unwrap();
                dbg!("sent");
            }
            instant = std::time::Instant::now();
        }

        let msgs = host.recv();

        if msgs.len() > 0 {
        dbg!(msgs);
        }
    }
}
