use std::ops;
use crate::task::Commands;

pub enum Dependency {
    Implicit,
    Explicit(usize, usize),
}

pub struct Graph<'a> {
    optimizer: &'a dyn ops::Fn(&mut Graph<'a>),
    executor: Option<Executor>,
    tasks: Vec<Box<dyn ops::FnOnce(&'a mut Commands) + 'a>>,
    dependencies: Vec<Dependency>,
}

impl<'a> Graph<'a> {
    pub fn new(optimizer: &'a dyn ops::Fn(&mut Graph<'a>)) -> Self {
        Self {
            optimizer,
            executor: None,
            tasks: vec![],
            dependencies: vec![],
        }
    }

    pub fn add(&mut self, task: impl ops::FnOnce(&'a mut Commands) + 'a) {
        let _ = self.executor.take();
        self.tasks.push(box task);
        (self.optimizer)(self);
    }

    pub fn execute(&mut self) {
        if let None = self.executor { 
            self.executor = Some(Executor::new(&self));
        }

        self.executor.as_mut().unwrap().execute();
    }
}

pub struct Executor;

impl Executor {
    fn new(graph: &'_ Graph<'_>) -> Self {
        todo!()
    }

    fn execute(&mut self) {
        todo!()
    }
}
