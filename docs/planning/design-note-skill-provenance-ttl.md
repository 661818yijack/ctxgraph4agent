# Design Note: Skill Provenance and Context Decay

> Status: Draft
> Created: 2026-04-04
> Motivated by: hermes-mindgraph-plugin FTS research + skill enhancement gap

## Problem

Skills encode durable procedures ("how"), but lose the reasoning behind them ("why"). When we enhance a skill months later, we're working blind -- we don't know:

1. **Why** each instruction exists (what problem was it solving?)
2. **What** was the landscape when we made the decision
3. **What alternatives** were considered and rejected
4. **What assumptions** the skill is built on

Without this provenance, skill enhancement becomes guesswork. Worse, the world changes but the skill doesn't -- because we can't tell which parts are still valid.

## Concrete Example: Model Behavior Shift

Six months ago, a skill might include:

```
Always research thoroughly before implementing. Never make assumptions.
```

The **reasoning** at the time: LLMs were overconfident, would code the wrong thing without understanding the problem.

Today, models are better at asking clarifying questions. The same instruction might now be counterproductive -- it could cause the model to spiral into excessive research. The skill needs to evolve to something like:

```
Research with a timebox. Ask the user before assuming scope.
```

But if the skill only stores the procedure (no provenance), we can't tell:
- Why "research first" was added
- That the underlying assumption (models are overconfident) has flipped
- Whether to tighten or loosen the instruction

## Proposed Structure

Skills should have two layers with different TTL characteristics:

### Layer 1: Skill Core (Durable)
The procedure itself. What to do. Steps, triggers, actions.

- **TTL**: None (never expires) or very long (1-2 years)
- **Updated when**: Procedure fundamentally changes
- **What lives here**: `trigger_condition`, `action`, `description`

This is what D2 already defines. No changes needed.

### Layer 2: Skill Provenance (Perishable)
The decision trace behind the skill. Why it was created, what context shaped it.

- **TTL**: 3-6 months (configurable via `skill_provenance_ttl_days`)
- **Auto-decays**: Uses the same decay mechanism as Phase A entities
- **Captured at creation time**: NOT reconstructed later -- the creator writes it while the context is fresh
- **Updated when**: New evidence contradicts or refines the reasoning

### Layer 3: Skill Context (Fast-Perishable)
Current state of the world relevant to this skill. Tool versions, model capabilities, ecosystem facts.

- **TTL**: 1-3 months
- **Purpose**: Quick check "is the world still the way I think it is?"
- **Examples**: "GPT-4 is best for code review", "MiniMax M2.7 > OpenAI for CLI tasks"

## TTL Rationale

The TTL values come from how fast the relevant context shifts:

| Layer | TTL | Why |
|-------|-----|-----|
| Core | None | Procedures are abstract -- "ask before assuming" is always reasonable |
| Provenance | 3-6mo | Decision reasoning becomes unreliable as context shifts. 6 months ago's "why" may be actively misleading |
| Context | 1-3mo | Factual landscape (model rankings, tool versions) changes fastest. 3 months is already stale |

These are configurable defaults, not hard rules. A skill about "how to use SQLite FTS5" might have longer provenance TTL than a skill about "which model to use for research."

## Data Model Changes (for D2)

Extend the `Skill` struct with provenance fields:

```rust
pub struct Skill {
    // Existing fields (D2)
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger_condition: String,
    pub action: String,
    pub success_count: u32,
    pub failure_count: u32,
    pub confidence: f64,
    pub superseded_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub entity_types: Vec<String>,

    // New: Provenance layer
    pub provenance: Option<SkillProvenance>,
}

pub struct SkillProvenance {
    /// Why was this skill created? What problem does it solve?
    pub reasoning: String,

    /// What alternatives were considered and rejected?
    pub alternatives_rejected: Option<String>,

    /// What assumptions is this skill built on?
    pub assumptions: Option<String>,

    /// Current context facts relevant to this skill
    pub context_facts: Option<String>,

    /// When was provenance last verified against reality?
    pub verified_at: DateTime<Utc>,

    /// When does provenance expire? (default: created_at + provenance_ttl)
    pub expires_at: DateTime<Utc>,

    /// How many times has provenance been renewed?
    pub renewal_count: u32,
}
```

### Provenance TTL Config

Add to `MemoryPolicyConfig`:

```toml
[policies.skill]
provenance_ttl_days = 180        # 6 months default
context_ttl_days = 90            # 3 months default
```

### Decay Behavior

Provenance follows the same exponential decay as Phase A entities:

- `decay_score` computed at query time (not stored)
- When `decay_score < threshold`, provenance is flagged as "stale" in retrieval
- Stale provenance doesn't delete the skill -- it flags it for review
- The skill core remains fully functional; only the "why" is questioned

### Supersession Flow (Enhanced)

When a skill is superseded (D2 already supports this):

1. Old skill gets `superseded_by` pointing to new skill
2. Old skill's provenance is archived (not deleted) -- useful for understanding evolution
3. New skill inherits provenance template from old skill but with fresh reasoning
4. The "what changed" delta between old and new provenance is captured

## Interaction with Phase A (TTL + Decay)

Provenance TTL reuses Phase A infrastructure directly:

- `decay_score()` from A2 applies to provenance
- TTL cleanup from A6 marks expired provenance for pruning (not the skill itself)
- The `renewal_count` field on `SkillProvenance` is independent of entity/edge renewal (C2) — skills are not entities

This is additive -- Phase A doesn't need changes. Provenance is just another thing with a TTL.

## Interaction with Phase D (Learn)

When `learn` creates a skill (D2/D4):

1. Pattern extraction finds consistent success/failure signals
2. Skill creation generates `trigger_condition` and `action` (existing behavior)
3. **New**: Provenance is auto-generated from the pattern's source episodes:
   - `reasoning`: "Created from N compression groups showing [pattern]"
   - `alternatives_rejected`: patterns that didn't meet the occurrence threshold
   - `assumptions`: derived from the entity types and relations involved
   - `context_facts`: snapshot of relevant entities at creation time

This means provenance capture is automatic, not manual. The LLM doesn't need to write a justification -- the context graph provides it from the decision trace.

## What This Solves

1. **Skill enhancement without amnesia**: When updating a skill, you can see WHY each instruction exists and whether the underlying assumption is still true
2. **Confident deletion**: If provenance is expired AND the skill has low confidence, you can prune it. If provenance is fresh, you know there's active reasoning behind it
3. **Context shift detection**: When context_facts expire, the system can flag "this skill was created when GPT-4 was the best model -- verify it still applies"
4. **Skill evolution audit trail**: Supersession chain + provenance = full history of why a skill changed

## Open Questions

1. Should provenance be a separate table or JSON column on the skills table?
   - Deferred to POC after Phase D2 implementation
   - Start with JSON column (simpler), POC against separate table with FTS5 on provenance text
   - Benchmark: query speed, FTS quality on provenance, write overhead

2. Should `context_facts` be linked to actual entities in the graph?
   - **Decision: Yes, link to entities.** Provenance-entity edges make shift detection automatic via Phase A decay.
   - When a linked entity decays or gets superseded, provenance flags itself -- no polling, no re-evaluation needed.
   - Example: provenance links to "MiniMax M2.7 best for research" entity. Entity decays when model landscape changes. Provenance auto-flags "context shifted."
   - Add `skill_provenance_edges` table: `(provenance_id, entity_uid, relation_type)`

3. How to handle provenance for manually-created skills (not from `learn`)?
   - Manual creation should still prompt for reasoning (even brief)
   - Empty provenance is valid -- just means "I don't remember why, proceed with caution"

4. Should expired provenance trigger an LLM re-evaluation?
   - **Decision: Yes, as Phase D re-verify step.** When provenance expires, flag the skill for LLM re-evaluation.
   - Gated by `usage_count` -- only re-evaluate skills with meaningful usage (configurable threshold).
   - LLM re-evaluation reads the skill core + old provenance + current context facts from the graph, then outputs: keep/modify/supersede recommendation.
   - This becomes Phase C4's "update format" applied to provenance -- structured re-verification instead of freeform.
