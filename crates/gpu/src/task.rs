use std::ops;

pub struct Commands {

}

bitflags! {
    pub struct Access: u32 {
        const READ = 0x00000001;
        const WRITE = 0x00000002;
    }
}


pub enum Usage<T> {
    Image(&mut T, Access),
    Buffer(&mut T, Access),
}

macro_rules! task {
    () => {
        pub struct Task {
            usages: (),
            task: ops::FnOnce(Commands),
        }
    }
    ($x: ident $(, $y: ident)*) => {
        pub struct Task<$x $(, $y)*> {
            usages: (Usage<$x>$(, Usage<$y>)*),
            task: ops::FnOnce(Commands, &mut x $(, &mut $y)*),
        }
        task!($y)
    }
}

task!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
       
impl Node for Task {
    type Executor = TaskExecutor;
    fn execute(mut self) {
        self.task(self.cmds);
    }
}

impl Node for Task<A> {
    type Executor = Task::Executor;
    fn execute(mut self) {
        self.task(self.cmds, self.usages.0.0);
    }
}

impl Node for Task<A, B> {
    type Executor = Task::Executor;
    fn execute(mut self) {
        self.task(
            self.cmds, 
            self.usages.0.0,
            self.usages.1.0
        );
    }
}

impl Node for Task<A, B, C> {
    type Executor = Task::Executor;
    fn execute(mut self) {
        self.task(self.cmds, self.usages.0.0, self.usages.1.0, self.usages.2.0);
    }
}

impl Node for Task<A, B, C, D> {
    type Executor = Task::Executor;
    fn execute(mut self) {
        self.task(self.cmds, self.usages.0.0, self.usages.1.0, self.usages.2.0, self.usages.3.0);
    }
}

pub fn non_optimizer(graph: &mut Graph<Task, TaskExecutor>) { }

pub struct TaskExecutor {

}
