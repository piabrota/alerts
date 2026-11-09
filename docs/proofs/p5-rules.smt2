; P5: the rule algebra holds its laws key by key, except filter over union
; Written by `just proofs` (crates/alerts-proofs); checked by `just prove`.
; Each statement is asserted negated: `unsat` proves it. A statement marked
; `expect: sat` must have a counterexample, and the model is that counterexample.

; Every set operation of `go` (expr.rs:187-266) acts key by key: `canonical` (expr.rs:268-281)
; keeps one item per key, the one that `wins` (expr.rs:289-292), a strict total order on items;
; `intersect` and `diff` test keys; `filter` tests the kept item. The order of a result is the
; canonical sort of its set. So one key is the whole statement. For that key, an operand is a
; Slot: no item, or the rank of its winning item in the `wins` order (the lower rank wins).
; `map`, `dedupe` and `take` do not act key by key and are not here.
(declare-datatypes ((Slot 0)) (((none) (some (winner Int)))))
(define-fun first ((a Int) (b Int)) Int (ite (<= a b) a b))
(define-fun union2 ((a Slot) (b Slot)) Slot
  (ite (is-none a) b (ite (is-none b) a (some (first (winner a) (winner b))))))
(define-fun inter2 ((a Slot) (b Slot)) Slot
  (ite (or (is-none a) (is-none b)) none (some (first (winner a) (winner b)))))
(define-fun diff ((a Slot) (b Slot)) Slot (ite (is-some b) none a))
(declare-fun p (Int) Bool)
(declare-fun q (Int) Bool)
(define-fun filter-p ((a Slot)) Slot (ite (and (is-some a) (p (winner a))) a none))
(define-fun filter-q ((a Slot)) Slot (ite (and (is-some a) (q (winner a))) a none))
(define-fun filter-pq ((a Slot)) Slot (ite (and (is-some a) (p (winner a)) (q (winner a))) a none))
(declare-const a Slot) (declare-const b Slot) (declare-const c Slot)

; The alert predicates (expr.rs:329-337) call the harness stages of alerts-core (lib.rs:224-232).
; A set of UFs is a 27-bit vector, one bit per Uf::ALL; a severity is its rank, Minor 0 to Extreme 3.
(declare-const A (_ BitVec 27)) (declare-const S (_ BitVec 27)) (declare-const T (_ BitVec 27))
(define-fun touches ((x (_ BitVec 27)) (s (_ BitVec 27))) Bool (distinct (bvand x s) (_ bv0 27)))
(define-fun sev ((x Int)) Bool (<= 0 x 3))
(declare-const sv Int) (declare-const lo Int) (declare-const hi Int)

; union_commutative (crates/alerts-rss/src/expr.rs:5, 211-217): union(a, b) = union(b, a)
(push)
(assert (not (= (union2 a b) (union2 b a))))
(check-sat)
(pop)

; union_associative (crates/alerts-rss/src/expr.rs:5, 211-217): union(union(a, b), c) = union(a, union(b, c))
(push)
(assert (not (= (union2 (union2 a b) c) (union2 a (union2 b c)))))
(check-sat)
(pop)

; union_idempotent (crates/alerts-rss/src/expr.rs:5, 211-217): union(a, a) = a
(push)
(assert (not (= (union2 a a) a)))
(check-sat)
(pop)

; union_empty (crates/alerts-rss/src/expr.rs:33): union(a, union []) = a
(push)
(assert (not (= (union2 a none) a)))
(check-sat)
(pop)

; intersect_commutative (crates/alerts-rss/src/expr.rs:5, 218-235): intersect(a, b) = intersect(b, a)
(push)
(assert (not (= (inter2 a b) (inter2 b a))))
(check-sat)
(pop)

; intersect_associative (crates/alerts-rss/src/expr.rs:5, 218-235): intersect(intersect(a, b), c) = intersect(a, intersect(b, c))
(push)
(assert (not (= (inter2 (inter2 a b) c) (inter2 a (inter2 b c)))))
(check-sat)
(pop)

; intersect_idempotent (crates/alerts-rss/src/expr.rs:5, 218-235): intersect(a, a) = a
(push)
(assert (not (= (inter2 a a) a)))
(check-sat)
(pop)

; diff_self_empty (crates/alerts-rss/src/expr.rs:6, 236-241): diff(a, a) is empty
(push)
(assert (not (= (diff a a) none)))
(check-sat)
(pop)

; diff_drops_right (crates/alerts-rss/src/expr.rs:6, 236-241): diff(a, b) never holds a key of b
(push)
(assert (and (is-some b) (is-some (diff a b))))
(check-sat)
(pop)

; diff_of_union (crates/alerts-rss/src/expr.rs:236-241): diff(a, union(b, c)) = diff(diff(a, b), c)
(push)
(assert (not (= (diff a (union2 b c)) (diff (diff a b) c))))
(check-sat)
(pop)

; filter_filter (crates/alerts-rss/src/expr.rs:8, 242-247, 338): filter(p, filter(q, a)) = filter(all[p, q], a)
(push)
(assert (not (= (filter-p (filter-q a)) (filter-pq a))))
(check-sat)
(pop)

; filter_of_diff (crates/alerts-rss/src/expr.rs:236-247): filter(p, diff(a, b)) = diff(filter(p, a), b)
(push)
(assert (not (= (filter-p (diff a b)) (diff (filter-p a) b))))
(check-sat)
(pop)

; filter_union_distributes (crates/alerts-rss/src/expr.rs:9): filter(p, union(a, b)) = union(filter(p, a), filter(p, b))
; expect: sat (a key in both operands whose winner fails p and whose loser passes: the left side drops the key, the right keeps the loser (W5))
(push)
(assert (not (= (filter-p (union2 a b)) (union2 (filter-p a) (filter-p b)))))
(check-sat)
(pop)

; filter_union_when_items_agree (crates/alerts-rss/src/expr.rs:9): the law of expr.rs:9 holds when p gives the same answer on the two items of a shared key
(push)
(assert (=> (and (is-some a) (is-some b)) (= (p (winner a)) (p (winner b)))))
(assert (not (= (filter-p (union2 a b)) (union2 (filter-p a) (filter-p b)))))
(check-sat)
(pop)

; ufs_any_is_union (crates/alerts-rss/src/expr.rs:333-339): any[ufs S, ufs T] keeps the alerts of ufs (S union T)
(push)
(assert (not (= (or (touches A S) (touches A T)) (touches A (bvor S T)))))
(check-sat)
(pop)

; ufs_intersection_implies_all (crates/alerts-rss/src/expr.rs:333-338): ufs (S intersect T) keeps only alerts that all[ufs S, ufs T] keeps
(push)
(assert (and (touches A (bvand S T)) (not (and (touches A S) (touches A T)))))
(check-sat)
(pop)

; ufs_all_is_intersection (crates/alerts-rss/src/expr.rs:333-338): all[ufs S, ufs T] keeps the alerts of ufs (S intersect T)
; expect: sat (an alert for PR and SP passes all[ufs [PR], ufs [SP]], but ufs [] keeps nothing: `all` of two UF tests asks for each UF list, not for a common UF)
(push)
(assert (not (= (and (touches A S) (touches A T)) (touches A (bvand S T)))))
(check-sat)
(pop)

; severity_all_is_max (crates/alerts-rss/src/expr.rs:329-338): all[min_severity lo, min_severity hi] = min_severity (max lo hi)
(push)
(assert (and (sev sv) (sev lo) (sev hi)))
(assert (not (= (and (>= sv lo) (>= sv hi)) (>= sv (ite (>= lo hi) lo hi)))))
(check-sat)
(pop)

; severity_any_is_min (crates/alerts-rss/src/expr.rs:329-339): any[min_severity lo, min_severity hi] = min_severity (min lo hi)
(push)
(assert (and (sev sv) (sev lo) (sev hi)))
(assert (not (= (or (>= sv lo) (>= sv hi)) (>= sv (ite (<= lo hi) lo hi)))))
(check-sat)
(pop)
