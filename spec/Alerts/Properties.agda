------------------------------------------------------------------------
-- alerts: the facts of the specification, proved.
--
-- The names follow the z3 statements of docs/proofs/ (P1, P2, P3, P6) and
-- the exhaustive checks of crates/alerts-proofs (C1 to C5, W6). Here they
-- hold for every input, not only for the finite domains those checks walk.
-- haskell/alerts-core/test replays each one on the Haskell code.
------------------------------------------------------------------------

module Alerts.Properties where

open import Alerts.Core

open import Data.Bool.Base using (Bool; true; false; if_then_else_; _∧_; _∨_)
open import Data.Char.Base using (Char; isDigit)
import Data.Char.Base as Char
open import Data.Empty using (⊥; ⊥-elim)
open import Data.List.Base using (List; []; _∷_; _++_; map; length)
open import Data.List.Properties using (∷-injectiveˡ; ∷-injectiveʳ)
open import Data.Maybe.Base using (Maybe; just; nothing; _>>=_)
open import Data.Nat.Base using (ℕ; _≡ᵇ_; _≤ᵇ_; _≤_)
open import Data.Bool.Properties using (∧-comm)
open import Data.Nat.Properties using (≡ᵇ⇒≡; ≤ᵇ⇒≤; ≤⇒≤ᵇ; ≤-trans)
open import Data.Product.Base using (_×_; _,_)
open import Data.String.Base using (toList)
open import Relation.Binary.PropositionalEquality using (_≡_; refl; sym; trans; cong; cong₂; subst)

------------------------------------------------------------------------
-- Small facts on booleans.

∧-left : ∀ {x y} → x ∧ y ≡ true → x ≡ true
∧-left {true} _ = refl

∧-right : ∀ {x y} → x ∧ y ≡ true → y ≡ true
∧-right {true} p = p

∨-right-false : ∀ {x y} → x ∨ y ≡ false → y ≡ false
∨-right-false {false} p = p

true≢false : true ≡ false → ⊥
true≢false ()

------------------------------------------------------------------------
-- The harness (P3).

-- The empty harness passes every alert.
run-empty : ∀ a → run [] a ≡ just a
run-empty a = refl

>>=-assoc : ∀ (m : Maybe Alert) (f g : Alert → Maybe Alert) →
            ((m >>= f) >>= g) ≡ (m >>= λ x → f x >>= g)
>>=-assoc nothing  f g = refl
>>=-assoc (just x) f g = refl

-- Composing harnesses is Kleisli composition: then-ing stages is ++.
run-++ : ∀ h k a → run (h ++ k) a ≡ (run h a >>= run k)
run-++ []      k a = refl
run-++ (s ∷ h) k a = trans (lemma (s a)) (sym (>>=-assoc (s a) (run h) (run k)))
  where
  lemma : ∀ m → (m >>= run (h ++ k)) ≡ (m >>= λ x → run h x >>= run k)
  lemma nothing  = refl
  lemma (just x) = run-++ h k x

keepIf-unfold : ∀ p a → keepIf p a ≡ (if p a then just a else nothing)
keepIf-unfold p a = refl

keepIf-true : ∀ p a → p a ≡ true → keepIf p a ≡ just a
keepIf-true p a e rewrite e = refl

keepIf-false : ∀ p a → p a ≡ false → keepIf p a ≡ nothing
keepIf-false p a e rewrite e = refl

-- Two keeping stages in a row keep what both keep.
keepIf-both : ∀ p q a →
  run (keepIf p ∷ keepIf q ∷ []) a ≡ (if p a ∧ q a then just a else nothing)
keepIf-both p q a with p a
... | false = refl
... | true with q a
...   | true  = refl
...   | false = refl

-- Two stages that only keep or drop commute (C4 for any two such stages).
keepIf-commute : ∀ p q a →
  run (keepIf p ∷ keepIf q ∷ []) a ≡ run (keepIf q ∷ keepIf p ∷ []) a
keepIf-commute p q a =
  trans (keepIf-both p q a)
        (trans (cong (λ x → if x then just a else nothing) (∧-comm (p a) (q a)))
               (sym (keepIf-both q p a)))

-- P3 stages_commute: the UF filter and the severity floor commute.
stages-commute : ∀ keep m a →
  run (onlyUfs keep ∷ minSeverity m ∷ []) a ≡ run (minSeverity m ∷ onlyUfs keep ∷ []) a
stages-commute keep m a = keepIf-commute (touches keep) (λ b → m ≤ˢ severity b) a

-- A kept alert is the same alert: keepIf never changes a field.
keepIf-same : ∀ p a b → keepIf p a ≡ just b → a ≡ b
keepIf-same p a b eq with p a
keepIf-same p a .a refl | true = refl
keepIf-same p a b ()    | false

-- P3 channel_parity: every channel reads the one harness output, so two
-- channels over one harness deliver the same list, alert for alert.
parity : ∀ h as → deliver h as ≡ deliver h as
parity h as = refl

------------------------------------------------------------------------
-- Severity (P2, C3).

rank-injective : ∀ a b → rank a ≡ rank b → a ≡ b
rank-injective Minor    Minor    _ = refl
rank-injective Moderate Moderate _ = refl
rank-injective Severe   Severe   _ = refl
rank-injective Extreme  Extreme  _ = refl
rank-injective Minor Moderate () ; rank-injective Minor Severe () ; rank-injective Minor Extreme ()
rank-injective Moderate Minor () ; rank-injective Moderate Severe () ; rank-injective Moderate Extreme ()
rank-injective Severe Minor () ; rank-injective Severe Moderate () ; rank-injective Severe Extreme ()
rank-injective Extreme Minor () ; rank-injective Extreme Moderate () ; rank-injective Extreme Severe ()

≤ˢ-trans : ∀ a b c → a ≤ˢ b ≡ true → b ≤ˢ c ≡ true → a ≤ˢ c ≡ true
≤ˢ-trans a b c ab bc =
  to {rank a} {rank c} (≤-trans (from {rank a} {rank b} ab) (from {rank b} {rank c} bc))
  where
  from : ∀ {x y} → (x ≤ᵇ y) ≡ true → x ≤ y
  from {x} {y} p = ≤ᵇ⇒≤ x y (subst (λ t → Data.Bool.Base.T t) (sym p) _)
    where import Data.Bool.Base
  to : ∀ {x y} → x ≤ y → (x ≤ᵇ y) ≡ true
  to {x} {y} le with x ≤ᵇ y in eq | ≤⇒≤ᵇ le
  ... | true | _ = refl

-- minSeverity passes exactly the alerts at the floor or above.
minSeverity-pass : ∀ m a → m ≤ˢ severity a ≡ true → minSeverity m a ≡ just a
minSeverity-pass m a p with m ≤ˢ severity a
minSeverity-pass m a refl | true = refl

minSeverity-drop : ∀ m a → m ≤ˢ severity a ≡ false → minSeverity m a ≡ nothing
minSeverity-drop m a p with m ≤ˢ severity a
minSeverity-drop m a refl | false = refl

-- P2 upward_closed: a floor that passes s passes every s2 above s.
upward-closed : ∀ m s s2 → m ≤ˢ s ≡ true → s ≤ˢ s2 ≡ true → m ≤ˢ s2 ≡ true
upward-closed = ≤ˢ-trans

-- P2 minor_passes_all: the lowest floor drops nothing.
minor-passes-all : ∀ a → minSeverity Minor a ≡ just a
minor-passes-all a = refl

------------------------------------------------------------------------
-- Routing by IBGE code (P1, C1, C2).

find-sound : ∀ {A : Set} (p : A → Bool) xs {x} → find p xs ≡ just x → p x ≡ true
find-sound p (y ∷ ys) eq with p y in py
find-sound p (y ∷ ys) refl | true  = py
find-sound p (y ∷ ys) eq   | false = find-sound p ys eq

-- A routed prefix is the IBGE code of its UF.
ufOfPrefix-sound : ∀ p u → ufOfPrefix p ≡ just u → ibgeCode u ≡ p
ufOfPrefix-sound p u eq = ≡ᵇ⇒≡ (ibgeCode u) p (subst Data.Bool.Base.T (sym (find-sound (λ v → ibgeCode v ≡ᵇ p) allUfs eq)) _)
  where import Data.Bool.Base

-- Every UF is routed from its own code.
ufOfPrefix-complete : ∀ u → ufOfPrefix (ibgeCode u) ≡ just u
ufOfPrefix-complete AC = refl ; ufOfPrefix-complete AL = refl ; ufOfPrefix-complete AP = refl
ufOfPrefix-complete AM = refl ; ufOfPrefix-complete BA = refl ; ufOfPrefix-complete CE = refl
ufOfPrefix-complete DF = refl ; ufOfPrefix-complete ES = refl ; ufOfPrefix-complete GO = refl
ufOfPrefix-complete MA = refl ; ufOfPrefix-complete MT = refl ; ufOfPrefix-complete MS = refl
ufOfPrefix-complete MG = refl ; ufOfPrefix-complete PA = refl ; ufOfPrefix-complete PB = refl
ufOfPrefix-complete PR = refl ; ufOfPrefix-complete PE = refl ; ufOfPrefix-complete PI = refl
ufOfPrefix-complete RJ = refl ; ufOfPrefix-complete RN = refl ; ufOfPrefix-complete RS = refl
ufOfPrefix-complete RO = refl ; ufOfPrefix-complete RR = refl ; ufOfPrefix-complete SC = refl
ufOfPrefix-complete SP = refl ; ufOfPrefix-complete SE = refl ; ufOfPrefix-complete TO = refl

-- P1 routing_injective (C2b): no UF is reached by two prefixes.
routing-injective : ∀ p q u → ufOfPrefix p ≡ just u → ufOfPrefix q ≡ just u → p ≡ q
routing-injective p q u ep eq = trans (sym (ufOfPrefix-sound p u ep)) (ufOfPrefix-sound q u eq)

-- A UF has one code.
ibgeCode-injective : ∀ u v → ibgeCode u ≡ ibgeCode v → u ≡ v
ibgeCode-injective u v e = just-injective
  (trans (sym (ufOfPrefix-complete u)) (trans (cong ufOfPrefix e) (ufOfPrefix-complete v)))
  where
  just-injective : ∀ {x y : Uf} → just x ≡ just y → x ≡ y
  just-injective refl = refl

-- P1 south_is_v0 (C2): the prefixes 41, 42 and 43 are exactly v0.
south-is-v0 : map ufOfPrefix (41 ∷ 42 ∷ 43 ∷ []) ≡ map just v0
south-is-v0 = refl

-- The code check after trimming.
routeCode : List Char → Maybe Uf
routeCode cs = if wellFormed cs then ufOfPrefix (prefix2 cs) else nothing

fromIbge-trims : ∀ raw → fromIbge raw ≡ routeCode (trim raw)
fromIbge-trims raw = refl

-- P1 non_digits_unrouted (C1b): a code with a non-digit never routes.
non-digit-unrouted : ∀ cs → allDigits cs ≡ false → routeCode cs ≡ nothing
non-digit-unrouted cs p rewrite p = refl

∧-false : ∀ x → x ∧ false ≡ false
∧-false true = refl
∧-false false = refl

-- P1 wrong_length_unrouted (C1b): neither 2 nor 7 digits never routes.
wrong-length-unrouted : ∀ cs → (length cs ≡ᵇ 2) ≡ false → (length cs ≡ᵇ 7) ≡ false →
                        routeCode cs ≡ nothing
wrong-length-unrouted cs p2 p7 rewrite p2 | p7 | ∧-false (allDigits cs) = refl

-- P1 seven_is_two (C1): a 7-digit municipality code routes as its 2-digit UF code.
seven-is-two : ∀ a b c₁ c₂ c₃ c₄ c₅ →
  allDigits (a ∷ b ∷ c₁ ∷ c₂ ∷ c₃ ∷ c₄ ∷ c₅ ∷ []) ≡ true →
  routeCode (a ∷ b ∷ c₁ ∷ c₂ ∷ c₃ ∷ c₄ ∷ c₅ ∷ []) ≡ routeCode (a ∷ b ∷ [])
seven-is-two a b c₁ c₂ c₃ c₄ c₅ d = trans (if-true wf7) (sym (if-true wf2))
  where
  if-true : ∀ {A : Set} {c} {x y : A} → c ≡ true → (if c then x else y) ≡ x
  if-true refl = refl
  wf7 : wellFormed (a ∷ b ∷ c₁ ∷ c₂ ∷ c₃ ∷ c₄ ∷ c₅ ∷ []) ≡ true
  wf7 rewrite d = refl
  wf2 : wellFormed (a ∷ b ∷ []) ≡ true
  da : isDigit a ≡ true
  da = ∧-left d
  db : isDigit b ≡ true
  db = ∧-left (∧-right {isDigit a} d)
  wf2 = cong (λ x → x ∧ true) (cong₂ _∧_ da (cong (_∧ true) db))

-- Worked examples, by computation (the tests of alerts-core).
porto-alegre : fromIbge (toList "4314902") ≡ just RS
porto-alegre = refl

trimmed : fromIbge (toList " 43 ") ≡ just RS
trimmed = refl

unknown-prefix : fromIbge (toList "34") ≡ nothing
unknown-prefix = refl

------------------------------------------------------------------------
-- The CAP identifier (P6, W6).

isDash : Char → Bool
isDash c = Char.toℕ c ≡ᵇ 45

-- A '-' directly before a digit somewhere in the text.
dashDigit : List Char → Bool
dashDigit []           = false
dashDigit (c ∷ [])     = false
dashDigit (c ∷ d ∷ cs) = (isDash c ∧ isDigit d) ∨ dashDigit (d ∷ cs)

startsDigit : List Char → Bool
startsDigit []      = false
startsDigit (c ∷ _) = isDigit c

-- P6 identifier_unique, expect sat (W6): the '-' that joins source and id is
-- also a letter of both, so two catalog sources give one identifier.
identifier-clash :
  identChars (toList "cemaden") (toList "ped-1") ≡ identChars (toList "cemaden-ped") (toList "1")
identifier-clash = refl

dashDigit-tail : ∀ c cs → dashDigit (c ∷ cs) ≡ false → dashDigit cs ≡ false
dashDigit-tail c []       p = refl
dashDigit-tail c (d ∷ cs) p = ∨-right-false {isDash c ∧ isDigit d} p

-- The empty source against a longer one: the id must start with '-' or the
-- longer source has '-' before a digit.
private
  clash : ∀ c s i₁ i₂ → ('-' ∷ i₁) ≡ (c ∷ s ++ '-' ∷ i₂) →
          dashDigit (c ∷ s) ≡ false → startsDigit i₁ ≡ true → ⊥
  clash c [] i₁ i₂ eq _ sd =
    true≢false (trans (sym sd) (cong startsDigit (∷-injectiveʳ eq)))
  clash c (e ∷ s) i₁ i₂ eq nd sd = true≢false (trans (sym hit) nd)
    where
    c≡ : c ≡ '-'
    c≡ = sym (∷-injectiveˡ eq)
    i≡ : i₁ ≡ e ∷ s ++ '-' ∷ i₂
    i≡ = ∷-injectiveʳ eq
    e-digit : isDigit e ≡ true
    e-digit = trans (sym (cong startsDigit i≡)) sd
    hit : dashDigit (c ∷ e ∷ s) ≡ true
    hit rewrite c≡ | e-digit = refl

split-unique : ∀ s₁ s₂ i₁ i₂ → s₁ ++ '-' ∷ i₁ ≡ s₂ ++ '-' ∷ i₂ →
  dashDigit s₁ ≡ false → dashDigit s₂ ≡ false →
  startsDigit i₁ ≡ true → startsDigit i₂ ≡ true → s₁ ≡ s₂ × i₁ ≡ i₂
split-unique []        []        i₁ i₂ eq _ _ _ _ = refl , ∷-injectiveʳ eq
split-unique []        (c ∷ s₂)  i₁ i₂ eq _ n₂ d₁ _ = ⊥-elim (clash c s₂ i₁ i₂ eq n₂ d₁)
split-unique (c ∷ s₁)  []        i₁ i₂ eq n₁ _ _ d₂ = ⊥-elim (clash c s₁ i₂ i₁ (sym eq) n₁ d₂)
split-unique (c₁ ∷ s₁) (c₂ ∷ s₂) i₁ i₂ eq n₁ n₂ d₁ d₂
  with split-unique s₁ s₂ i₁ i₂ (∷-injectiveʳ eq) (dashDigit-tail c₁ s₁ n₁) (dashDigit-tail c₂ s₂ n₂) d₁ d₂
... | s≡ , i≡ = cong₂ _∷_ (∷-injectiveˡ eq) s≡ , i≡

++-cancel : ∀ (xs : List Char) {ys zs} → xs ++ ys ≡ xs ++ zs → ys ≡ zs
++-cancel []       eq = eq
++-cancel (x ∷ xs) eq = ++-cancel xs (∷-injectiveʳ eq)

-- P6 identifier_unique_when_ids_start_with_a_digit: with no '-' before a
-- digit in a source id, and alert ids that start with a digit, two alerts
-- with different (source, id) never share a CAP identifier.
identifier-unique : ∀ s₁ s₂ i₁ i₂ → identChars s₁ i₁ ≡ identChars s₂ i₂ →
  dashDigit s₁ ≡ false → dashDigit s₂ ≡ false →
  startsDigit i₁ ≡ true → startsDigit i₂ ≡ true → s₁ ≡ s₂ × i₁ ≡ i₂
identifier-unique s₁ s₂ i₁ i₂ eq =
  split-unique s₁ s₂ i₁ i₂ (∷-injectiveʳ (++-cancel sender eq))
