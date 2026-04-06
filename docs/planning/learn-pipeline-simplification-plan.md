# Learn Pipeline Simplification — Implementation Plan

## Overview

**Current flow:**
`Episodes → extract candidates → per-candidate describer (LLM) → DraftSkill → per-draft synthesizer (LLM) → Skill → D3 supersession (entity_type + action)`

**Target flow:**
`Episodes+Edges → D1 co-occurrence (count≥3) → **intra-batch dedup** → **inter-pattern dedup** → **batch LLM** → Skill directly → D3 supersession (entity_type overlap only)`

This eliminates:
- One full round of LLM calls (the `PatternDescriber` → becomes batch `BatchLabelDescriber`)
- The intermediate `DraftSkill` layer (merged into direct `Skill` creation)
- Pattern storage entirely (candidates go directly to skills, no `store_pattern()` calls)

---

## 1. Introduce `BatchLabelDescriber` trait (replaces `PatternDescriber`)

**File:** `crates/ctxgraph-core/src/pattern.rs` (~line 31)

- **Remove:** `PatternDescriber` trait entirely
- **Add:**
  ```rust
  pub trait BatchLabelDescriber: Send + Sync {
      /// Given a slice of PatternCandidate, return a Vec of (candidate_id, label) pairs.
      /// label is a 1-2 sentence behavioral description.
      /// May return fewer results than input (some candidates may be skipped).
      fn describe_batch(
          &self,
          candidates: &[PatternCandidate],
          source_summaries: &HashMap<String, Vec<String>>,
      ) -> crate::error::Result<Vec<(String, String)>>;
  }
  ```
- **Move** `MockPatternDescriber` → `MockBatchLabelDescriber` (implements batch interface, returns deterministic descriptions per candidate)
- **Rename** `FailingPatternDescriber` → `FailingBatchLabelDescriber` (same impl, just rename)
- Keep `PatternExtractor` and `PatternCandidate` unchanged

### Batch Prompt Format

The batch LLM prompt uses this structure:

```
You are a behavioral pattern analyzer. For each pattern below, generate a 1-2 sentence behavioral label describing what the agent or user does/should do.

Patterns:
1. [entity_a] --(relation)--> [entity_b] (triplet) | episodes: 3 | types: Docker, Network
2. [entity_c] <-> [entity_d] (pair) | episodes: 4 | types: Service, Database

Output JSON array:
[
  {"id": "1", "label": "When Docker networking issues occur, the agent should check DNS first..."},
  {"id": "2", "label": "Service and Database frequently co-occur in failure scenarios..."}
]

Rules:
- Max 150 chars per label
- Focus on observable behaviors, not metadata
- DO NOT include episode counts or entity type names
```

**`source_summaries` HashMap key:** `pattern_id -> Vec<String>` — episode contents for that pattern's source groups (episode IDs as keys).

---

## 2. Remove `DraftSkill` — merge into direct `Skill` creation

**File:** `crates/ctxgraph-core/src/types.rs` (~line 1512-1520)

- **Remove:** `DraftSkill` struct

**File:** `crates/ctxgraph-core/src/skill.rs` (~line 121-209)

- **Remove:** `SkillSynthesizer` trait (lines ~20-29)
- **Remove:** `MockSkillSynthesizer`, `FailingSkillSynthesizer` (lines ~34-122)
- **Replace** `SkillCreator::draft_skills()` with `SkillCreator::create_skills()`:

```rust
pub fn create_skills(
    patterns: &[PatternCandidate],
    edges: &[Edge],
    source_summaries: &HashMap<String, Vec<String>>,
    descriptions: &HashMap<String, String>,  // candidate_id → LLM description from batch
    config: &SkillCreatorConfig,
    scope: SkillScope,
    agent: &str,
) -> Vec<Skill>
```

**Internal logic:**
1. Success/failure counting happens INSIDE `create_skills()` before the LLM call (same logic as current `draft_skills()`, using edges matched by `episode_id` from `source_groups`)
2. `Skill.name` — template-based from `entity_types` + `success_count` + `failure_count` (e.g., "Docker+Network pattern (3 successes, 1 failure)")
3. `Skill.description` — from the `descriptions` map (the batch LLM output)
4. `Skill.trigger_condition` — template-based from `entity_types`
5. `Skill.action` — template-based from entity_types + most common success relation
6. `Skill.provenance` — template-based (no LLM): "Pattern observed across N episodes with X successes and Y failures", `context_facts` = joined episode summaries

**No LLM call needed for provenance** in the simplified flow.

---

## 3. Rewrite `run_learning_pipeline` in `graph.rs`

**File:** `crates/ctxgraph-core/src/graph.rs` (~line 1143-1253)

- **Signature change:**
  ```rust
  pub fn run_learning_pipeline(
      &self,
      agent: &str,
      scope: SkillScope,
      describer: &dyn BatchLabelDescriber,   // was: &dyn PatternDescriber
      limit: usize,                           // removed: synthesizer param
  ) -> Result<LearningOutcome>
  ```

- **New flow:**
  1. `extract_pattern_candidates(&config)` — unchanged
  2. **Filter:** `candidates.retain(|c| c.occurrence_count >= 3)`
  3. **Intra-batch dedup:** Build `pattern_key()` set from candidates themselves, remove duplicates within the batch
  4. **Inter-pattern dedup:** Build `pattern_key()` set from existing stored patterns, filter candidates against that
  5. **Build `source_summaries`:** `HashMap<pattern_id, Vec<String>>` — episode contents keyed by pattern_id
  6. **Batch LLM call:** `describer.describe_batch(&candidates, &source_summaries)` → `HashMap<candidate_id, description>`
  7. **Create Skills directly:** `SkillCreator::create_skills(...)` with the descriptions map
  8. **D3 Supersession:** entity_type overlap only (remove action comparison)

- **Remove:**
  - The per-candidate `generate_pattern_description()` loop (lines ~1168-1189)
  - `create_skills_from_patterns()` call (absorbed into new flow)
  - `store_pattern()` calls — **NO pattern storage in simplified flow** (candidates go directly to skills)

---

## 4. Simplify D3 supersession — entity_type overlap only

**File:** `crates/ctxgraph-core/src/graph.rs` (~line 1220-1238)

- **Current logic:**
  ```rust
  if new_skill.entity_types.iter().any(|et| old_skill.entity_types.contains(et))
      && new_skill.action != old_skill.action
  ```
- **New logic:**
  ```rust
  if new_skill.entity_types.iter().any(|et| old_skill.entity_types.contains(et))
  ```
- Remove the `action` comparison

---

## 5. Update CLI learn command

**File:** `crates/ctxgraph-cli/src/commands/learn.rs` (~230 lines)

- **Remove:** `RealPatternDescriber` struct (lines 11-120)
- **Remove:** `RealSkillSynthesizer` struct (lines 123-217)
- **Add:** `RealBatchLabelDescriber` — single struct that:
  - Takes all candidates, builds a **single batch prompt** (see prompt format above)
  - Calls LLM **once** (not N times)
  - Parses JSON array response `[{id, description}, ...]`
  - Returns `Vec<(String, String)>`
- **Update** `run()` to use new pipeline signature
- **Remove** `synthesizer` parameter

---

## 6. Update public API exports

**File:** `crates/ctxgraph-core/src/lib.rs`

- **Remove exports:** `PatternDescriber`, `SkillSynthesizer`, `DraftSkill`, `MockSkillSynthesizer`, `FailingSkillSynthesizer`, `MockPatternDescriber`, `FailingPatternDescriber`
- **Add exports:** `BatchLabelDescriber`, `MockBatchLabelDescriber`, `FailingBatchLabelDescriber`

---

## Deduplication Strategy

Two-stage deduplication using `pattern_key()`:

**Stage 1 — Intra-batch dedup (NEW):**
```rust
let mut seen_keys: HashSet<String> = HashSet::new();
candidates.retain(|c| {
    let key = pattern_key(c);
    !seen_keys.contains(&key)
});
```

**Stage 2 — Inter-pattern dedup against existing:**
```rust
let existing_patterns = self.storage.get_patterns()?;
let existing_keys: HashSet<String> = existing_patterns.iter().map(|p| pattern_key(p)).collect();
candidates.retain(|c| !existing_keys.contains(&pattern_key(c)));
```

**Key insight:** The simplified flow does NOT store patterns — candidates go directly to skills. So Stage 2 dedupes against existing stored patterns only to avoid re-creating skills from patterns already in storage.

---

## Test Strategy

| Test | Location | What |
|------|----------|------|
| `test_batch_describer_returns_all` | `pattern.rs` | MockBatchLabelDescriber returns description per candidate |
| `test_batch_describer_empty` | `pattern.rs` | Empty input → empty output |
| `test_create_skills_from_candidates` | `skill.rs` | End-to-end: patterns + edges + descriptions → Skills |
| `test_create_skills_no_success_no_failure` | `skill.rs` | Patterns with no signal → no skills |
| `test_create_skills_uses_description_from_map` | `skill.rs` | Skill.description matches the batch LLM output |
| `test_create_skills_name_template` | `skill.rs` | Skill.name is template-based (entity_types + counts) |
| `test_create_skills_provenance_template` | `skill.rs` | Provenance is template-based, not LLM |
| `test_pipeline_intra_batch_dedup` | `graph.rs` integration | Duplicate candidates within batch → only one passes |
| `test_pipeline_dedup_before_llm` | `graph.rs` integration | Existing pattern key → candidate filtered out before LLM |
| `test_pipeline_count_filter` | `graph.rs` integration | occurrence_count < 3 → filtered out |
| `test_supersession_entity_type_overlap` | `graph.rs` integration | Overlapping entity_type triggers supersession regardless of action |
| `test_cli_batch_llm_single_call` | `learn.rs` | Verify only 1 HTTP call for N candidates |
| Existing extractor tests | `pattern.rs` | No changes needed |

---

## Change Summary By File

| File | Action | Lines affected |
|------|--------|----------------|
| `crates/ctxgraph-core/src/pattern.rs` | Replace `PatternDescriber` → `BatchLabelDescriber` trait + mocks; rename `FailingPatternDescriber` → `FailingBatchLabelDescriber` | ~31-150 | **DONE** |
| `crates/ctxgraph-core/src/skill.rs` | Remove `SkillSynthesizer`, merge `draft_skills` → `create_skills` (with template-based name/trigger/action/provenance), update tests | ~1-210, ~212-460 | **DONE** |
| `crates/ctxgraph-core/src/types.rs` | Remove `DraftSkill` struct | ~1512-1520 | **DONE** |
| `crates/ctxgraph-core/src/graph.rs` | Rewrite `run_learning_pipeline` with intra-batch dedup, no pattern storage, batch LLM call, direct Skill creation; simplify supersession | ~870-920, ~1143-1253 | **DONE** |
| `crates/ctxgraph-core/src/lib.rs` | Update public exports | exports section | **DONE** |
| `crates/ctxgraph-cli/src/commands/learn.rs` | Replace 2 describers with 1 batch describer, update CLI call | ~1-230 | **DONE** |
| `crates/ctxgraph-mcp/src/tools.rs` | Update learn tool to new API | line 385-391 | **DONE** (bonus — not in original plan) |

## Implementation Status

All 6 planned sections implemented and tested. See [session10-summary.md](session10-summary.md) for full details, trade-offs, and follow-up work.

**Commits:**
- `f6679f2` — S1: codebase analysis
- `68cdbc3` — S2: BatchLabelDescriber trait
- `ee53161` — S3-S8: remove DraftSkill/SkillSynthesizer, rewrite pipeline
- `870b085` — S9: test pass marker (92 lib + 50 integration)
