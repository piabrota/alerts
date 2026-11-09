; P4: feed times: the calendar arithmetic is exact; four inputs pass that should not
; Written by `just proofs` (crates/alerts-proofs); checked by `just prove`.
; Each statement is asserted negated: `unsat` proves it. A statement marked
; `expect: sat` must have a counterexample, and the model is that counterexample.

; days_from_civil (time.rs:32-41): days since 1970-01-01 of a civil date. Every Rust `/` here
; has a non-negative dividend and a positive divisor, so it is SMT `div`; `div_euclid` is `div`.
(define-fun days ((y0 Int) (m Int) (d Int)) Int
  (let ((y (ite (<= m 2) (- y0 1) y0)))
  (let ((era (div y 400)))
  (let ((yoe (- y (* era 400))))
  (let ((mp (ite (> m 2) (- m 3) (+ m 9))))
  (let ((doy (- (+ (div (+ (* 153 mp) 2) 5) d) 1)))
  (let ((doe (+ (* yoe 365) (div yoe 4) (- (div yoe 100)) doy)))
  (- (+ (* era 146097) doe) 719468))))))))

; The same days in alerts-obsidian/src/active.rs:34-42, which writes the month shift as a modulo.
(define-fun days-obsidian ((y0 Int) (m Int) (d Int)) Int
  (let ((y (ite (<= m 2) (- y0 1) y0)))
  (let ((era (div y 400)))
  (let ((yoe (- y (* era 400))))
  (let ((mp (mod (+ m 9) 12)))
  (let ((doy (- (+ (div (+ (* 153 mp) 2) 5) d) 1)))
  (let ((doe (+ (* yoe 365) (div yoe 4) (- (div yoe 100)) doy)))
  (- (+ (* era 146097) doe) 719468))))))))

; rfc3339_utc (time.rs:15-29): the civil date of a day number n.
(define-fun c-doe ((n Int)) Int (- (+ n 719468) (* (div (+ n 719468) 146097) 146097)))
(define-fun c-yoe ((n Int)) Int
  (let ((doe (c-doe n))) (div (- (+ (- doe (div doe 1460)) (div doe 36524)) (div doe 146096)) 365)))
(define-fun c-doy ((n Int)) Int
  (let ((doe (c-doe n)) (yoe (c-yoe n))) (- doe (- (+ (* 365 yoe) (div yoe 4)) (div yoe 100)))))
(define-fun c-mp ((n Int)) Int (div (+ (* 5 (c-doy n)) 2) 153))
(define-fun civ-d ((n Int)) Int (+ (- (c-doy n) (div (+ (* 153 (c-mp n)) 2) 5)) 1))
(define-fun civ-m ((n Int)) Int (ite (< (c-mp n) 10) (+ (c-mp n) 3) (- (c-mp n) 9)))
(define-fun civ-y ((n Int)) Int
  (+ (c-yoe n) (* (div (+ n 719468) 146097) 400) (ite (<= (civ-m n) 2) 1 0)))

; The Gregorian calendar, written from its definition, not from the code.
(define-fun leap ((y Int)) Bool (and (= (mod y 4) 0) (or (distinct (mod y 100) 0) (= (mod y 400) 0))))
(define-fun month-len ((y Int) (m Int)) Int
  (ite (= m 2) (ite (leap y) 29 28) (ite (or (= m 4) (= m 6) (= m 9) (= m 11)) 30 31)))
(define-fun date ((y Int) (m Int) (d Int)) Bool (and (<= 1 m 12) (<= 1 d (month-len y m))))
(define-fun lex3 ((a1 Int) (a2 Int) (a3 Int) (b1 Int) (b2 Int) (b3 Int)) Bool
  (or (< a1 b1) (and (= a1 b1) (or (< a2 b2) (and (= a2 b2) (< a3 b3))))))

; What epoch (time.rs:42-51) accepts.
(define-fun accepted-date ((m Int) (d Int)) Bool (and (<= 1 m 12) (<= 1 d 31)))
(define-fun accepted-clock ((h Int) (mi Int) (s Int)) Bool (and (<= 0 h 23) (<= 0 mi 59) (<= 0 s 60)))

(declare-const n Int)
(declare-const y Int) (declare-const m Int) (declare-const d Int)
(declare-const y2 Int) (declare-const m2 Int) (declare-const d2 Int)

; civil_round_trip (crates/alerts-rss/src/time.rs:15-41): days_from_civil undoes the civil date of rfc3339_utc, for every integer day
(push)
(assert (not (= (days (civ-y n) (civ-m n) (civ-d n)) n)))
(check-sat)
(pop)

; civil_is_a_calendar_date (crates/alerts-rss/src/time.rs:15-29): the month of rfc3339_utc is 1 to 12 and the day fits the month, leap years included
(push)
(assert (not (date (civ-y n) (civ-m n) (civ-d n))))
(check-sat)
(pop)

; days_strictly_monotone (crates/alerts-rss/src/time.rs:32-41): a later calendar date has a larger day number, so two dates never share one
(push)
(assert (and (date y m d) (date y2 m2 d2) (lex3 y m d y2 m2 d2)))
(assert (not (< (days y m d) (days y2 m2 d2))))
(check-sat)
(pop)

; utc_text_order_is_time_order (crates/alerts-rss/src/time.rs:1-4): from the three lemmas above: a later day prints a later date (the module comment)
(push)
(declare-fun D (Int Int Int) Int)
(declare-const a Int) (declare-const b Int)
(assert (< a b))
(assert (and (date y m d) (date y2 m2 d2)))
(assert (and (= (D y m d) a) (= (D y2 m2 d2) b)))
(assert (=> (lex3 y m d y2 m2 d2) (< (D y m d) (D y2 m2 d2))))
(assert (=> (lex3 y2 m2 d2 y m d) (< (D y2 m2 d2) (D y m d))))
(assert (not (lex3 y m d y2 m2 d2)))
(check-sat)
(pop)

; obsidian_epoch_agrees (crates/alerts-obsidian/src/active.rs:34-42): the Obsidian plugin counts the same days as alerts-rss, for every month 1 to 12
(push)
(assert (and (<= 1 m 12) (distinct (days y m d) (days-obsidian y m d))))
(check-sat)
(pop)

; accepted_dates_are_dates (crates/alerts-rss/src/time.rs:45-46): every date that epoch accepts is a calendar date
; expect: sat (epoch checks 1 <= d <= 31 for every month: 2026-02-31 is accepted and becomes 2026-03-03 (W1))
(push)
(assert (and (accepted-date m d) (not (date y m d))))
(check-sat)
(pop)

; accepted_clock_within_the_day (crates/alerts-rss/src/time.rs:47-49): every clock time that epoch accepts is less than one day
; expect: sat (the leap second 23:59:60 is accepted and is 00:00:00 of the next day (W2))
(push)
(declare-const h Int) (declare-const mi Int) (declare-const s Int)
(assert (and (accepted-clock h mi s) (>= (+ (* h 3600) (* mi 60) s) 86400)))
(check-sat)
(pop)

; offset_within_a_day (crates/alerts-rss/src/time.rs:76-84): every RFC 3339 offset hh:mm that parse_rfc3339 accepts is within -23:59..+23:59
; expect: sat (the offset is not range-checked: +99:99 is accepted, and +00:60 equals +01:00 (W3))
(push)
(declare-const hh Int) (declare-const mm Int)
(assert (and (<= 0 hh 99) (<= 0 mm 99)))
(assert (> (+ (* hh 60) mm) (+ (* 23 60) 59)))
(check-sat)
(pop)

; utc_year_has_four_digits (crates/alerts-rss/src/time.rs:61-66, 28): every date parse_rfc3339 accepts prints with a four-digit year, so the text order holds
; expect: sat (the year 10000 is accepted; its text sorts before 2026, so `since` and the canonical order misjudge it (W4))
(push)
(assert (and (>= y 0) (date y m d) (= n (days y m d))))
(assert (not (<= 0 (civ-y n) 9999)))
(check-sat)
(pop)
