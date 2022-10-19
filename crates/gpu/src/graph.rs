use std::ops;
    
pub trait Optimizer<T, U> = ops::Fn(&mut Graph<T, U>);   

pub trait Executor<T: ?Sized> {
    fn new(&mut Graph<T>) -> Self;
    fn execute(&mut self);
}

pub trait Node {
    type Executor: Executor<Self>;
    fn execute(mut self);
}

pub enum Dependency {
    Implicit,
    Explicit(usize, usize),
}

pub struct Graph<'a, T: Node, U: Executor<T>> {
    optimizer: &'a dyn Optimizer<T, U>,
    executor: Option<U>,
    nodes: Vec<Box<dyn Node<Executor = U>>>,
    dependencies: Vec<Dependency>,
}

impl<T: Node, U: Executor<T>> Graph<'_, T, U> {
    fn new(optimizer: &dyn Optimizer<T, U>) -> Self {
        Self {
            optimizer,
            executor: None,
            nodes: vec![],
            dependencies: vec![],
        }
    }

    fn add(&mut self, node: impl Node) {
        let _ = self.state.take();
        self.nodes.push(box node);
        self.optimizer.optimize(&mut self);
    }

    fn execute(&mut self) {
        if let None = self.executor { 
            self.executor = Some(U::new(&mut self));
        }

        self.executor.as_mut().unwrap().execute();
    }
}
