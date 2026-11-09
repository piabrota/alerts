; P3: only_ufs and min_severity commute; channels have parity
; Written by `just proofs` (crates/alerts-proofs); checked by `just prove`.
; Every conjecture is asserted negated: every answer must be unsat.

(declare-sort Alert 0)
(declare-fun touches-kept-uf (Alert) Bool)
(declare-fun sev-at-least (Alert) Bool)

; c1: the two stage orders pass the same alerts.
(push)
(assert (exists ((a Alert)) (not (= (and (touches-kept-uf a) (sev-at-least a)) (and (sev-at-least a) (touches-kept-uf a))))))
(check-sat)
(pop)

; c2: parity: both channels deliver exactly the harness output.
(declare-fun harness (Alert) Bool)
(define-fun discord ((a Alert)) Bool (harness a))
(define-fun feed ((a Alert)) Bool (harness a))
(push)
(assert (exists ((a Alert)) (not (= (discord a) (feed a)))))
(check-sat)
(pop)
