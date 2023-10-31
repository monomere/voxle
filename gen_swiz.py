import itertools

xyzs = set(itertools.product(('x', 'y', 'z'), repeat=3))

for c in itertools.product(('x', 'y', 'z', 'w'), repeat=3):
	if c in xyzs: continue
	print(f"impl_swizzle_for_vec!($n -> 3: {''.join(c)} => {', '.join(c)});")
