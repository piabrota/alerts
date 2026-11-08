; P2: the min_severity stage is monotone in CAP severity
; Written by `just proofs` (crates/alerts-proofs); checked by `just prove`.
; Every conjecture is asserted negated: every answer must be unsat.

; Severities as ranks: Minor 0, Moderate 1, Severe 2, Extreme 3 (alerts_core order).
(define-fun sev ((x Int)) Bool (and (<= 0 x) (<= x 3)))
(define-fun pass ((m Int) (x Int)) Bool (>= x m))

; c1: upward closure: what passes at severity s passes at every s2 >= s.
(push)
(assert (exists ((m Int) (s Int) (s2 Int)) (and (sev m) (sev s) (sev s2) (pass m s) (<= s s2) (not (pass m s2)))))
(check-sat)
(pop)

; c2: the Minor threshold passes every severity.
(push)
(assert (exists ((s Int)) (and (sev s) (not (pass 0 s)))))
(check-sat)
(pop)
