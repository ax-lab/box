package util

func Insert[T any](ls *[]T, index int, elems ...T) {
	inc_len := len(elems)
	if inc_len == 0 {
		return
	}

	cur := *ls
	cur_len := len(cur)
	if index == cur_len {
		*ls = append(cur, elems...)
		return
	}

	new_len := cur_len + inc_len
	if cap(cur) >= new_len {
		cur = cur[0:new_len]
		copy(cur[index+inc_len:], cur[index:])
		copy(cur[index:], elems)
		*ls = cur
	} else {
		var new_cap int
		if cur_len < 8 {
			new_cap = 8
		} else if cur_len < 1024 {
			new_cap = cur_len * 2
		} else {
			new_cap = cur_len + (cur_len / 2)
		}
		if min_cap := cur_len + inc_len*2; min_cap > new_cap {
			new_cap = min_cap
		}

		new := make([]T, new_len, new_cap)
		copy(new, cur[:index])
		copy(new[index:], elems)
		copy(new[index+inc_len:], cur[index:])
		*ls = new
	}
}
