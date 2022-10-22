use crate::graph::{Node, Graph};
use crate::buffer::Buffer;

use std::ops;

use bitflags::bitflags;

pub struct Commands {

}

bitflags! {
    pub struct Access: u32 {
        const READ = 0x00000001;
        const WRITE = 0x00000002;
    }
}

pub trait Accessable {
}

impl Accessable for () { }

pub enum Usage<'a, T: Accessable> {
    Ref(&'a T),
    Mut(&'a mut T)
}

pub struct Task1<'a, T: Accessable> {
    data: (Usage<'a, T>),
    call: &'a mut dyn ops::FnOnce(Commands, &mut T),
}

impl<'a, T: Accessable> ops::FnOnce(Commands) for Task1<'a, T> {
    type Output = ();
    fn call_once(self) {

    }
}

impl<A> Node for Task<A> {
    fn execute(mut self) {
        self.task(self.cmds, self.usages.0.0);
    }
}

impl<A, B> Node for Task<A, B> {
    fn execute(mut self) {
        self.task(
            self.cmds, 
            self.usages.0.0,
            self.usages.1.0
        );
    }
}

impl<A, B, C> Node for Task<A, B, C> {
    fn execute(mut self) {
        self.task(self.cmds, self.usages.0.0, self.usages.1.0, self.usages.2.0);
    }
}

impl<A, B, C, D> Node for Task<A, B, C, D> {
    fn execute(mut self) {
        self.task(self.cmds, self.usages.0.0, self.usages.1.0, self.usages.2.0, self.usages.3.0);
    }
}

pub fn non_optimizer(graph: &mut Graph<'_>) { }
