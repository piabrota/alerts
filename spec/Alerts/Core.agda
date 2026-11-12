------------------------------------------------------------------------
-- alerts: the domain and the FP harness, as a specification.
--
-- This module is the reference of haskell/alerts-core (Alerts.Core). Every
-- definition here has a twin of the same name there; Alerts.Properties
-- proves the facts that the Haskell test suite replays on the real code.
------------------------------------------------------------------------

module Alerts.Core where

open import Data.Bool.Base using (Bool; true; false; if_then_else_; _∧_; _∨_; not)
open import Data.Char.Base using (Char; isDigit; isSpace)
open import Data.List.Base using (List; []; _∷_; _++_; map; length; reverse; dropWhileᵇ; mapMaybe)
open import Data.Bool.ListAction using (any)
open import Data.Maybe.Base using (Maybe; just; nothing; _>>=_)
open import Data.Nat.Base using (ℕ; _≡ᵇ_; _≤ᵇ_; _*_; _+_; _∸_)
open import Data.String.Base using (String)
open import Function.Base using (_∘_)
import Data.Char.Base as Char

------------------------------------------------------------------------
-- UF: the 27 federative units, and their IBGE codes.

data Uf : Set where
  AC AL AP AM BA CE DF ES GO MA MT MS MG PA PB PR PE PI RJ RN RS RO RR SC SP SE TO : Uf

allUfs : List Uf
allUfs = AC ∷ AL ∷ AP ∷ AM ∷ BA ∷ CE ∷ DF ∷ ES ∷ GO ∷ MA ∷ MT ∷ MS ∷ MG ∷ PA ∷
         PB ∷ PR ∷ PE ∷ PI ∷ RJ ∷ RN ∷ RS ∷ RO ∷ RR ∷ SC ∷ SP ∷ SE ∷ TO ∷ []

-- The states v0 serves: the South region.
v0 : List Uf
v0 = PR ∷ SC ∷ RS ∷ []

-- The two-digit IBGE code of a UF.
ibgeCode : Uf → ℕ
ibgeCode RO = 11 ; ibgeCode AC = 12 ; ibgeCode AM = 13 ; ibgeCode RR = 14
ibgeCode PA = 15 ; ibgeCode AP = 16 ; ibgeCode TO = 17
ibgeCode MA = 21 ; ibgeCode PI = 22 ; ibgeCode CE = 23 ; ibgeCode RN = 24
ibgeCode PB = 25 ; ibgeCode PE = 26 ; ibgeCode AL = 27 ; ibgeCode SE = 28
ibgeCode BA = 29
ibgeCode MG = 31 ; ibgeCode ES = 32 ; ibgeCode RJ = 33 ; ibgeCode SP = 35
ibgeCode PR = 41 ; ibgeCode SC = 42 ; ibgeCode RS = 43
ibgeCode MS = 50 ; ibgeCode MT = 51 ; ibgeCode GO = 52 ; ibgeCode DF = 53

-- The first element that satisfies p.
find : {A : Set} → (A → Bool) → List A → Maybe A
find p []       = nothing
find p (x ∷ xs) = if p x then just x else find p xs

-- The UF of a two-digit prefix: the inverse of ibgeCode, read off it.
ufOfPrefix : ℕ → Maybe Uf
ufOfPrefix p = find (λ u → ibgeCode u ≡ᵇ p) allUfs

digitValue : Char → ℕ
digitValue c = Char.toℕ c ∸ Char.toℕ '0'

trim : List Char → List Char
trim = reverse ∘ dropWhileᵇ isSpace ∘ reverse ∘ dropWhileᵇ isSpace

allDigits : List Char → Bool
allDigits []       = true
allDigits (c ∷ cs) = isDigit c ∧ allDigits cs

-- A code is well formed when it has 2 or 7 digits after trimming.
wellFormed : List Char → Bool
wellFormed cs = allDigits cs ∧ ((length cs ≡ᵇ 2) ∨ (length cs ≡ᵇ 7))

prefix2 : List Char → ℕ
prefix2 (a ∷ b ∷ _) = 10 * digitValue a + digitValue b
prefix2 _           = 0

-- The UF of an IBGE code: a 2-digit UF code or a 7-digit municipality code.
fromIbge : List Char → Maybe Uf
fromIbge raw = let cs = trim raw in
  if wellFormed cs then ufOfPrefix (prefix2 cs) else nothing

------------------------------------------------------------------------
-- Severity, ordered from lowest to highest, and hazards.

data Severity : Set where
  Minor Moderate Severe Extreme : Severity

allSeverities : List Severity
allSeverities = Minor ∷ Moderate ∷ Severe ∷ Extreme ∷ []

rank : Severity → ℕ
rank Minor = 0 ; rank Moderate = 1 ; rank Severe = 2 ; rank Extreme = 3

_≤ˢ_ : Severity → Severity → Bool
a ≤ˢ b = rank a ≤ᵇ rank b

data Hazard : Set where
  Flood Landslide : Hazard

------------------------------------------------------------------------
-- Alerts.

record Alert : Set where
  constructor mkAlert
  field
    alertId  : List Char
    source   : List Char
    hazard   : Hazard
    severity : Severity
    ufs      : List Uf
    headline : String
    area     : String
    sent     : String
    url      : Maybe String
open Alert public

------------------------------------------------------------------------
-- The FP harness: a stage passes an alert on, maybe changed, or drops it.

Stage : Set
Stage = Alert → Maybe Alert

-- An ordered composition of stages. The empty harness passes every alert.
Harness : Set
Harness = List Stage

run : Harness → Alert → Maybe Alert
run []      a = just a
run (s ∷ h) a = s a >>= run h

-- A stage that keeps or drops, never changes.
keepIf : (Alert → Bool) → Stage
keepIf p a = if p a then just a else nothing

elemUf : Uf → List Uf → Bool
elemUf u = any (λ v → ibgeCode u ≡ᵇ ibgeCode v)

touches : List Uf → Alert → Bool
touches keep a = any (λ u → elemUf u keep) (ufs a)

-- Keeps alerts that touch at least one of keep.
onlyUfs : List Uf → Stage
onlyUfs keep = keepIf (touches keep)

-- Keeps alerts at min severity or above.
minSeverity : Severity → Stage
minSeverity m = keepIf (λ a → m ≤ˢ severity a)

-- A channel delivers what the harness passes, from a batch of alerts.
deliver : Harness → List Alert → List Alert
deliver h = mapMaybe (run h)

------------------------------------------------------------------------
-- The CAP identifier: sender, source id and alert id joined by '-'.

sender : List Char
sender = 'p' ∷ 'i' ∷ 'a' ∷ 'b' ∷ 'r' ∷ 'o' ∷ 't' ∷ 'a' ∷ '-' ∷
         'a' ∷ 'l' ∷ 'e' ∷ 'r' ∷ 't' ∷ 's' ∷ []

identChars : List Char → List Char → List Char
identChars src i = sender ++ '-' ∷ src ++ '-' ∷ i

-- CAP forbids whitespace and , < > & " ' in an identifier: they become '_'.
forbidden : Char → Bool
forbidden c = isSpace c ∨ any (λ f → Char.toℕ c ≡ᵇ Char.toℕ f) (',' ∷ '<' ∷ '>' ∷ '&' ∷ '"' ∷ '\'' ∷ [])

capSafe : Char → Char
capSafe c = if forbidden c then '_' else c

identifier : Alert → List Char
identifier a = map capSafe (identChars (source a) (alertId a))
