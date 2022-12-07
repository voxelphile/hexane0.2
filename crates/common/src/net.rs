use std::mem;
use std::net;
use std::io;
use std::time;
use std::result;
use std::slice;

#[derive(Debug)]
pub enum Error {
    FailedToBind,
    FailedToConnect,
    FailedToSend,
    FailedToSetNonBlocking,
    PeerNotFound,
}

type Result<T> = result::Result<T, Error>;

type Peer = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Packet {
    peer: Peer,
    token: Token,
    msg: Message,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    None,
    HelloWorld(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    Reliable,
    Efficient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token {
    Absent,
    Valid(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Client,
    Server,
}

pub struct Host {
    listener: Option<net::TcpListener>,
    udp: net::UdpSocket,
    connections: Vec<Connection>,
    backlog: Vec<Message>,
    mode: Mode,
    first_packet_recv: bool,
}

struct Connection {
    peer: Peer,
    token: Token,
    addr: net::SocketAddr,
    tcp: net::TcpStream,
    content: Vec<u8>,
}

impl Host {
    pub fn bind<A: net::ToSocketAddrs + Copy>(addr: A) -> Result<Self> {
        let listener = net::TcpListener::bind(addr).map_err(|_| Error::FailedToBind)?;

        listener
            .set_nonblocking(true)
            .map_err(|_| Error::FailedToSetNonBlocking)?;

        let udp = net::UdpSocket::bind(listener.local_addr().unwrap())
            .map_err(|_| Error::FailedToBind)?;

        udp.set_nonblocking(true)
            .map_err(|_| Error::FailedToSetNonBlocking)?;

        Ok(Host {
            listener: Some(listener),
            udp,
            first_packet_recv: false,
            mode: Mode::Server,
            connections: vec![],
            backlog: vec![],
        })
    }

    pub fn connect<A: net::ToSocketAddrs + Copy>(addr: A) -> Result<Self> {
        let tcp = net::TcpStream::connect(addr).map_err(|_| Error::FailedToConnect)?;

        tcp.set_nonblocking(true)
            .map_err(|_| Error::FailedToSetNonBlocking)?;
        tcp.set_read_timeout(Some(time::Duration::new(60*20, 0)));
        tcp.set_write_timeout(Some(time::Duration::new(60*20, 0)));


        let udp =
            net::UdpSocket::bind(tcp.local_addr().unwrap()).map_err(|_| Error::FailedToBind)?;

        udp.set_nonblocking(true)
            .map_err(|_| Error::FailedToSetNonBlocking)?;

        let connection = Connection {
            peer: 0 as Peer,
            token: Token::Absent,
            addr: tcp.peer_addr().unwrap(),
            tcp,
            content: vec![],
        };

        Ok(Host {
            listener: None,
            udp,
            first_packet_recv: false,
            mode: Mode::Client,
            connections: vec![connection],
            backlog: vec![],
        })
    }

    pub fn accept(&mut self) -> Option<Peer> {
        let Some(listener) = &self.listener else {
            None?
        };

        match listener.accept() {
            Ok((tcp, addr)) => {
                tcp.set_nonblocking(true).ok()?;
                tcp.set_read_timeout(Some(time::Duration::new(60*20, 0)));
                tcp.set_write_timeout(Some(time::Duration::new(60*20, 0)));

                use rand::Rng;

                let token = loop {
                    let token = Token::Valid(rand::thread_rng().gen());

                    let mut conflict = false;

                    for connection in &self.connections {
                        if connection.token == token {
                            conflict = true;
                            break;
                        }
                    }

                    if !conflict {
                        break token;
                    }
                };

                let peer = self.connections.len() as Peer;

                let connection = Connection {
                    peer,
                    token,
                    addr,
                    tcp,
                    content: vec![],
                };

                self.connections.push(connection);
                
                let Ok(_) = self.send(peer as _, Delivery::Reliable, Message::None) else {
                    self.connections[peer as usize].tcp.shutdown(net::Shutdown::Both);
                    self.connections.pop();
                    None?
                };

                Some(peer)
            }
            Err(_) => None,
        }
    }

    pub fn drop(&mut self) -> Option<Peer> {
        let mut drop_peer = None;

        'tcp: for (i, connection) in self.connections.iter_mut().enumerate() {
            loop {
                let mut buf = [0; mem::size_of::<Packet>()];

                use std::io::Read;

                //process TCP first
                //this also ensures we dont read from dead connections
                match connection.tcp.read(&mut buf) {
                    Ok(l) => {
                        //if read length is 0
                        if l == 0 {
                            drop_peer = Some(i as Peer);
                            break;
                        }
                    }
                    Err(_) => {
                        continue 'tcp;
                    }
                }
            }
        }

        if let Some(peer) = drop_peer {
            self.connections.remove(peer as usize);
        }

        drop_peer
    }

    pub fn send(&mut self, peer: Peer, delivery: Delivery, msg: Message) -> Result<()> {
        let connection = self
            .connections
            .get_mut(peer as usize)
            .ok_or(Error::PeerNotFound)?;

        let packet = Packet {
            peer,
            token: connection.token,
            msg,
        };

        let data = unsafe {
            slice::from_raw_parts(&packet as *const _ as *const u8, mem::size_of::<Packet>())
        };

        match delivery {
            Delivery::Reliable => {
                use std::io::Write;

                connection.tcp.write_all(data).map_err(|_| Error::FailedToSend)?;
                connection.tcp.flush().map_err(|_| Error::FailedToSend)?;
            }
            Delivery::Efficient => {
                self.udp.send_to(data, connection.addr);
            }
        }

        Ok(())
    }

    pub fn recv(&mut self) -> Vec<Message> {
        let mut msgs = vec![];

        'tcp: for (i, connection) in self.connections.iter_mut().enumerate() {

            const MESSAGE_SIZE: usize = mem::size_of::<Packet>();
            let mut buffer = [0; MESSAGE_SIZE];

            use std::io::Read;

            loop {
                let bytes_read = match connection.tcp.read(&mut buffer[..MESSAGE_SIZE - connection.content.len()]) {
                        Ok(bytes_read) => bytes_read,
                        Err(e) => {
                            if e.kind() == io::ErrorKind::WouldBlock {
                                continue 'tcp;
                            }
                            dbg!(e);
                            panic!("whoops");
                        }
                    };

                connection.content.extend_from_slice(&buffer[..bytes_read]);

                dbg!(connection.content.len());
                dbg!(MESSAGE_SIZE);

                if connection.content.len() == MESSAGE_SIZE {
                    break;
                }
            }

            dbg!(connection.tcp.take_error().expect("lol"));

            //THIS IS UNSAFE AND WILL CRASH THE SERVER IF PEOPLE EXPLOIT THIS
            //TODO: replace with serialization
            let packet =
                unsafe { slice::from_raw_parts(connection.content.as_ptr() as *const _ as *const Packet, 1)[0] };

            let special_auth_token = !self.first_packet_recv && self.mode == Mode::Client;

            if special_auth_token {
                //we want to use the special auth token to get the same
                //token as the server has on record.
                println!("set special auth token");
                connection.token = packet.token;
                self.first_packet_recv = true;
                continue 'tcp;
            }


            if connection.token != packet.token {
                println!("3");
                continue 'tcp;
            }

            if packet.msg == Message::None {
                println!("4");
                continue 'tcp;
            }

            msgs.push(packet.msg);

            connection.content.clear();
        }

        //there is no outer loop because udp is connection independent
        //we detect and process the same way as tcp though
        'udp: loop {
            let mut buf = [0; mem::size_of::<Packet>()];

            match self.udp.recv(&mut buf) {
                Ok(l) => {
                    if l == 0 {
                        break;
                    }

                    if l < mem::size_of::<Packet>() {
                        continue;
                    }

                    //THIS IS UNSAFE AND WILL CRASH THE SERVER IF PEOPLE EXPLOIT THIS
                    let packet =
                        unsafe { slice::from_raw_parts(&buf as *const _ as *const Packet, 1)[0] };

                    let Some(connection) = self.connections.get(packet.peer as usize) else {
                        dbg!("yo");
                        continue;
                    };

                    if connection.token != packet.token {
                        dbg!("1");
                        continue;
                    }

                    if packet.msg == Message::None {
                        dbg!("2");
                        continue;
                    }

                    msgs.push(packet.msg);
                }
                Err(_) => {
                    break;
                }
            }
        }

        self.backlog.extend(msgs);

        mem::replace(&mut self.backlog, vec![])
    }
}
