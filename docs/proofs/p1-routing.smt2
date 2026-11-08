; P1: IBGE routing is total on valid prefixes, visible when unrouted, and lands v0 in the South
; Written by `just proofs` (crates/alerts-proofs); checked by `just prove`.
; Every conjecture is asserted negated: every answer must be unsat.

(declare-datatypes ((Route 0)) (((Unrouted) (UF-RO) (UF-AC) (UF-AM) (UF-RR) (UF-PA) (UF-AP) (UF-TO) (UF-MA) (UF-PI) (UF-CE) (UF-RN) (UF-PB) (UF-PE) (UF-AL) (UF-SE) (UF-BA) (UF-MG) (UF-ES) (UF-RJ) (UF-SP) (UF-PR) (UF-SC) (UF-RS) (UF-MS) (UF-MT) (UF-GO) (UF-DF))))
(define-fun prefix ((m Int)) Int (div m 100000))
(define-fun muni ((m Int)) Bool (and (<= 1000000 m) (< m 10000000)))
(define-fun route ((p Int)) Route
  (ite (= p 11) UF-RO (ite (= p 12) UF-AC (ite (= p 13) UF-AM (ite (= p 14) UF-RR (ite (= p 15) UF-PA (ite (= p 16) UF-AP (ite (= p 17) UF-TO (ite (= p 21) UF-MA (ite (= p 22) UF-PI (ite (= p 23) UF-CE (ite (= p 24) UF-RN (ite (= p 25) UF-PB (ite (= p 26) UF-PE (ite (= p 27) UF-AL (ite (= p 28) UF-SE (ite (= p 29) UF-BA (ite (= p 31) UF-MG (ite (= p 32) UF-ES (ite (= p 33) UF-RJ (ite (= p 35) UF-SP (ite (= p 41) UF-PR (ite (= p 42) UF-SC (ite (= p 43) UF-RS (ite (= p 50) UF-MS (ite (= p 51) UF-MT (ite (= p 52) UF-GO (ite (= p 53) UF-DF Unrouted))))))))))))))))))))))))))))
(define-fun valid ((p Int)) Bool (or (= p 11) (= p 12) (= p 13) (= p 14) (= p 15) (= p 16) (= p 17) (= p 21) (= p 22) (= p 23) (= p 24) (= p 25) (= p 26) (= p 27) (= p 28) (= p 29) (= p 31) (= p 32) (= p 33) (= p 35) (= p 41) (= p 42) (= p 43) (= p 50) (= p 51) (= p 52) (= p 53)))

; c1: no valid prefix of a municipality code is unrouted.
(push)
(assert (exists ((m Int)) (and (muni m) (valid (prefix m)) (= (route (prefix m)) Unrouted))))
(check-sat)
(pop)

; c2: an invalid prefix never routes to a UF: the gap stays visible.
(push)
(assert (exists ((m Int)) (and (muni m) (not (valid (prefix m))) (distinct (route (prefix m)) Unrouted))))
(check-sat)
(pop)

; c3: every South municipality code (prefix 41, 42, 43) routes into v0.
(push)
(assert (exists ((m Int)) (and (muni m) (<= 41 (prefix m)) (<= (prefix m) 43) (not (or (= (route (prefix m)) UF-PR) (= (route (prefix m)) UF-SC) (= (route (prefix m)) UF-RS))))))
(check-sat)
(pop)

; c4: the table is deterministic: two different valid prefixes never share a UF.
(push)
(assert (exists ((p Int) (q Int)) (and (valid p) (valid q) (distinct p q) (= (route p) (route q)))))
(check-sat)
(pop)
