use typed_arena::Arena;
use std::marker::PhantomData;
use std::ops::Deref;
use std::cell::Cell;

pub struct Graph<N> {
    graph: Arena<N>,
}

pub struct GraphGuard<'gg, N> {
    inside: &'gg Graph<N>,
    invariant: PhantomData<&'gg mut &'gg ()>,
}

pub struct NodeGuard<'gg, N> {
    inside: &'gg N,
    invariant: PhantomData<&'gg mut &'gg ()>,
}

impl <'gg, N> Deref for NodeGuard<'gg, N> {
    type Target = N;
    fn deref(&self) -> &N {
        self.inside
    }
}

impl <N> Graph<N> {
    pub fn new() -> Self {
        unimplemented!()
    }

    pub fn with<F: for<'any> FnOnce(GraphGuard<'any, N>)>(&self, func: F) {
        func(GraphGuard {inside: self, invariant: PhantomData})
    }
}

impl <'gg, N> GraphGuard<'gg, N> {
    pub fn get(&self) -> NodeGuard<'gg, N> {
        unimplemented!()
    }
}

pub struct NodePtr<N>(*const N);

impl <'gg, N> NodeGuard<'gg, N> {
    pub unsafe fn make_ptr(&self, other: &NodeGuard<'gg, N>) -> NodePtr<N> {
        NodePtr(other.inside as *const N)
    }

    pub unsafe fn lookup_ptr(&self, NodePtr(ptr): NodePtr<N>) -> NodeGuard<'gg, N> {
        NodeGuard {
            inside: &*ptr,
            invariant: self.invariant,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let g1 = Graph::<()>::new();
        let g2 = Graph::<()>::new();
        g1.with(|g1| {
            g2.with(|g2| {
                let a = g1.get();
                let b = g1.get();
                let c = g2.get();
                let d = g2.get();
                d.set_parent(&a);
                b.set_parent(&a);
            })
        });
    }
}
