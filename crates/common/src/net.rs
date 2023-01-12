use serde::{Serialize, Deserialize};
use math::{Vector};

const ACK_COUNT: usize = 64;

#[derive(Serialize, Deserialize)]
pub struct Packet {
    ack: [u64; ACK_COUNT],
    message: Message,
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    None,
    Spawn {
        id: usize,
        position: Vector<f32, 3>,
    },
    Move {
        id: usize,
        position: Vector<f32, 3>,
    }
}

