; P6: the CAP identifier is unique only when the source and alert ids keep a form
; Written by `just proofs` (crates/alerts-proofs); checked by `just prove`.
; Each statement is asserted negated: `unsat` proves it. A statement marked
; `expect: sat` must have a counterexample, and the model is that counterexample.

; identifier (alerts-feed/src/lib.rs:65-78) joins the sender, the source id and the alert id
; with `-`, then replaces the characters CAP forbids. The statements take ids without those
; characters, where the replacement is the identity.
(define-fun ident ((src String) (id String)) String (str.++ "piabrota-alerts-" src "-" id))
(define-fun alnum () RegLan (re.union (re.range "a" "z") (re.range "0" "9")))
; is_slug (alerts-crud/src/validate.rs:62-68): the form of a source id.
(define-fun slug ((s String)) Bool
  (and (str.in_re s (re.+ (re.union alnum (str.to_re "-"))))
       (not (str.prefixof "-" s)) (not (str.suffixof "-" s))))
(define-fun plain ((s String)) Bool
  (str.in_re s (re.* (re.union alnum (str.to_re "-") (str.to_re "_") (str.to_re ".") (str.to_re ":")))))
(declare-const s1 String) (declare-const s2 String) (declare-const i1 String) (declare-const i2 String)

; identifier_unique (crates/alerts-feed/src/lib.rs:65-68): two alerts with different (source, id) never share a CAP identifier
; expect: sat (the `-` that joins source and id is also a letter of both: the source cemaden with the id ped-1 and the source cemaden-ped (both in the catalog) with the id 1 give one identifier (W6))
(push)
(assert (and (slug s1) (slug s2) (plain i1) (plain i2)))
(assert (and (str.in_re i1 (re.++ alnum re.all)) (str.in_re i2 (re.++ alnum re.all))))
(assert (not (and (= s1 s2) (= i1 i2))))
(assert (= (ident s1 i1) (ident s2 i2)))
(check-sat)
(pop)

; identifier_unique_when_ids_start_with_a_digit (crates/alerts-feed/src/lib.rs:65-68): the identifier is unique when no source id has a `-` before a digit and every alert id starts with a digit
(push)
(declare-fun w (Int) Int)
(declare-const L1 Int) (declare-const L2 Int)
(define-fun digit ((k Int)) Bool (<= 48 (w k) 57))
(assert (and (< 0 L1) (< L1 L2)))
; each split is a `-` followed by the first character of an id: a digit
(assert (and (= (w L1) 45) (digit (+ L1 1)) (= (w L2) 45) (digit (+ L2 1))))
; the longer source w[0..L2) has no `-` before a digit, and does not end in `-`
(assert (forall ((k Int)) (=> (and (<= 0 k) (< (+ k 1) L2)) (not (and (= (w k) 45) (digit (+ k 1)))))))
(assert (distinct (w (- L2 1)) 45))
(check-sat)
(pop)
