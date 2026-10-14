Agent-state classification (apply at initial authoring and every domain expansion)

Decide what each field means before choosing its storage or UI projection.
Agent state describes this actor's explicitly modeled identity, personal resources,
or recorded perspective: its own declared intention, plan, belief, or remembered
observation. World state describes the institution and its operations. Ownership,
control, an actor reference, a type named Agent, or usefulness to the actor does
not establish agent state. Agent state is also stored in Onto; storage location
alone does not determine its meaning.

For each proposed agent field, check:
1. Whose state is it, and what does its value assert? If it asserts an objective
   fact about a desk, ticket, queue, workflow, inventory, customer, account, or
   organization, keep it in world state. If it would still describe the institution
   after this actor were replaced, that is strong evidence it belongs in the world.
2. What explicitly establishes this actor-specific meaning? Use the supplied seed
   commitments or recorded actor declarations/observations, never an inferred
   private goal, invented thought, or assumption that an actor has learned a fact.
3. What authorized operation changes it? A world transition does not by itself
   mean the actor noticed, remembered, intended, or believed anything. A memory
   requires an explicit modeled observation/recording operation with evidence;
   a plan or intention requires an explicit actor choice. Do not generate completed
   mental events as a side effect of registering a ticket.
4. Is it genuinely new state needed for this capability? Preserve existing
   declarations. Do not duplicate a world fact under an actor-owned wrapper or
   add fields merely to fill the left panel. If ambiguous, omit the projection
   and keep the domain fact in world state. Return {fields:[]} when appropriate.

Examples and boundary cases:
- IntakeDesk.status = ready/registered: WORLD, even if owner = Operator:Ada.
  RepairRequest.status, assigned operator, queue size, and registration receipt:
  WORLD. An assignment records an institutional relationship, not an intention.
- Company cash, ledger balance, department budget, or organizational ceiling:
  WORLD. A separate personal spending allowance explicitly allocated to the acting
  entity may be AGENT resource state. Never infer an allowance from account access
  or ownership. An organization acting as the agent can expose its explicitly
  declared spendable resources; establish that actor identity in the seed first.
- An explicitly recorded agent intention, such as its chosen next task: AGENT.
  A task assigned by the institution alone: WORLD. Do not invent an intention
  from the assignment or from the private investigator goal.
- Ticket.status = pending: WORLD. An agent's last observed ticket status can be
  AGENT memory only if separately recorded by a modeled observation action; it
  can remain stale when the world changes. Do not automatically mirror live facts.
- A new repair-intake capability with only desks, tickets and registration
  Stories adds no agent-internal state: return agent_state: {fields:[]}.

Initialize only facts justified by the seed. Do not fabricate a belief or successful
observation to provide a nonempty value. If a required scalar cannot be initialized
truthfully, omit that agent field or model an explicit unobserved/unset state with
valid declared types. Check semantics again after repair, not just compilation.
Return only the required source/agent_state/error object, without extra explanation
keys. Type/ownership validation checks the binding; it does not prove this semantic
classification. A declaration also does not provide private or role-filtered reads.
