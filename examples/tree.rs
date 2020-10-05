use arena_graph::*;
use std::sync::Arc;
use std::cell::Cell;
use std::ops::Deref;
use std::cell::RefCell;

struct TreeNode<T> {
    data: T,
    parent: Cell<NodePtr<Self>>,
    children: RefCell<Vec<NodePtr<Self>>>
}

struct TreeNodeGuard<'gg, T>(NodeGuard<'gg, TreeNode<T>>);

impl <'gg, T> Deref for TreeNodeGuard<'gg, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0.data
    }
}

impl <'gg, T> TreeNodeGuard<'gg, T> {
    fn set_parent(&self, other: &Self) {
    }
}

fn main() {
}
