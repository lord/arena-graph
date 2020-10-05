# arena-graph

A library for constructing fast, pointer-based graphs in Rust. A lil hacky, and my understanding of variance is dubious at best, so consult your local lifetime specialist before using. Useful if your graph has the following properties:

- the node lookup overhead of a slotmap-style graph is too slow for your the problem you're trying to solve
- nodes in your graph all have the same type
- you only need `&` access to nodes once they've been added, and can use mutexes or cells to handle mutation
- your graph doesn't need to delete individual nodes, only an entire graph of nodes all at once (although having a `Cell<bool>` that indicates deleted status is fine, as is creating a linked list of deleted nodes for recycling)
- your Node's Drop implementation doesn't need to access other nodes

arena-graph is based on my vague understanding that it's perfectly safe to convert `*const Node` to `&Node`, so long as the resource targeted by that raw pointer still exists and has not been moved. Arena allocation gives us both of these properties! We only store our `*const Node` in places that will be dropped at the same time as the arena — either inside a node, or alongside the arena in the same struct. This guarantees the raw pointers won't outlive the arena they point into.

However, there's still one problem. Let's say we have a set_parent method like this:

```rs
impl TreeNode {
  fn set_parent(&self, new_parent: &TreeNode) {
    self.parent.set(new_parent as *const TreeNode);
  }
}
```

In this example, there's no guarantee that `new_parent` is a TreeNode in the same graph as `self`. If these two nodes are part of different arenas, `new_parent` could be deallocated at a different time from `self`, `self` later goes to dereference its parent, and we've hit undefined behavior.

To avoid this, we need to ensure `new_parent` and `self` are part of the same tree. One way to do this is to have `TreeNode` have some sort of unique arena ID, and we could compare these IDs any time we add an edge from one graph node to another. This check is frustrating, though. If we're already going to all this trouble of avoiding slotmap-style checks, ideally we wouldn't have any checks at all, even when adding new edges. Another solution is we could just have the user promise they won't do this, but marking `set_parent` as `unsafe` will clutter up our users' code with countless unsafe blocks.

What if instead, we could have the Rust compiler statically check that `self` and `new_parent` come from the same graph? The bet of this library is that you maybe can hack this in with Rust's lifetime system, if you can guarantee that:

- When adding an edge from `&'a TreeNode` and `&'b TreeNode`, we ensure that `'a` and `'b` have exactly the same lifetime.
- If `&'a TreeNode` and `&'b TreeNode` came from different graphs, `'a` and `'b` will be different lifetimes.

To get these properties, we have to talk briefly about variance.

## A brief aside about variance

The following code [compiles](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=22fa15a461255ce17b8acd61e0fef04f):

```rs
#[derive(Debug)]
struct NoClone(i32);

fn main() {
    let num_1 = NoClone(1);
    let num_1_ref = &num_1;
    let num_2 = NoClone(2);
    let num_2_ref = &num_2;
    print_numbers(num_1_ref, num_2_ref);
    std::mem::drop(num_2);
    println!("{:?}", num_1_ref);
}

fn print_numbers<'a>(num_1: &'a NoClone, num_2: &'a NoClone) {
    println!("{:?} {:?}", num_1, num_2);
}
```

At first, this may seem weird. `print_numbers` is asking for two numbers with the same lifetime, but the two numbers have different lifetimes — `num_2` is dropped in `main` before we print `num_1_ref`.

The answer is [variance](https://doc.rust-lang.org/nomicon/subtyping.html). `&'a NoClone` is covariant for `'a`, which has a complex type theory meaning but for our purposes means you can replace `'a` with any lifetime longer than `'a`. The two arguments passed into `print_numbers` can have two different lifetimes, so long as both lifetimes last at least as long as the call to `print_numbers`.

However, the following [doesn't compile](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=8fc847fbfef646c11493fd7a81b592dd):

```rs
#[derive(Debug)]
struct NoClone(i32);

fn main() {
    let num_1 = NoClone(1);
    let mut num_1_ref = &num_1;
    let num_2 = NoClone(2);
    let mut num_2_ref = &num_2;
    print_numbers(&mut num_1_ref, &mut num_2_ref);
    std::mem::drop(num_2); // delete this line to make it compile
    println!("{:?}", num_1_ref);
}

fn print_numbers<'a>(num_1: &mut &'a NoClone, num_2: &mut &'a NoClone) {
    println!("{:?} {:?}", num_1, num_2);
}
```

All we've changed here is switched `print_numbers` to accept `&mut &'a NoClone` arguments instead of `&'a NoClone`. Why is this invalid? Well, we could add a quick line to `print_numbers`:

```rs
*num_1 = *num_2;
```

In our `main` function, this would update `num_1_ref` to point to `num_2`. Since `num_2` is dropped before `num_1_ref` is printed, this swap would cause undefined behavior, so the compiler is right to complain about this.

How does lifetime variance prevent this second case, but allow the first one? `&'b mut T` is (just like `&'b T`) covariant for `'b`. However, it's *invariant* for `T`, which means only and exactly a `T` can be passed in. Since `T` in this example is `&'a NoClone`, our new `print_numbers` requires the two `&'a NoClone` to have exactly the same lifetime `'a`.

## Applying this to our graph

We mentioned earlier we want our add edge method to have two properties:

- When adding an edge from `&'a TreeNode` and `&'b TreeNode`, we ensure that `'a` and `'b` have exactly the same lifetime.
- If `&'a TreeNode` and `&'b TreeNode` came from different graphs, `'a` and `'b` will be different lifetimes.

To get the first property, we just need to make sure `&'a TreeNode` is invariant for `'a`. To get this, we can use a `PhantomData` and a wrapper struct:

```rs
use std::marker::PhantomData;

#[derive(Clone, Copy)]
struct TreeNodeRef<'a> {
    inner: &'a TreeNode,
    mark_invariant: PhantomData<&'a mut &'a ()>,
}

impl <'a> TreeNodeRef<'a> {
    fn set_parent(self, new_parent: TreeNodeRef<'a>) {
        self.parent.set(new_parent.inner as *const TreeNode);
    }
    fn get_parent(self) -> TreeNodeRef<'a> {
        TreeNodeRef {
            inner: unsafe { &*self.parent.get() },
            mark_invariant: PhantomData,
        }
    }
}
```

We still need that second property, though, where two trees always produce `TreeNodeRef`s with different lifetimes. There's a lot here that won't work. For instance, this stripped down example initially appears to error correctly:

```rs
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
struct TreeNodeRef<'a> {
    // inner node data ommited for brevity
    mark_invariant: PhantomData<&'a mut &'a ()>,
}

fn same_lifetime<'a>(a: TreeNodeRef<'a>, b: TreeNodeRef<'a>) {
    // two nodes have same lifetime; set recursive pointers here
}

struct Tree;
impl Tree {
    fn root<'a>(&'a self) -> TreeNodeRef<'a> {
        TreeNodeRef {
            mark_invariant: PhantomData,
        }
    }
}

fn main() {
    let tree_1 = Tree;
    let root_1 = tree_1.root();
    {
      let tree_2 = Tree;
      let root_2 = tree_2.root();
      same_lifetime(root_1, root_2);
    }
    println!("{:?}", root_1);
}
```

This fails because `root_1` and `root_2` have different lifetimes. But move the creation of `root_1` into the block, and we can get this to incorrectly compile:

```rs
fn main() {
    let tree_1 = Tree;
    {
      let tree_2 = Tree;
      let root_1 = tree_1.root();
      let root_2 = tree_2.root();
      same_lifetime(root_1, root_2);
    }
    println!("{:?}", tree_1.root());
}
```

Now root_1 and root_2 are created at the same time, and so share the same lifetime. How do we make this impossible? Initially it may seem we could use a closure to force the `root()` calls to be in different scopes:

```rs
struct Tree;
impl Tree {
    fn with_root<'a, F: FnOnce(TreeNodeRef<'a>)>(&'a self, func: F) {
        func(TreeNodeRef {
            mark_invariant: PhantomData,
        })
    }
}

fn main() {
    let tree_1 = Tree;
    let tree_2 = Tree;
    tree_1.with_root(|root_1| {
        tree_2.with_root(|root_2| {
            same_lifetime(root_1, root_2);
        })
    });
}
```

However, you'll find that this actually compiles! How is this possible? My understanding gets a little fuzzier here, but I'm pretty sure since `with_root`'s `&'a self` is covariant for `'a`, it allows the constructed `TreeNodeRef<'a>` to also have an arbitrarily long lifetime. How can we make this correctly error? Ideally we need some way to express that the `FnOnce` passed to `with_root` should *not* have a lifetime selected by the caller, but instead some unique lifetime determined by `with_root`. Fortunately for us, Rust has a bit of magic called [higher ranked trait bounds](https://doc.rust-lang.org/nomicon/hrtb.html) that do exactly that:

```rs
impl Tree {
    fn with_root<F: for <'any> FnOnce(TreeNodeRef<'any>)>(&self, func: F) {
        func(TreeNodeRef {
            mark_invariant: PhantomData,
        })
    }
}
```

With the code above, our `main` will correctly fail to compile, since `root_1` and `root_2` will be guaranteed to have different lifetimes.


## Why not `Cell<&'a Node<'a>>`?

Some ppl construct graphs using a node that looks something like this:

```rs
struct Node<'a> {
    parent: Cell<Option<&'a Self>>
}
```

While this has the advantage of not needing unsafe, it unfortunately means your graph struct has a lifetime in it:

```rs
struct Graph<'a> {
    arena: Arena<Node<'a>>,
}
```

I've found that this lifetime gets in the way a lot, and results in unergonomic interfaces. It also means the `Graph` can never be moved after a node is inserted, which is an unnecessary requirement, given how arena allocated nodes are always on the heap.


## Does this actually work correctly??

maybe??? idk
