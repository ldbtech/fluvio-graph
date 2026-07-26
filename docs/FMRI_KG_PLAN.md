# fMRI Knowledge Graph — Activatable Connectome (fluviome proof)

**Thesis:** A general-purpose KG engine (fluviome), never built for neuroscience,
ingests real 3T fMRI data and answers neuroscience questions by *activating*
subgraphs. Proof point: ask the same question, watch different brain-types light up
differently — via spreading activation over a connectome knowledge graph.

Dataset: https://www.kaggle.com/datasets/mathurinache/3t-fmri-dataset/data

---

## 1. The core design — 3 layers

```
┌─ SEMANTIC layer ──── concept/term ──(BGE text-embed)──► brain region
│                       "memory","fear"        │  (a QUESTION enters here)
│                                              ▼
├─ CONNECTOME layer ── region ──functional-connectivity── region   (per subject)
│                        │  activation SPREADS through each brain
│                        ▼
└─ INDIVIDUAL layer ── subjects clustered by connectome fingerprint
                         (activation stays inside compatible brains)
```

**Query flow:** question → embed → match Neurosynth concept → seed region(s) →
spreading activation over each subject's connectome → return lit-up subgraph,
grouped per cluster so the answer contrasts brain-types.

**Why clustering (the "reasoning gets mixed up" fix):** inter-subject variability
means you must NOT average brains. Activation runs inside each subject's own
connectome; clusters become "brain subtypes" and the per-cluster contrast IS the
insight (connectome fingerprinting — Finn et al. 2015).

### Honesty caveats (non-negotiable)
- fMRI functional connectivity = statistical co-activation, **not** semantic reasoning.
  A region does not "store a concept." The concept→region link MUST be grounded, via:
  - **Neurosynth** (~1,300 term→coordinate maps) — always-available prior, and the
    place BGE-small text embeddings earn their keep; and/or
  - **task labels** in the dataset, if it is task-based (confirm in Step 0).
- "Talk to a brain region" is a UI metaphor over spreading activation, stated as such.

---

## 2. Graph schema (SurrealDB via fluvio-graph)

Nodes:
- `Subject`  — id, demographics (if present), cluster_id, connectome_embedding
- `Region`   — atlas id, name, network (DMN/salience/…), MNI coords
- `Concept`  — Neurosynth term, text_embedding (BGE-small, 384-dim)
- `Cluster`  — subtype id, centroid connectome, member count

Edges:
- `(Subject)-[:HAS_CONNECTIVITY {weight}]->(Region)` per-subject region activity anchor
- `(Region)-[:CONNECTED {weight, subject_id}]->(Region)` functional connectivity
- `(Concept)-[:ASSOCIATED {z}]->(Region)` Neurosynth prior
- `(Subject)-[:IN_CLUSTER]->(Cluster)`

Query capability (spreading activation): personalized PageRank / random-walk-with-
restart seeded at Concept→Region, restricted to one Subject/Cluster's edge set.

---

## 3. Compute topology

- **Gaming PC (GPU, primary):** download + preprocess + connectivity + clustering +
  embeddings + hosts the fluviome stack (SurrealDB + engine + gateway, docker compose).
- **MacBook:** dev + NeuroCopilot UI only, over LAN (host-derived API URLs / apiBase.ts;
  CORS + 0.0.0.0 already configured).
- Raw NIfTI never leaves the gaming PC; only the derived graph (small) is served.

---

## 4. Phased plan (mapped to real components)

### Step 0 — Confirm the dataset (blocking)
On the gaming PC: `kaggle datasets download mathurinache/3t-fmri-dataset`.
Determine: raw NIfTI vs preprocessed · resting-state vs task · #subjects · size ·
labels/atlas present. Decides the grounding path (Neurosynth-only vs +task labels).

### Phase 1 — fMRI → connectome (gaming PC, Python)
New connector at `services/fluvio-connectors/src/connectors/fmri/`:
- preprocess (nilearn: motion-correct, register to MNI)
- parcellate (Schaefer-200 or AAL atlas) → per-region BOLD time series
- functional connectivity matrix per subject (correlation, Fisher-z)
- output: per-subject edge list + node table (Parquet/JSON)
Reuses the existing connector pattern (github/notion/local_drive).

### Phase 2 — Fingerprint clustering
- vectorize each connectome (upper-triangle of connectivity matrix)
- cluster (k-means / spectral) → `Cluster` nodes + `Subject.cluster_id`
- validate: connectome identifies individuals (fingerprint accuracy sanity check)

### Phase 3 — Semantic layer (Neurosynth + BGE)
- ingest Neurosynth term→region associations → `Concept` + `ASSOCIATED` edges
- embed concept terms with `crates/fluvio-embed` (BGE-small, 384-dim)
- if task-labeled: add empirical condition→region edges alongside priors

### Phase 4 — Ingest into fluviome
- push nodes/edges through `services/fluvio-ingestion` → `services/fluvio-graph`
  (SurrealDB), embeddings via `services/fluvio-graph/src/embeddings.rs`

### Phase 5 — Activation query capability
- implement seeded spreading activation (personalized PageRank) as a graph query /
  CSP capability, scoped per Subject/Cluster
- expose via `fluvio-gateway` (Apollo Router :4001)

### Phase 6 — NeuroCopilot (MacBook UI)
- reuse Pitch/Cortexa chat + `services/agent-planner`: question → embed → seed concept
  → activation query → visualize lit-up subgraph per cluster
- typed view contract (brain map + per-cluster panels)

---

## 5. Success criteria
- [ ] End-to-end: raw fMRI → connectome → fluviome → activation answer
- [ ] Fingerprint clustering separates subjects into stable subtypes
- [ ] Same NL question activates *different* subgraphs across clusters (the demo)
- [ ] Concept→region grounding is Neurosynth/label-backed, not fabricated
- [ ] Whole thing runs on the gaming PC, driven from the MacBook over LAN

## 6. Open questions / risks
- Dataset contents (Step 0) — may be raw (heavy preprocess) or preprocessed (skip Phase 1 work)
- Atlas choice (Schaefer vs AAL) — pick after seeing resolution
- Preprocessing cost — full fMRIPrep is heavy; nilearn lightweight path preferred for v1
- Neurosynth coord→atlas-region mapping needs a resampling step
```
