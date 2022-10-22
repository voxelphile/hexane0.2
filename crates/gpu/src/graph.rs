use std::ops;

pub trait Node {
    fn execute(&mut self); 
}

pub enum Dependency {
    Implicit,
    Explicit(usize, usize),
}

pub struct Graph<'a> {
    optimizer: &'a dyn ops::Fn(&mut Graph<'a>),
    executor: Option<Executor>,
    nodes: Vec<Box<dyn Node + 'a>>,
    dependencies: Vec<Dependency>,
}

impl<'a> Graph<'a> {
    fn new(optimizer: &'a dyn ops::Fn(&mut Graph<'a>)) -> Self {
        Self {
            optimizer,
            executor: None,
            nodes: vec![],
            dependencies: vec![],
        }
    }

    fn add(&mut self, node: impl Node + 'a) {
        let _ = self.executor.take();
        self.nodes.push(box node);
        (self.optimizer)(self);
    }

    fn execute(&mut self) {
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
