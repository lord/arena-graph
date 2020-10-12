<p align="center">
  <img src="https://user-images.githubusercontent.com/1976330/95137243-83675c80-071c-11eb-815b-6c652262cfc2.png" alt="arena-graph: fast, arena-allocated graphs" width="226">
</p>

A library for constructing fast, pointer-based graphs in Rust. A lil hacky, and my understanding of variance is dubious at best, so consult your local lifetime specialist before using. Useful if your graph has the following properties:

- the node lookup overhead of a slotmap-style graph is too slow for your the problem you're trying to solve
- nodes in your graph all have the same type
- you only need `&` access to nodes once they've been added, and can use mutexes or cells to handle mutation
- your graph doesn't need to delete individual nodes, only an entire graph of nodes all at once (although having a `Cell<bool>` that indicates deleted status is fine, as is creating a linked list of deleted nodes for recycling)
- your Node's Drop implementation doesn't need to access other nodes

Still need to write documentation, but you can check out [anchors](https://github.com/lord/anchors) if you're interested in seeing this in action.
