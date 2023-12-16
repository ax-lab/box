package nodes

import (
	"fmt"
	"slices"
	"strings"

	"axlab.dev/byte/pkg/core"
	"axlab.dev/byte/pkg/lexer"
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

func (ls *NodeList) RemoveAt(index int) *Node {
	node := ls.nodes[index]
	ls.nodes = append(ls.nodes[:index], ls.nodes[index+1:]...)
	ls.updateFrom(index)
	node.list, node.index = nil, -1
	return node
}

func (ls *NodeList) SplitBy(split func(node *Node) bool) (out []*NodeList) {
	for i := ls.Len() - 1; i >= 0; i-- {
		if split(ls.nodes[i]) {
			if i < ls.Len()-1 {
				out = append(out, ls.SplitAt(i+1))
			}
			ls.RemoveAt(i)
		}
	}

	slices.Reverse(out)
	if len(out) == 0 {
		out = append(out, ls)
	}
	return out
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

func (ls *NodeList) String() string {
	out := strings.Builder{}
	out.WriteString("{")
	for i, it := range ls.nodes {
		out.WriteString("\n    ")
		out.WriteString(fmt.Sprintf("[%03d] = ", i))
		out.WriteString(util.Indented(fmt.Sprintf("%s -- %s", it.String(), it.Span().Location())))
	}
	if len(ls.nodes) > 0 {
		out.WriteString("\n")
	} else {
		out.WriteString(" ")
	}
	out.WriteString("}")
	return out.String()
}

type Node struct {
	val   core.Value
	span  lexer.Span
	list  *NodeList
	index int
}

func NewNode(val core.Value, span lexer.Span) *Node {
	return &Node{val, span, nil, -1}
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
	key, _ := GetKey(node.val)
	return key
}

func (node *Node) Span() lexer.Span {
	return node.span
}

func (node *Node) Offset() int {
	return node.span.Sta
}

func (node *Node) Value() core.Value {
	return node.val
}

func (node *Node) String() string {
	return fmt.Sprintf("Node(%s)", node.val.String())
}
