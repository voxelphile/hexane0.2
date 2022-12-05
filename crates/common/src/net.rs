use std::net;
use std::result;
use std::mem;
use std::slice;

#[derive(Debug)]
pub enum Error {
    FailedToBind,
    FailedToConnect,
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
    HelloWorld,
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
    Server
}

pub struct Host {  
        listener: Option<net::TcpListener>,
        udp: net::UdpSocket,
        connections: Vec<Connection>,
        backlog: Vec<Message>,
        mode: Mode,
        first_packet_sent: bool,
}

struct Connection {
        peer: Peer,
        token: Token,
        addr: net::SocketAddr,
        tcp: net::TcpStream,
}

impl Host {
    pub fn bind<A: net::ToSocketAddrs + Copy>(addr: A) -> Result<Self> {
        let listener = net::TcpListener::bind(addr)
            .map_err(|_| Error::FailedToBind)?;
        
        listener.set_nonblocking(true).map_err(|_| Error::FailedToSetNonBlocking)?;

        let udp = net::UdpSocket::bind(listener.local_addr().unwrap())
            .map_err(|_| Error::FailedToBind)?;

        udp.set_nonblocking(true).map_err(|_| Error::FailedToSetNonBlocking)?;

        Ok(Host { 
            listener: Some(listener),
            udp,
            first_packet_sent: false,
            mode: Mode::Server,
            connections: vec![],
            backlog: vec![],
        })
    }
    
    pub fn connect<A: net::ToSocketAddrs + Copy>(addr: A) -> Result<Self> {
        let tcp = net::TcpStream::connect(addr)
            .map_err(|_| Error::FailedToConnect)?;

        tcp.set_nonblocking(true).map_err(|_| Error::FailedToSetNonBlocking)?;

        let udp = net::UdpSocket::bind(tcp.local_addr().unwrap())
            .map_err(|_| Error::FailedToBind)?;
        
        udp.set_nonblocking(true).map_err(|_| Error::FailedToSetNonBlocking)?;

        let connection = Connection {
            peer: 0 as Peer,
            token: Token::Absent,
            addr: tcp.peer_addr().unwrap(),
            tcp,
        };

        Ok(Host {
            listener: None,
            udp,
            first_packet_sent: false,
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
                };

                self.connections.push(connection);

                self.send(peer as _, Delivery::Reliable, Message::None);

                Some(peer)
            },
            Err(_) => None
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
                    },
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
        let connection = self.connections.get_mut(peer as usize).ok_or(Error::PeerNotFound)?;

        let packet = Packet {
            peer,
            token: connection.token,
            msg,
        };

        let data = unsafe { slice::from_raw_parts(&packet as *const _ as *const u8, mem::size_of::<Packet>()) };

        match delivery {
            Delivery::Reliable => {
                use std::io::Write;

                connection.tcp.write(data);             
                connection.tcp.write('\0');
                connection.tcp.flush();
            },
            Delivery::Efficient => {
                self.udp.send_to(data, connection.addr);
            },
        }

        Ok(())
    }

    pub fn recv(&mut self) -> Vec<Message> {
        let mut msgs = vec![];
       
        'tcp: for (i, connection) in self.connections.iter_mut().enumerate() {
            loop {
                let mut content = vec![];
                let mut buf = [0; mem::size_of::<Packet>() + 1];
                let mut cursor = 0;

                use std::io::Read;

                while let Ok(l) = connection.tcp.read(&mut buf) {
                    if l == 0 {
                            continue 'tcp;
                    }

                    cursor += l; 

                    if buf[buf.len() - 1] == '\0' {

                    }
                    
                }

                //process TCP first
                //this also ensures we dont read from dead connections
                    match connection.tcp.read(&mut buf) {
                    Ok(l) => {
                        //if read length is 0 
                        if l == 0 {
                            println!("1");
                        }

                        //discard the packet if its not the right size
                        if l < mem::size_of::<Packet>() {
                            println!("2");
                            continue 'tcp;           
                        }

                        //THIS IS UNSAFE AND WILL CRASH THE SERVER IF PEOPLE EXPLOIT THIS
                        //TODO: replace with serialization
                        let packet = unsafe { slice::from_raw_parts(&buf as *const _ as *const Packet, 1)[0] };

                        let special_auth_token = !self.first_packet_sent && self.mode == Mode::Client;

                        if special_auth_token {
                            //we want to use the special auth token to get the same
                            //token as the server has on record.
                            println!("set special auth token");
                            connection.token = packet.token;
                            continue;
                        }

                        self.first_packet_sent = true;

                        if connection.token != packet.token {
                            continue;
                        }

                        if packet.msg == Message::None {
                            println!("4");
                            continue;
                        }

                        msgs.push(packet.msg);
                    },
                    Err(e) => {
                        //discard this attempt to read if there was an error
                        //this is 99% of the time due to lack of info to read
                        continue 'tcp;
                    }
                }
            }
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
                    let packet = unsafe { slice::from_raw_parts(&buf as *const _ as *const Packet, 1)[0] };
                    
                    let Some(connection) = self.connections.get(packet.peer as usize) else {
                        continue;  
                    };

                    if connection.token != packet.token {
                        continue;
                    }

                    if packet.msg == Message::None {
                        continue;
                    }

                    msgs.push(packet.msg);
                },
                Err(_) => {
                    break;
                }
            }
        } 
        
        self.backlog.extend(msgs);
        
        mem::replace(&mut self.backlog, vec![])
    }
}
