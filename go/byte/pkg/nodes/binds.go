package nodes

import (
	"sort"

	"axlab.dev/byte/pkg/core"
)

type NodeList struct{}

type Node struct {
	val any
	pos int
}

func NewNode(val any, pos int) Node {
	return Node{val, pos}
}

func (node Node) Offset() int {
	return node.pos
}

func (node Node) Value() any {
	return node.val
}

type Segment struct{}

type NodeSet struct {
}

func (set *NodeSet) Add(node Node) {}

func (set *NodeSet) Bind(sta, end int, key, val, ord core.Value) {}

func (set *NodeSet) Peek() Segment {
	panic("TODO")
}

func (set *NodeSet) Shift() Segment {
	panic("TODO")
}

type RangeTable struct {
	segmentTable
}

func (tb *RangeTable) Get(pos int) any {
	cnt := len(tb.segments)
	idx := sort.Search(cnt, func(i int) bool {
		return tb.segments[i].end > pos
	})
	if idx < cnt && pos >= tb.segments[idx].sta {
		return tb.segments[idx].bind.val
	}
	return nil
}

func (tb *RangeTable) Set(sta, end int, val any) {
	tb.bind(sta, end, val)
}

func (tb *RangeTable) Add(node Node) {
	pos := node.Offset()
	cnt := len(tb.segments)
	idx := sort.Search(cnt, func(i int) bool {
		return tb.segments[i].end > pos
	})
	if idx < cnt && pos >= tb.segments[idx].sta {
		insertNode(&tb.segments[idx].list, node)
	} else {
		insertNode(&tb.unbound, node)
	}
}

func insertNode(nodes *[]Node, node Node) {
	offset := node.Offset()
	list := *nodes
	if len(list) == 0 || list[len(list)-1].Offset() <= offset {
		list = append(list, node)
	} else {
		idx := sort.Search(len(list), func(i int) bool {
			return list[i].Offset() > offset
		})
		list = append(append(list[:idx], node), list[idx:]...)
	}
	*nodes = list
}

type binding struct {
	sta int
	end int
	val any
}

func (bind *binding) overrides(other *binding) bool {
	if is_more_specific := other.contains(bind); is_more_specific {
		return true
	}

	intersect := bind.sta < other.end && other.sta < bind.end
	return intersect && !bind.contains(other)
}

func (bind *binding) contains(other *binding) bool {
	return bind.sta <= other.sta && other.end <= bind.end
}

type segment struct {
	sta  int
	end  int
	bind *binding
	list []Node
}

func (seg *segment) updateQueuePos() {}

func (seg *segment) removeQueuePos() {}

func (seg *segment) splitOff(at int) (new *segment) {
	if at <= seg.sta || seg.end <= at {
		panic("splitting a segment out of bounds")
	}

	lhs, rhs := splitNodes(seg.list, at)
	new = &segment{at, seg.end, seg.bind, rhs}
	seg.end, seg.list = at, lhs
	return new
}

type segmentTable struct {
	segments []*segment
	unbound  []Node
}

func (tb *segmentTable) bind(sta, end int, val any) {
	if sta >= end {
		return
	}

	new_bind := &binding{sta, end, val}
	pre, mid, pos := splitSegments(tb.segments, sta, end)

	tb.segments = append([]*segment(nil), pre...)

	push := func(seg *segment, isNew bool) *segment {
		if isNew {
			seg.list = extractNodes(&tb.unbound, seg.sta, seg.end)
		}

		if len(tb.segments) > 0 {
			last := tb.segments[len(tb.segments)-1]
			can_merge := last.bind == seg.bind && last.end == seg.sta
			if can_merge {
				last.end = seg.end
				last.list = append(last.list, seg.list...)
				seg.list = nil
				seg.removeQueuePos()
				return last
			}
		}

		tb.segments = append(tb.segments, seg)
		seg.updateQueuePos()
		return seg
	}

	cur := sta
	for _, next := range mid {
		if has_gap := next.sta > cur; has_gap {
			push(&segment{cur, next.sta, new_bind, nil}, true)
			cur = next.sta
		}

		if new_bind.overrides(next.bind) {
			if split_pre := next.sta < cur; split_pre {
				next = push(next, false)
				next = next.splitOff(cur)
			}

			prev_bind := next.bind
			next.bind = new_bind
			next = push(next, false)

			if split_pos := end < next.end; split_pos {
				next = next.splitOff(end)
				next.bind = prev_bind
				push(next, false)
			}
		} else {
			next = push(next, false)
		}
		cur = next.end
	}

	if cur < end {
		push(&segment{cur, end, new_bind, nil}, true)
	}

	tb.segments = append(tb.segments, pos...)
}

func splitSegments(segments []*segment, sta, end int) (pre, mid, pos []*segment) {
	count := len(segments)
	idx_sta := sort.Search(count, func(i int) bool {
		return segments[i].end > sta
	})
	idx_end := idx_sta + sort.Search(count-idx_sta, func(i int) bool {
		return segments[i+idx_sta].sta >= end
	})

	pre = segments[:idx_sta]
	mid = segments[idx_sta:idx_end]
	pos = segments[idx_end:]
	return
}

func extractNodes(nodes *[]Node, sta, end int) (del []Node) {
	out := *nodes
	count := len(out)
	idx_sta := sort.Search(count, func(i int) bool {
		return out[i].Offset() >= sta
	})
	idx_end := idx_sta + sort.Search(count-idx_sta, func(i int) bool {
		return out[i+idx_sta].Offset() >= end
	})

	del = append(del, out[idx_sta:idx_end]...)
	out = append(out[:idx_sta], out[idx_end:]...)

	*nodes = out
	return del
}

func splitNodes(nodes []Node, at int) (lhs, rhs []Node) {
	len := len(nodes)
	idx := sort.Search(len, func(i int) bool {
		return nodes[i].Offset() >= at
	})

	// don't share the underlying storage since those are writable
	lhs = nodes[:idx]
	rhs = append([]Node(nil), nodes[idx:]...)
	return
}
