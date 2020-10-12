use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use typed_arena::Arena;

pub struct Graph<N> {
    graph: Arena<N>,
}

pub struct GraphGuard<'gg, N> {
    inside: &'gg Graph<N>,
    invariant: PhantomData<&'gg mut &'gg ()>,
}

impl<'gg, N> GraphGuard<'gg, N> {
    pub fn insert(&self, node: N) -> NodeGuard<'gg, N> {
        let node_ref = self.inside.graph.alloc(node);
        NodeGuard {
            inside: node_ref,
            invariant: self.invariant,
        }
    }

    pub unsafe fn lookup_ptr(&self, NodePtr(ptr): NodePtr<N>) -> NodeGuard<'gg, N> {
        NodeGuard {
            inside: &*ptr.as_ptr(),
            invariant: self.invariant,
        }
    }
}

pub struct NodeGuard<'gg, N> {
    inside: &'gg N,
    invariant: PhantomData<&'gg mut &'gg ()>,
}

impl<N> Clone for NodeGuard<'_, N> {
    fn clone(&self) -> Self {
        Self {
            inside: self.inside,
            invariant: self.invariant,
        }
    }
}

impl<N> Copy for NodeGuard<'_, N> {}

impl<'gg, N> Deref for NodeGuard<'gg, N> {
    type Target = N;
    fn deref(&self) -> &N {
        self.inside
    }
}

impl<N> Graph<N> {
    pub fn new() -> Self {
        Graph {
            graph: Arena::new(),
        }
    }

    pub fn with<F: for<'any> FnOnce(GraphGuard<'any, N>) -> R, R>(&self, func: F) -> R {
        func(GraphGuard {
            inside: self,
            invariant: PhantomData,
        })
    }
}

pub struct NodePtr<N>(NonNull<N>);

impl<N> Clone for NodePtr<N> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}
impl<N> Copy for NodePtr<N> {}

use std::fmt;
impl<N> fmt::Debug for NodePtr<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodePtr").finish()
    }
}
impl<N> fmt::Debug for NodeGuard<'_, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeGuard").finish()
    }
}

impl<N> NodePtr<N> {
    pub fn ptr_eq(self, other: Self) -> bool {
        std::ptr::eq(self.0.as_ptr(), other.0.as_ptr())
    }

    pub unsafe fn lookup_unchecked<'gg>(&self) -> NodeGuard<'gg, N> {
        NodeGuard {
            inside: &*self.0.as_ptr(),
            invariant: PhantomData,
        }
    }
}

use std::hash::{Hash, Hasher};
impl<N> Hash for NodePtr<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

use std::cmp::Ordering;
impl<N> PartialOrd for NodePtr<N> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<N> Ord for NodePtr<N> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<N> PartialEq for NodePtr<N> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<N> Eq for NodePtr<N> {}

impl<'gg, N> NodeGuard<'gg, N> {
    pub unsafe fn make_ptr(&self) -> NodePtr<N> {
        // ideally would not have to cast to *mut N, but will need to until we get NonNullConst
        NodePtr(NonNull::new_unchecked(self.inside as *const N as *mut N))
    }

    pub unsafe fn lookup_ptr(&self, NodePtr(ptr): NodePtr<N>) -> NodeGuard<'gg, N> {
        NodeGuard {
            inside: &*ptr.as_ptr(),
            invariant: self.invariant,
        }
    }

    pub fn node(&self) -> &'gg N {
        self.inside
    }
}

impl<N> PartialEq for NodeGuard<'_, N> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.inside, other.inside)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {}
}
