package nodes

import (
	"axlab.dev/byte/pkg/core"
	"axlab.dev/util"
)

type NodeList struct {
	nodes []*Node
}

func (ls *NodeList) Len() int {
	return len(ls.nodes)
}

func (ls *NodeList) Get(i int) *Node {
	return ls.nodes[i]
}

func (ls *NodeList) Nodes() []*Node {
	return ls.nodes
}

func (ls *NodeList) Add(nodes ...*Node) {
	len := ls.Len()
	ls.nodes = append(ls.nodes, nodes...)
	ls.updateFrom(len)
}

func (ls *NodeList) Insert(index int, nodes ...*Node) {
	util.Insert(&ls.nodes, index, nodes...)
	ls.updateFrom(index)
}

func (ls *NodeList) Remove(index int) *Node {
	node := ls.nodes[index]
	ls.nodes = append(ls.nodes[:index], ls.nodes[index+1:]...)
	ls.updateFrom(index)
	node.list, node.index = nil, -1
	return node
}

func (ls *NodeList) SplitAt(index int) *NodeList {
	return ls.Extract(index, ls.Len())
}

func (ls *NodeList) Extract(sta, end int) *NodeList {
	var nodes []*Node
	if end > sta {
		nodes = append(nodes, ls.nodes[sta:end]...)
		ls.nodes = append(ls.nodes[:sta], ls.nodes[end:]...)
		ls.updateFrom(sta)
	}
	out := &NodeList{nodes}
	out.updateFrom(0)
	return out
}

func (ls *NodeList) updateFrom(index int) {
	for i := index; i < len(ls.nodes); i++ {
		ls.nodes[i].list = ls
		ls.nodes[i].index = i
	}
}

type Node struct {
	val   core.Value
	pos   int
	list  *NodeList
	index int
}

func NewNode(val core.Value, pos int) *Node {
	return &Node{val, pos, nil, -1}
}

func (node *Node) List() *NodeList {
	return node.list
}

func (node *Node) Index() int {
	return node.index
}

func (node *Node) Next() *Node {
	if ls := node.list; ls != nil && node.index < ls.Len()-1 {
		return ls.Get(node.index + 1)
	}
	return nil
}

func (node *Node) Prev() *Node {
	if ls := node.list; ls != nil && node.index > 0 {
		return ls.Get(node.index - 1)
	}
	return nil
}

func (node *Node) Key() core.Value {
	if v, ok := node.val.Any().(WithKey); ok {
		return v.Key()
	} else {
		return core.Value{}
	}
}

func (node *Node) Offset() int {
	return node.pos
}

func (node *Node) Value() core.Value {
	return node.val
}
