package nodes

import (
	"container/heap"
	"sort"

	"axlab.dev/byte/pkg/core"
)

type WithKey interface {
	Key() core.Value
}

type NodeList struct{}

type Node struct {
	val core.Value
	pos int
}

func NewNode(val core.Value, pos int) Node {
	return Node{val, pos}
}

func (node Node) Key() core.Value {
	if v, ok := node.val.Any().(WithKey); ok {
		return v.Key()
	} else {
		return core.Value{}
	}
}

func (node Node) Offset() int {
	return node.pos
}

func (node Node) Value() any {
	return node.val
}

type Segment struct {
	nodes []Node
	bind  *binding
	sta   int
	end   int
}

func newSegment(seg *segment) Segment {
	return Segment{seg.list, seg.bind, seg.sta, seg.end}
}

type NodeSet struct {
	types    core.TypeMap
	bindings map[core.Value]*RangeTable
	queue    nodeSetQueue
}

func (set *NodeSet) Types() *core.TypeMap {
	return &set.types
}

func (set *NodeSet) Add(node Node) {
	if key := node.Key(); !key.IsZero() {
		tb := set.getTable(key)
		tb.Add(node)
	}
}

func (set *NodeSet) Bind(sta, end int, key, val, ord core.Value) {
	if !key.IsZero() {
		tb := set.getTable(key)
		tb.Bind(sta, end, key, val)
	}
}

func (set *NodeSet) Peek() Segment {
	set.shiftEmpty()
	if set.queue.Len() > 0 {
		seg := set.queue.segments[0]
		return newSegment(seg)
	}
	return Segment{}
}

func (set *NodeSet) Shift() Segment {
	set.shiftEmpty()
	if set.queue.Len() > 0 {
		seg := set.queue.segments[0]
		out := newSegment(seg)
		seg.list = nil
		heap.Pop(&set.queue)
		return out
	}
	return Segment{}
}

func (set *NodeSet) PopUnbound() (keys []core.Value, nodes [][]Node) {
	panic("TODO")
}

func (set *NodeSet) shiftEmpty() {
	for set.queue.Len() > 0 && len(set.queue.segments[0].list) == 0 {
		heap.Pop(&set.queue)
	}
}

type nodeSetQueue struct {
	segments []*segment
}

func (q *nodeSetQueue) Len() int {
	return len(q.segments)
}

func (q *nodeSetQueue) Less(i, j int) bool {
	si, sj := q.segments[i], q.segments[j]
	return si.bind.key.Less(sj.bind.key)
}

func (q *nodeSetQueue) Swap(i, j int) {
	q.segments[i], q.segments[j] = q.segments[j], q.segments[i]
	q.segments[i].queue = i
	q.segments[j].queue = j
}

func (q *nodeSetQueue) Push(x any) {
	seg := x.(*segment)
	seg.queue = q.Len()
	q.segments = append(q.segments, seg)
}

func (q *nodeSetQueue) Pop() any {
	n := len(q.segments)
	s := q.segments[n]
	q.segments = q.segments[:n-1]
	s.queue = -1
	return s
}

func (set *NodeSet) getTable(key core.Value) *RangeTable {
	if tb, ok := set.bindings[key]; ok {
		return tb
	}

	if set.bindings == nil {
		set.bindings = make(map[core.Value]*RangeTable)
	}

	tb := &RangeTable{queue: &set.queue}
	set.bindings[key] = tb
	return tb
}

type RangeTable struct {
	queue    *nodeSetQueue
	segments []*segment
	unbound  []Node
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

func (tb *RangeTable) Bind(sta, end int, key, val core.Value) {
	if sta >= end {
		return
	}
	bind := &binding{sta, end, val, key}
	tb.addBinding(bind)
}

func (tb *RangeTable) Set(sta, end int, val any) {
	if sta >= end {
		return
	}
	bind := &binding{sta, end, val, core.Value{}}
	tb.addBinding(bind)
}

func (tb *RangeTable) Add(node Node) {
	pos := node.Offset()
	cnt := len(tb.segments)
	idx := sort.Search(cnt, func(i int) bool {
		return tb.segments[i].end > pos
	})
	if idx < cnt && pos >= tb.segments[idx].sta {
		insertNode(&tb.segments[idx].list, node)
		tb.segments[idx].ensureQueued(tb.queue)
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
	key core.Value
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
	sta   int
	end   int
	bind  *binding
	list  []Node
	queue int
}

func (seg *segment) updateQueuePos(q *nodeSetQueue) {
	if seg.queue >= 0 {
		heap.Fix(q, seg.queue)
	}
}

func (seg *segment) ensureQueued(q *nodeSetQueue) {
	if seg.queue < 0 {
		heap.Push(q, seg)
	}
}

func (seg *segment) removeQueuePos(q *nodeSetQueue) {
	if seg.queue >= 0 {
		heap.Remove(q, seg.queue)
	}
}

func (seg *segment) splitOff(at int) (new *segment) {
	if at <= seg.sta || seg.end <= at {
		panic("splitting a segment out of bounds")
	}

	lhs, rhs := splitNodes(seg.list, at)
	new = &segment{at, seg.end, seg.bind, rhs, -1}
	seg.end, seg.list = at, lhs
	return new
}

func (tb *RangeTable) addBinding(new_bind *binding) {
	sta, end := new_bind.sta, new_bind.end
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
				seg.removeQueuePos(tb.queue)
				return last
			}
		}

		tb.segments = append(tb.segments, seg)
		seg.updateQueuePos(tb.queue)
		return seg
	}

	cur := sta
	for _, next := range mid {
		if has_gap := next.sta > cur; has_gap {
			push(&segment{cur, next.sta, new_bind, nil, -1}, true)
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
		push(&segment{cur, end, new_bind, nil, -1}, true)
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
