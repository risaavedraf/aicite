# Users and Problems

The MVP targets people and agents who need reliable document context from a private corpus, not open-ended chat.

## Primary persona

### Corpus owner / operator

A person who owns the document corpus and wants it to be searchable, inspectable, grounded, and private.

Examples:

- Founder reviewing internal docs, specs, customer notes, or policies.
- Professional querying study material, manuals, or reports.
- Team member preparing a private knowledge base for an agent.

## Secondary persona

### Agent integrator / developer

A person wiring the CLI engine into an agent, workflow, or automation.

They care about:

- Stable command names and outputs.
- Machine-readable JSON responses.
- Clear errors and exit codes.
- Citations, traceability, and provider abstraction.
- Durable local state that works across separate CLI invocations.

## System actor

### Consumer agent

An AI agent that uses the CLI/engine as a tool.

It needs:

- Deterministic commands.
- Ranked chunks and citations it can hand back to a human.
- Context packs it can place into its own reasoning flow.
- Source-read and trace output it can inspect or forward.
- Explicit no-results behavior when the corpus does not support the request.

## Jobs to be done

| Job | User story |
|---|---|
| Load knowledge | As a corpus owner, I want to ingest documents so the engine can retrieve from my own material. |
| Retrieve grounded context | As an operator or agent, I want cited chunks based on the corpus, not generic model guesses. |
| Build an agent context pack | As an agent integrator, I want stable JSON that can be safely added to an agent prompt or workflow. |
| Verify sources | As a user, I want citations and trace data so I can check whether downstream answers are trustworthy. |
| Manage corpus state | As a user, I want to know whether documents are ready, processing, or failed. |
| Demo the cite | As a technical reviewer, I want to see a real grounded retrieval flow end to end. |
