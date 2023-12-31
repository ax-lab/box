package nodes

import (
	"container/heap"
	"fmt"
	"sort"
	"strings"

	"axlab.dev/byte/pkg/core"
	"axlab.dev/byte/pkg/lexer"
)

func GetKey(v core.Value) (core.Value, bool) {
	type withKey interface {
		GetValueKey(v core.Value) core.Value
	}

	if t, ok := v.Type().Def().(withKey); ok {
		return t.GetValueKey(v), true
	}
	return core.Value{}, false
}

type Segment struct {
	nodes []*Node
	bind  *binding
	sta   int
	end   int
}

func newSegment(seg *segment) Segment {
	return Segment{seg.list, seg.bind, seg.sta, seg.end}
}

func (seg *Segment) String() string {
	out := strings.Builder{}
	out.WriteString(fmt.Sprintf("Segment([%d..%d] %s #%s = %+v -- %+v)", seg.sta, seg.end, seg.bind.key.Debug(), seg.bind.ord.Debug(), seg.bind.val, seg.nodes))
	return out.String()
}

type NodeSet struct {
	types    *core.TypeMap
	bindings map[core.Value]*RangeTable
	queue    *nodeSetQueue
}

func newNodeSet(types *core.TypeMap, queue *nodeSetQueue) *NodeSet {
	return &NodeSet{types: types, queue: queue}
}

func (set *NodeSet) Types() *core.TypeMap {
	return set.types
}

func (set *NodeSet) Add(node *Node) {
	if key := node.Key(); !key.IsZero() {
		tb := set.getTable(key)
		tb.Add(node)
	}
}

func (set *NodeSet) Bind(span lexer.Span, key, ord core.Value, val any) {
	if !key.IsZero() {
		tb := set.getTable(key)
		tb.Bind(span, key, ord, val)
	}
}

type unboundSort struct {
	keys  []core.Value
	nodes [][]*Node
}

func (s unboundSort) Len() int {
	return len(s.keys)
}

func (s unboundSort) Less(a, b int) bool {
	return s.keys[a].Less(s.keys[b])
}

func (s unboundSort) Swap(a, b int) {
	s.keys[a], s.keys[b] = s.keys[b], s.keys[a]
	s.nodes[a], s.nodes[b] = s.nodes[b], s.nodes[a]
}

func (set *NodeSet) PopUnbound() (keys []core.Value, nodes [][]*Node) {
	for k, v := range set.bindings {
		if len(v.unbound) > 0 {
			keys = append(keys, k)
			nodes = append(nodes, v.unbound)
			v.unbound = nil
		}
	}

	sortable := unboundSort{keys, nodes}
	sort.Sort(sortable)

	return
}

type nodeSetQueue struct {
	segments []*segment
}

func (q *nodeSetQueue) Peek() Segment {
	q.shiftEmpty()
	if q.Len() > 0 {
		seg := q.segments[0]
		return newSegment(seg)
	}
	return Segment{}
}

func (q *nodeSetQueue) Shift() Segment {
	q.shiftEmpty()
	if q.Len() > 0 {
		seg := q.segments[0]
		out := newSegment(seg)
		seg.list = nil
		heap.Pop(q)
		return out
	}
	return Segment{}
}

func (q *nodeSetQueue) Len() int {
	return len(q.segments)
}

func (q *nodeSetQueue) Less(i, j int) bool {
	b0, b1 := q.segments[i].bind, q.segments[j].bind

	if ord := b0.ord.Compare(b1.ord); ord != 0 {
		return ord < 0
	}

	if key := b0.key.Compare(b1.key); key != 0 {
		return key < 0
	}

	if b0.src.Sort != b1.src.Sort {
		return b0.src.Sort < b1.src.Sort
	}

	if b0.sta != b1.sta {
		return b0.sta < b1.sta
	}

	if b0.end != b1.end {
		return b0.end < b1.end
	}

	return false
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
	s := q.segments[n-1]
	q.segments = q.segments[:n-1]
	s.queue = -1
	return s
}

func (q *nodeSetQueue) shiftEmpty() {
	for q.Len() > 0 && len(q.segments[0].list) == 0 {
		heap.Pop(q)
	}
}

func (set *NodeSet) getTable(key core.Value) *RangeTable {
	if tb, ok := set.bindings[key]; ok {
		return tb
	}

	if set.bindings == nil {
		set.bindings = make(map[core.Value]*RangeTable)
	}

	tb := &RangeTable{queue: set.queue}
	set.bindings[key] = tb
	return tb
}

type RangeTable struct {
	queue    *nodeSetQueue
	segments []*segment
	unbound  []*Node
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

func (tb *RangeTable) Bind(span lexer.Span, key, ord core.Value, val any) {
	sta, end := span.Sta, span.End
	if sta >= end {
		return
	}
	bind := &binding{sta, end, span.Src, key, ord, val}
	tb.addBinding(bind)
}

func (tb *RangeTable) Add(node *Node) {
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

func insertNode(nodes *[]*Node, node *Node) {
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
	src *lexer.Source
	key core.Value
	ord core.Value
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
	sta   int
	end   int
	bind  *binding
	list  []*Node
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

func extractNodes(nodes *[]*Node, sta, end int) (del []*Node) {
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

func splitNodes(nodes []*Node, at int) (lhs, rhs []*Node) {
	len := len(nodes)
	idx := sort.Search(len, func(i int) bool {
		return nodes[i].Offset() >= at
	})

	// don't share the underlying storage since those are writable
	lhs = nodes[:idx]
	rhs = append([]*Node(nil), nodes[idx:]...)
	return
}
