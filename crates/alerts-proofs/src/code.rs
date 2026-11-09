//! Statements about the code itself, not a model of it: each one names the function and the
//! lines it is about, and restates their arithmetic or their logic in SMT-LIB, for every input.
//! The tests of the crates show that the code works on the inputs that ran; these show a fact
//! for every input, or find the input where the fact fails.
//!
//! - P4, feed times (`alerts-rss/src/time.rs`, `alerts-obsidian/src/active.rs`): the calendar
//!   arithmetic is exact, and four inputs that the parsers accept and should not.
//! - P5, the rule algebra (`alerts-rss/src/expr.rs`) and the alert predicates: the laws of the
//!   module comment, one of which fails on a key clash.
//! - P6, the CAP identifier (`alerts-feed/src/lib.rs`): unique only under a hypothesis on the
//!   source ids and the alert ids, which every catalog id keeps (check C7).
//!
//! `checks.rs` replays every counterexample through the real Rust (checks W1 to W6), so a
//! counterexample here is a fact about the code, not about this model of it.

use crate::refute::{Expect, Statement, Theory};

/// P4: the calendar of the feed times.
pub fn time_theory() -> Theory {
    let prelude = "\
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
"
    .to_owned();

    let statements = vec![
        Statement {
            name: "civil_round_trip",
            code: "crates/alerts-rss/src/time.rs:15-41",
            says: "days_from_civil undoes the civil date of rfc3339_utc, for every integer day",
            smt: "(assert (not (= (days (civ-y n) (civ-m n) (civ-d n)) n)))".into(),
            expect: Expect::Unsat,
        },
        Statement {
            name: "civil_is_a_calendar_date",
            code: "crates/alerts-rss/src/time.rs:15-29",
            says: "the month of rfc3339_utc is 1 to 12 and the day fits the month, leap years included",
            smt: "(assert (not (date (civ-y n) (civ-m n) (civ-d n))))".into(),
            expect: Expect::Unsat,
        },
        Statement {
            name: "days_strictly_monotone",
            code: "crates/alerts-rss/src/time.rs:32-41",
            says: "a later calendar date has a larger day number, so two dates never share one",
            smt: "(assert (and (date y m d) (date y2 m2 d2) (lex3 y m d y2 m2 d2)))\n\
                  (assert (not (< (days y m d) (days y2 m2 d2))))"
                .into(),
            expect: Expect::Unsat,
        },
        Statement {
            name: "utc_text_order_is_time_order",
            code: "crates/alerts-rss/src/time.rs:1-4",
            says: "from the three lemmas above: a later day prints a later date (the module comment)",
            // An uninterpreted D stands for `days`; the premises are the instances of the three
            // lemmas that the argument uses, so z3 checks the argument, not the arithmetic again.
            smt: "(declare-fun D (Int Int Int) Int)\n\
                  (declare-const a Int) (declare-const b Int)\n\
                  (assert (< a b))\n\
                  (assert (and (date y m d) (date y2 m2 d2)))\n\
                  (assert (and (= (D y m d) a) (= (D y2 m2 d2) b)))\n\
                  (assert (=> (lex3 y m d y2 m2 d2) (< (D y m d) (D y2 m2 d2))))\n\
                  (assert (=> (lex3 y2 m2 d2 y m d) (< (D y2 m2 d2) (D y m d))))\n\
                  (assert (not (lex3 y m d y2 m2 d2)))"
                .into(),
            expect: Expect::Unsat,
        },
        Statement {
            name: "obsidian_epoch_agrees",
            code: "crates/alerts-obsidian/src/active.rs:34-42",
            says: "the Obsidian plugin counts the same days as alerts-rss, for every month 1 to 12",
            smt: "(assert (and (<= 1 m 12) (distinct (days y m d) (days-obsidian y m d))))".into(),
            expect: Expect::Unsat,
        },
        Statement {
            name: "accepted_dates_are_dates",
            code: "crates/alerts-rss/src/time.rs:45-46",
            says: "every date that epoch accepts is a calendar date",
            smt: "(assert (and (accepted-date m d) (not (date y m d))))".into(),
            expect: Expect::Sat(
                "epoch checks 1 <= d <= 31 for every month: 2026-02-31 is accepted and becomes 2026-03-03 (W1)",
            ),
        },
        Statement {
            name: "accepted_clock_within_the_day",
            code: "crates/alerts-rss/src/time.rs:47-49",
            says: "every clock time that epoch accepts is less than one day",
            smt: "(declare-const h Int) (declare-const mi Int) (declare-const s Int)\n\
                  (assert (and (accepted-clock h mi s) (>= (+ (* h 3600) (* mi 60) s) 86400)))"
                .into(),
            expect: Expect::Sat(
                "the leap second 23:59:60 is accepted and is 00:00:00 of the next day (W2)",
            ),
        },
        Statement {
            name: "offset_within_a_day",
            code: "crates/alerts-rss/src/time.rs:76-84",
            says: "every RFC 3339 offset hh:mm that parse_rfc3339 accepts is within -23:59..+23:59",
            // `num` accepts any run of digits; two digits each is the RFC 3339 form.
            smt: "(declare-const hh Int) (declare-const mm Int)\n\
                  (assert (and (<= 0 hh 99) (<= 0 mm 99)))\n\
                  (assert (> (+ (* hh 60) mm) (+ (* 23 60) 59)))"
                .into(),
            expect: Expect::Sat(
                "the offset is not range-checked: +99:99 is accepted, and +00:60 equals +01:00 (W3)",
            ),
        },
        Statement {
            name: "utc_year_has_four_digits",
            code: "crates/alerts-rss/src/time.rs:61-66, 28",
            says: "every date parse_rfc3339 accepts prints with a four-digit year, so the text order holds",
            // `num` reads the year as any run of digits; `{y:04}` pads to at least four.
            smt: "(assert (and (>= y 0) (date y m d) (= n (days y m d))))\n\
                  (assert (not (<= 0 (civ-y n) 9999)))"
                .into(),
            expect: Expect::Sat(
                "the year 10000 is accepted; its text sorts before 2026, so `since` and the canonical order misjudge it (W4)",
            ),
        },
    ];
    Theory {
        title: "P4: feed times: the calendar arithmetic is exact; four inputs pass that should not",
        prelude,
        statements,
    }
}

/// P5: the rule algebra, key by key, and the alert predicates.
pub fn rules_theory() -> Theory {
    let prelude = "\
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
"
    .to_owned();

    let law = |name, code, says, smt: &str| Statement {
        name,
        code,
        says,
        smt: smt.into(),
        expect: Expect::Unsat,
    };
    let statements = vec![
        law(
            "union_commutative",
            "crates/alerts-rss/src/expr.rs:5, 211-217",
            "union(a, b) = union(b, a)",
            "(assert (not (= (union2 a b) (union2 b a))))",
        ),
        law(
            "union_associative",
            "crates/alerts-rss/src/expr.rs:5, 211-217",
            "union(union(a, b), c) = union(a, union(b, c))",
            "(assert (not (= (union2 (union2 a b) c) (union2 a (union2 b c)))))",
        ),
        law(
            "union_idempotent",
            "crates/alerts-rss/src/expr.rs:5, 211-217",
            "union(a, a) = a",
            "(assert (not (= (union2 a a) a)))",
        ),
        law(
            "union_empty",
            "crates/alerts-rss/src/expr.rs:33",
            "union(a, union []) = a",
            "(assert (not (= (union2 a none) a)))",
        ),
        law(
            "intersect_commutative",
            "crates/alerts-rss/src/expr.rs:5, 218-235",
            "intersect(a, b) = intersect(b, a)",
            "(assert (not (= (inter2 a b) (inter2 b a))))",
        ),
        law(
            "intersect_associative",
            "crates/alerts-rss/src/expr.rs:5, 218-235",
            "intersect(intersect(a, b), c) = intersect(a, intersect(b, c))",
            "(assert (not (= (inter2 (inter2 a b) c) (inter2 a (inter2 b c)))))",
        ),
        law(
            "intersect_idempotent",
            "crates/alerts-rss/src/expr.rs:5, 218-235",
            "intersect(a, a) = a",
            "(assert (not (= (inter2 a a) a)))",
        ),
        law(
            "diff_self_empty",
            "crates/alerts-rss/src/expr.rs:6, 236-241",
            "diff(a, a) is empty",
            "(assert (not (= (diff a a) none)))",
        ),
        law(
            "diff_drops_right",
            "crates/alerts-rss/src/expr.rs:6, 236-241",
            "diff(a, b) never holds a key of b",
            "(assert (and (is-some b) (is-some (diff a b))))",
        ),
        law(
            "diff_of_union",
            "crates/alerts-rss/src/expr.rs:236-241",
            "diff(a, union(b, c)) = diff(diff(a, b), c)",
            "(assert (not (= (diff a (union2 b c)) (diff (diff a b) c))))",
        ),
        law(
            "filter_filter",
            "crates/alerts-rss/src/expr.rs:8, 242-247, 338",
            "filter(p, filter(q, a)) = filter(all[p, q], a)",
            "(assert (not (= (filter-p (filter-q a)) (filter-pq a))))",
        ),
        law(
            "filter_of_diff",
            "crates/alerts-rss/src/expr.rs:236-247",
            "filter(p, diff(a, b)) = diff(filter(p, a), b)",
            "(assert (not (= (filter-p (diff a b)) (diff (filter-p a) b))))",
        ),
        Statement {
            name: "filter_union_distributes",
            code: "crates/alerts-rss/src/expr.rs:9",
            says: "filter(p, union(a, b)) = union(filter(p, a), filter(p, b))",
            smt: "(assert (not (= (filter-p (union2 a b)) (union2 (filter-p a) (filter-p b)))))"
                .into(),
            expect: Expect::Sat(
                "a key in both operands whose winner fails p and whose loser passes: the left side drops the key, the right keeps the loser (W5)",
            ),
        },
        law(
            "filter_union_when_items_agree",
            "crates/alerts-rss/src/expr.rs:9",
            "the law of expr.rs:9 holds when p gives the same answer on the two items of a shared key",
            "(assert (=> (and (is-some a) (is-some b)) (= (p (winner a)) (p (winner b)))))\n\
             (assert (not (= (filter-p (union2 a b)) (union2 (filter-p a) (filter-p b)))))",
        ),
        law(
            "ufs_any_is_union",
            "crates/alerts-rss/src/expr.rs:333-339",
            "any[ufs S, ufs T] keeps the alerts of ufs (S union T)",
            "(assert (not (= (or (touches A S) (touches A T)) (touches A (bvor S T)))))",
        ),
        law(
            "ufs_intersection_implies_all",
            "crates/alerts-rss/src/expr.rs:333-338",
            "ufs (S intersect T) keeps only alerts that all[ufs S, ufs T] keeps",
            "(assert (and (touches A (bvand S T)) (not (and (touches A S) (touches A T)))))",
        ),
        Statement {
            name: "ufs_all_is_intersection",
            code: "crates/alerts-rss/src/expr.rs:333-338",
            says: "all[ufs S, ufs T] keeps the alerts of ufs (S intersect T)",
            smt: "(assert (not (= (and (touches A S) (touches A T)) (touches A (bvand S T)))))"
                .into(),
            expect: Expect::Sat(
                "an alert for PR and SP passes all[ufs [PR], ufs [SP]], but ufs [] keeps nothing: `all` of two UF tests asks for each UF list, not for a common UF",
            ),
        },
        law(
            "severity_all_is_max",
            "crates/alerts-rss/src/expr.rs:329-338",
            "all[min_severity lo, min_severity hi] = min_severity (max lo hi)",
            "(assert (and (sev sv) (sev lo) (sev hi)))\n\
             (assert (not (= (and (>= sv lo) (>= sv hi)) (>= sv (ite (>= lo hi) lo hi)))))",
        ),
        law(
            "severity_any_is_min",
            "crates/alerts-rss/src/expr.rs:329-339",
            "any[min_severity lo, min_severity hi] = min_severity (min lo hi)",
            "(assert (and (sev sv) (sev lo) (sev hi)))\n\
             (assert (not (= (or (>= sv lo) (>= sv hi)) (>= sv (ite (<= lo hi) lo hi)))))",
        ),
    ];
    Theory {
        title: "P5: the rule algebra holds its laws key by key, except filter over union",
        prelude,
        statements,
    }
}

/// P6: the CAP identifier `piabrota-alerts-<source>-<id>`.
pub fn identifier_theory() -> Theory {
    let prelude = "\
; identifier (alerts-feed/src/lib.rs:65-78) joins the sender, the source id and the alert id
; with `-`, then replaces the characters CAP forbids. The statements take ids without those
; characters, where the replacement is the identity.
(define-fun ident ((src String) (id String)) String (str.++ \"piabrota-alerts-\" src \"-\" id))
(define-fun alnum () RegLan (re.union (re.range \"a\" \"z\") (re.range \"0\" \"9\")))
; is_slug (alerts-crud/src/validate.rs:62-68): the form of a source id.
(define-fun slug ((s String)) Bool
  (and (str.in_re s (re.+ (re.union alnum (str.to_re \"-\"))))
       (not (str.prefixof \"-\" s)) (not (str.suffixof \"-\" s))))
(define-fun plain ((s String)) Bool
  (str.in_re s (re.* (re.union alnum (str.to_re \"-\") (str.to_re \"_\") (str.to_re \".\") (str.to_re \":\")))))
(declare-const s1 String) (declare-const s2 String) (declare-const i1 String) (declare-const i2 String)
"
    .to_owned();
    let statements = vec![
        Statement {
            name: "identifier_unique",
            code: "crates/alerts-feed/src/lib.rs:65-68",
            says: "two alerts with different (source, id) never share a CAP identifier",
            smt: "(assert (and (slug s1) (slug s2) (plain i1) (plain i2)))\n\
                  (assert (and (str.in_re i1 (re.++ alnum re.all)) (str.in_re i2 (re.++ alnum re.all))))\n\
                  (assert (not (and (= s1 s2) (= i1 i2))))\n\
                  (assert (= (ident s1 i1) (ident s2 i2)))"
                .into(),
            expect: Expect::Sat(
                "the `-` that joins source and id is also a letter of both: the source cemaden with the id ped-1 and the source cemaden-ped (both in the catalog) with the id 1 give one identifier (W6)",
            ),
        },
        Statement {
            name: "identifier_unique_when_ids_start_with_a_digit",
            code: "crates/alerts-feed/src/lib.rs:65-68",
            says: "the identifier is unique when no source id has a `-` before a digit and every alert id starts with a digit",
            // Strings as characters: w is the text after `piabrota-alerts-`, read as w[k] (45 is
            // `-`, 48 to 57 the digits). A split at L gives the source w[0..L) and the id
            // w[L+1..). Two splits L1 < L2 of one text: z3's string solver cannot close this
            // argument, so it runs over the characters.
            smt: "(declare-fun w (Int) Int)\n\
                  (declare-const L1 Int) (declare-const L2 Int)\n\
                  (define-fun digit ((k Int)) Bool (<= 48 (w k) 57))\n\
                  (assert (and (< 0 L1) (< L1 L2)))\n\
                  ; each split is a `-` followed by the first character of an id: a digit\n\
                  (assert (and (= (w L1) 45) (digit (+ L1 1)) (= (w L2) 45) (digit (+ L2 1))))\n\
                  ; the longer source w[0..L2) has no `-` before a digit, and does not end in `-`\n\
                  (assert (forall ((k Int)) (=> (and (<= 0 k) (< (+ k 1) L2)) (not (and (= (w k) 45) (digit (+ k 1)))))))\n\
                  (assert (distinct (w (- L2 1)) 45))"
                .into(),
            expect: Expect::Unsat,
        },
    ];
    Theory {
        title: "P6: the CAP identifier is unique only when the source and alert ids keep a form",
        prelude,
        statements,
    }
}
