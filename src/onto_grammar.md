Pinned Onto grammar reference: trigramco/onto at 454af8cb5bf9d6bef725df6a3294a5e54be37894, packages/ontolang/docs/features/ontolang/grammar.md.
This is language reference data. The construction directive defines the narrower supported host profile.

> Portable snapshot of maintained Onto material. [Package capabilities](../../../START_HERE.md) define the shipped boundary; dated decisions and host implementation notes are context. [Source manifest](../../../SOURCE_MANIFEST.json) records provenance. Links marked “Onto source” are optional host/background references.

# ontolang — grammar reference

> **Authority note:** the loader/printer
> are **built to this page** ([spec.md](spec.md) — the parser
> catch-up, 2026-08-29): the parsers and the pinned golden corpus
> ([`examples/ontolang/`](../../../examples/ontolang/) — sangha is the reference; all registered models
> round-trip) are the truth of the grammar; this page is the reading aid. The Rule and Formula
> grammars are normatively defined in [rule-dsl](../rule-dsl/spec.md); the Story sentence grammar
> in [story-authoring](../story-authoring/spec.md). Decisions and their reasons:
> [backlog.md](backlog.md) (the 2026-08-24 walk, the 2026-08-26/27 verification rulings, and the
> 2026-08-29 session rulings — declaration annotations, `is a` + `@where` subsets, `some` + the
> trailing `in <[]-field>` restrictor, written birth) and [binding-backlog.md](binding-backlog.md)
> (clauses, `finds`, `about`, `Type:label`, mirror pairing).

## Actor policy extension — built locally

**Owner ruling, 2026-09-10:** actor declarations separate organizational affiliation, application
access and role enrollment. The [backlog ruling](backlog.md#actor-access-and-enrollment--ratified-2026-09-10)
and [build plan](actor-boundary-plan.md#language-contract) own the decision and migration. The
parser/printer support these annotations. Complete entry requirements and legacy
private-QA compatibility are described in the [spec](spec.md#actor-policy--built-locally-2026-10-10).

```ebnf
affiliation-annotation ::= "@affiliation" ("internal" | "external")
access-annotation      ::= "@access" ("authenticated" | "public" | "none")
signup-annotation      ::= "@signup" ("self" | "invite" | "none")
```

Affiliation and access must be explicit before participant entry, including roles and subsets.
Legacy authoring/private QA may omit them with a notice. Affiliation is separate from
`@boundary external`, which remains the system-execution ownership flag; organizational
affiliation does not authenticate an actor, bypass role eligibility or decide automatic execution.
Authenticated access requires verified identity and role eligibility; public access requires no
login; none allows no direct application access as that actor.

Signup is actor-scoped role enrollment. `self` permits self-service and invitations; `invite`
requires an invitation for enrollment; `none` disables both. All three permit authorized direct
grants to existing people. An existing account is reused, and the mode does not name an inviter or
grant authority. Bare `@signup` and boolean `@canSignup` are not aliases. Authenticated person entry requires explicit signup. Other actor kinds and derived
subsets cannot use self/invite enrollment. Missing authoring metadata remains missing;
the implementation never infers openness from absent seeds or grant stories.

```onto
actor Student
  @type person
  @affiliation external
  @access authenticated
  @signup self

actor TeacherAssistant is a Student
  @affiliation internal
  @access authenticated
  @signup invite

actor OldStudent is a Student
  @affiliation external
  @access authenticated
  @signup none
```

This illustrates the accepted syntax. It is a declaration fragment, not a complete
project; the actual Sangha corpus retains its attendance-derived OldStudent. The [retreat scenario](actor-boundary-plan.md#implemented-migration-choices) covers
the separate booking approval process and administrative grants.

## Lexical conventions (one lexer, all grammars)

| token | meaning |
|---|---|
| `CamelCaseWord` | entity name — compounded, everywhere (RoomClass, ManagingDirector); digits inside names legal |
| `Warehouse.Product` | imported type in a namespace; `Warehouse.Product.sku` accesses its field. Declared types disambiguate existing capitalized field spellings. |
| `lowercaseword` | field name, verb, value, or keyword — meaning is contextual; the parsers decide |
| `"quoted"` | a value, resolved by the field's declared type; values are lowercase (capitals are type names) |
| integer | safe whole number from -9007199254740991 through 9007199254740991; money is scaled minor units within the same range |
| decimal `4.50` | sugar, legal only where the typed context declares a scale (money → ×100); exact-or-error, never rounded |
| `@word` / `@word(value)` | annotation or attribute. Declaration annotations: `@description` · `@type` · `@rule` · `@where` · `@boundary` · `@affiliation` · `@access` · `@signup`. Field attributes (after the full type): `@required` · `@unique` · `@default(value)` · `@of(field)`. Attachment is per attribute kind (2026-08-27): `@final` binds to the **status value it follows** (inside the enum list); `@correction` trails a **beat line** (the correction channel, D10d); everything else binds to its field/block |
| `Type:label` | the same type twice in one beat, told apart by label — the token is used **verbatim** in clauses; beat-scoped |
| `#` | comment to end of line |
| indentation | attachment |

Logic is words, math is symbols, never mixed in one statement. `is an` is accepted wherever `is a` is.

Quoted strings preserve backslashes literally; there are no string escapes.
Rules and Formulas also accept single-quoted strings. A `#` inside either
supported quote style is text. Each enum value appears once, and each seed row
writes each field at most once. See [literal fidelity](spec.md#literal-fidelity--2026-10-10).

## 1 · The container

```ebnf
file        ::= (project | import | entity | story | seed | comment)*

import      ::= "import" Namespace "from" string          (* relative local directory *)
Namespace   ::= CamelCaseWord                            (* at most 24 characters *)
EntityName  ::= Name | Namespace "." EntityName           (* qualified references *)

project     ::= "project" Name
                  [ "@description" string ]

entity      ::= ("actor" | "object") Name [ ("is a" | "is an") EntityName ]
                  [ "@type" ("person" | "organization" | "system") ]   (* actors; omitted = person *)
                  [ "@description" string ]
                  [ "@boundary" "external" ]              (* only when the derived heuristic is wrong *)
                  [ affiliation-annotation ]
                  [ access-annotation ]
                  [ signup-annotation ]
                  [ "@where" predicate ]                  (* flips `is a` into a DERIVED SUBSET *)
                  field*
                  ( "@rule" rule-text )*                  (* type-attached invariant *)

field       ::= name ":" fieldtype attribute* [ "=" formula-text ]     (* `=` = computed *)
                  [ "@description" string ]               (* indented under the field *)
attribute   ::= "@required" | "@unique" | "@default(" literal ")" | "@of(" field ")"
fieldtype   ::= "string" | "int" | "bool" | "money" | "timestamp"
              | value [ "@final" ] ("|" value [ "@final" ])+   (* enum; order = RANK ONLY — birth is
                                                           written in the creating beat; @final marks
                                                           the finished value(s) it follows *)
              | EntityName                                (* link to one — an object OR an actor *)
              | EntityName "[]"                           (* link to many — the MIRROR line, written on
                                                           both sides and paired by the loader; @of(field)
                                                           names the partner when several links join the
                                                           same two types; two []-links between the same
                                                           two types = error (join record); two one-links
                                                           between the same two types pair one-to-one;
                                                           self-pairs inside one block are legal *)

seed        ::= "seed" EntityName
                  ( name ":" value ("," name ":" value)* )*   (* one record per line; `status:` omitted
                                                           fills only on a managed lifecycle; a seed of
                                                           a bare-actor role may write `name` — the
                                                           built-in Person carries it *)
```

**Semantics the EBNF can't say:**
- [Scoped imports](spec.md#scoped-vocabulary-imports--2026-10-10) load definitions
  only. The consumer's local declarations stay unqualified; imported declarations
  cannot be overridden. Generated `imports.json` pins exact source, including
  transitive imports, for offline reopening. Imports add no Stories or seed rows.
- Complete models enforce [inherited field contracts, typed predicates, acyclic
  derivations and documented missing-data semantics](spec.md#complete-model-semantic-validation--2026-10-10).
  These checks add no syntax and do not add recursive inference.
- `is a` on an **object** = subtype (one record, type fixed at creation); on an **actor** = role (base
  instance + assignment edge; a role block carries capability fields and mirror lines — never `name`,
  dates, or statuses). **`is a` + `@where` = derived subset** on either kind: live, read-only, no
  fields, never seeded, never granted — membership recomputes from the predicate (bare fields in the
  predicate are the base's; a trailing `in <[]-field>` scopes an atom to that collection's elements).
- Object parents stay concrete whether or not a Story names them. Declaring a
  subtype does not prohibit direct parent records or invent a creating Story.
- A bare actor IS a role over person (the collapse, 2026-08-18); `@type` is explicit best practice,
  and a role inherits its base chain's declared kind (`actor Business is a Company` under
  `@type organization` is an organization).
- Actors and objects may stand alone, including descriptive `status` enums.
  Unused vocabulary is legal in published applications too; no mode switch is required.
- **Only Story-written status lists are managed lifecycles.** Missing producers,
  endings and routes are authoring advice, enforced at execution compilation and
  publication. Explicit writes out of a `@final` value remain source errors unless
  marked `@correction`. See [workflow readiness](spec.md#standalone-models-and-workflow-readiness--2026-10-10).
- Attributes are THE extension point for field facts. `@unique`: no two instances share a set value
  (unset never collides; exact matching). `@default(value)`: written once at creation; bare `now` is
  the only timestamp default; never on a computed field, never on a **status** enum — birth lives in
  the creating beat.
- Seed field checks resolve through the `is a` chain, so a seed of a ROLE type is legal and stays
  keyed by the role (the runtime mints base + assignment edge). Supplied values must match their
  declared scalar type and enum values; integer values/defaults must be safe integers and timestamp
  strings must name valid calendar dates/times. Money literals scale exactly to minor units.

## 2 · Story sentences

```ebnf
story       ::= "story" Title [ "about" TypeName ]        (* the block is model data (2026-08-30): title,
                                                           about, and @description are kept and printed
                                                           back. `about` declares the working context —
                                                           each fire supplies its instance; a creating
                                                           beat links its newborn with a written sets
                                                           line (the about name never fills by itself) *)
                  [ "@description" string ]
                  beat*
beat        ::= story-sentence [ "@correction" ]          (* the correction channel (D10d): the only
                                                           beats that may leave @final *)
                  clause*
                  beat*                                   (* sub-beats = substory *)

sentence    ::= Actor verb Object ( prep participant )*   (* at most two participants *)
verb        ::= present-tense word, hyphenated when phrasal ("checks-in")
prep        ::= any single-word English preposition       (* the closed class, 54 words; stored, never
                                                           interpreted *)
participant ::= [ "every" ] (Actor | Object | TypeName ":" label)
determiner  ::= "every"                                   (* the language's ONLY determiner: one per
                                                           beat, before a singular noun; snapshot at
                                                           firing, one run per member; a/an/the/all/each
                                                           are pointed errors *)
```

**Every word in a beat is load-bearing (no filler):** after the verb, a lowercase word must be a
preposition or `every` — a typo'd preposition, a retired determiner, or a freestanding number is a
`file:line` error naming the word. Participants store as the ordered `{preposition, name}` pairs;
`Type:label` registers ONE type and carries the label as data (`story/subject-label`, the pair's
`label`); the printer writes the sentence back word-for-word.

## 3 · Beat clauses

Machinery lives ONLY in indented clauses — bare words, one clause per line. Every clause name needs
a **source**: the beat's own sentence (token verbatim, labels included), the story's `about`, or a
`finds` introduction.

```ebnf
clause      ::= gate | write | link | confer | find
gate        ::= ("requires" | "forbids") rule-text        (* must / must-not precondition; `ensures`
                                                           retired (D12a) — the produced state is a
                                                           written sets clause *)
write       ::= "sets" Name "." field "to" value          (* status writes (declared values only),
                                                           ref writes (the name's is-a chain must reach
                                                           the link's type), numbers, bools; several per
                                                           beat, distinct fields *)
              | "adds" Name "to" Name "." []-field        (* the collection write *)
link        ::= "where" Name "." path "is" ["not"] Name   (* entity-to-entity check — which thing;
                                                           conditions on values are requires/forbids *)
              | "where" Name "is" ["not"] "in" Name "." []-field
                                                          (* the membership guard over a collection *)
confer      ::= "grants" Role "to" Name                   (* role confer / withdraw; the receiver must be
              | "revokes" Role "from" Name                   named in the beat's OWN sentence; subsets
                                                           are never granted *)
find        ::= "finds" TypeName [":" label]              (* introduces a record the sentence does not
                  ("where" | "and") condition*               name; conditions indent one per line; bare
                                                           fields are the FOUND type's; sentence/about
                                                           names appear as values; exactly one match or
                                                           the step refuses *)
```

**Written birth (2026-08-29):** every creating beat writes its birth — `sets X.status to "value"`,
any declared value; different doors may write different births. Hand-sparse at the door: an omitted
line fills with the first declared value, and the printer always emits it — the canonical corpus is
never missing one. A beat gated on its primary's status is not a creating door; its status write is
the beat's transition edge.

```
Student starts Retreat
  finds RetreatBooking
    where student is Student
    and retreat is Retreat
    and status is "confirmed"
  sets RetreatBooking.status to "attending"
```

## 4 · Rules

```ebnf
rule        ::= disjunct ("or" disjunct)*                 (* parens distribute to flat DNF *)
disjunct    ::= atom ("and" atom)*
atom        ::= [ "not" ] shape
shape       ::= path relation (value | path)              (* state / comparison *)
              | Entity "is" ["not"] Entity                (* identity — crosses role types (one human) *)
              | quantifier Type [ "per" field ("and" field)* ] [ "where" filter ]
                                                          (* cardinality; `per` scopes the bound within
                                                           each distinct combination (D11c) *)
              | quantifier Type "in" []-field-path [ "where" filter ]
                                                          (* introducer-first collection quantification *)
              | ("no"|"some"|"all") element-pred "in" []-field-path
                                                          (* the trailing restrictor: closes exactly the
                                                           one atom; bare fields are the element's *)
              | "every" Type "in" []-field-path [ "where" filter ]
              | path "exists" | Type "exists"
relation    ::= "is" | "is not" | "is more than" | "is less than" | "is at least" | "is at most"
quantifier  ::= "all" | "no" | "some"                     (* the polarity words; some = at least one *)
              | ("exactly" | "at least" | "at most" | "more than" | "fewer than") bound
bound       ::= integer | path                            (* field-valued: at most Order.capacity Guest *)
```

Fail-closed on unset/dangling; `not` forms stay fail-open. `now` normalizes onto the field side.
Total: no loops; stratified. Over an empty collection `some` fails and `no` holds vacuously, so
paired subsets partition. Many-links never appear mid-path — reads over them use the set words.

```
@rule Retreat.capacity is more than 0
@rule Retreat.bookedCount is at most Retreat.capacity
@rule at most 1 RetreatBooking per student and retreat
@where some status is "attended" in retreatBookings
```

## 5 · Formulas

```ebnf
formula     ::= expression | aggregation | conditional
expression  ::= operand (op operand)*
op          ::= "+" | "*"                                 (* numbers; "+" on strings = concat *)
              | "/" … rounding                            (* division names its rounding at each site:
                                                           `rounded down` | `rounded up`; bare / = error —
                                                           no global default (D14c; `nearest` cut by owner
                                                           spelling 2026-08-30); money splits
                                                           largest-remainder *)
              | ("+" | "-") duration                      (* date shift: dueDate + 30 days *)
aggregation ::= agg-op "of" Type ["." field] ["via" refField] ["where" filter]
              | agg-op "of" []-field-path ["where" filter]         (* collection-scoped: count only *)
              | agg-op "of" Type ["." field] "in" []-field-path ["where" filter]   (* the D5 form *)
agg-op      ::= "sum" | "count" | "min" | "max" | "average"   (* `avg` renamed 2026-08-30; average
                                                           names its rounding — `average of X.f
                                                           rounded down|up`; bare average = error *)
split       ::= "split" "of" money-path "across" []-field-path
                                                          (* the D14c money allocation — the formula
                                                           lives on the MEMBER type; each member
                                                           computes its share; largest-remainder
                                                           (leftover minor units to the first members
                                                           in creation order), so the shares always
                                                           sum to the total exactly *)
conditional ::= value "if" condition ("," value "if" condition)* "," "else" value
```

```
confirmedCount: int = count of RetreatBooking where status is "confirmed"
bookedCount: int = confirmedCount + (count of RetreatBooking where status is "attending") + (count of RetreatBooking where status is "attended")
fillPercent: int = bookedCount * 100 / capacity rounded down
total: int = days between reservation.checkIn and reservation.checkOut
           * reservation.roomClass.rate + reservation.serviceCharges
```

Every path type-resolves against the catalog; collection-scoped reads lower through the mirror's
paired ref; all arithmetic is integer (decimals/`%` are parse-time sugar over scaled ints); acyclic.

## Retired forms — pointed replaced-syntax errors

`produces "value"` (→ `sets`) · `ensures` under beats · bare `rule` · `given` · `role … over` ·
`has` / `has one/many` · `@terminal` (→ `@final`) · comma field modifiers · `(stamped on create)` ·
the determiners `a`/`an`/`the`/`all`/`each` · freestanding numbers in sentences · `always` in type
blocks (→ `@rule`) · bare `description`/`type`/`boundary` lines · header `=` subsets (→ `is a` +
`@where`).
