# Harbor Works in Ontolang

`harbor-works.onto` is a separate model of the Bellwether commercial world. It
preserves the existing Rust Harbor Works experiment. The Onto model owns its
organizations' economic state, agreements, service registrations, reliance
reports and continuity purchases. Frontier supplies the business agent, tools,
clock, executable hosting and trusted counterparty orchestration.

The model contains the original fourteen organizations: Harbor Works, six
venues, four vendors, District Dispatch, the District Association and the
Festival Committee. Harbor begins with $5,000 and twelve units of monthly
capacity. The other organizations retain their opening cash, prior obligations,
procurement limits, capacity and interests. The calendar runs for thirty days,
with the festival on day 21 and association check-ins on days 7, 14, 21 and 28.
The fixture uses January 1, 2026 as simulated day zero, independent of wall time.
All financial fields exposed by the runtime are integer USD cents.

## What the model enforces

- Proposed agreements reserve seller capacity and buyer funds immediately.
  Buyer approval requires the right counterparty, a supported capability, its
  procurement ceiling and remaining capacity. Rejection releases reservations.
- Authorization, delivery, customer acceptance and payment are separate
  transitions. Delivery requires a service belonging to the seller, with the
  agreed capability and fresh trusted availability evidence. Failed customer
  validation can return an agreement for repair and delivery again.
- Payment changes balances through formulas over paid agreements. A second
  payment fails the lifecycle gate. Hosted services transfer exactly $100 from
  Harbor to Pier Print. Their cost is computed and cannot be chosen by callers.
- Cash is conserved across the district. An organization's balance, uncommitted
  cash and capacity cannot fall below the modeled limits. Existing obligations
  and the association's remaining contingency funds stay reserved.
- The three seeded incumbent reliance reports concern Pier Print. After the
  first due check-in, the association may buy the existing Dispatch offline
  roster alternative for $450 and two units of capacity. This can happen without
  any Harbor sales. The alternative and its verified export are explicit seeded
  facts. The pilot can be purchased once; existing private contracts stay intact.

## Identity and execution boundary

The agent acts only as `Business:Harbor Works`. Other organization identities
belong to trusted Frontier orchestration. `Verifier:Service verifier` records
actual sandbox execution, health and export checks. Declaring a service,
delivering it or writing a workspace claim does not establish that evidence.

The pinned runtime accepts a `fields` object on existing records as well as on
creation. Therefore the HTTP adapter must allow only the fields appropriate to
each public Story and inject its own timestamps. Raw runtime execution is an
internal API, not an authorization boundary. The model's authored identity,
status and fixed-price writes are a second line of enforcement.

Ontolang timestamps in this pinned version use `YYYY-MM-DDTHH:mm:ss` without a
zone suffix. Frontier interprets these fixture timestamps as UTC. Onto's date
arithmetic and ordering operate on calendar days; Frontier additionally enforces
exact 86,400-second decision and validation delays and exact delivery deadlines.
The runtime model cannot verify shell execution by itself.

All facts in this research fixture are public, including counterparty financial
limits. This is a transparent simulation, not a model of private commercial
information or production authentication.

## Validate without model calls

After building the optional pinned runtime, run from the repository root with
Node 24 or newer:

```sh
node scripts/test-harbor-onto.mjs
```

The deterministic suite parses, prints and reopens the complete model; checks
all fourteen organizations; exercises a paid agreement including failed delivery
validation and repair; verifies procurement, budget, capacity and identity
refusals; purchases the association pilot; and checks conserved cash after every
step. It makes no network calls and uses no model credentials. Frontier's Rust
integration tests separately exercise HTTP field restrictions, clock scheduling,
service probes and restart behavior.
