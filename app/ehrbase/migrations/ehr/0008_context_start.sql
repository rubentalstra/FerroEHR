-- node.context_start — the first promoted-leaf column (our own storage design;
-- no openEHR spec governs storage columns or indexing — docs/architecture.md
-- §Storage). It promotes EVENT_CONTEXT.start_time.value onto the COMPOSITION
-- root row (num = 0) so the AQL patient-dashboard shape
--   SELECT … FROM EHR e CONTAINS COMPOSITION c …
--   ORDER BY c/context/start_time/value DESC LIMIT n
-- orders by an indexed timestamptz column instead of re-extracting the value
-- through a correlated EVENT_CONTEXT subtree scan + per-row ::timestamptz cast.
-- That correlated subquery is the measured hot path (2 s+ at the 10k rung, the
-- one AQL row lost to upstream at 100k — docs/plans/phase-20-optimization.md
-- §The measured evidence). The (rm_type, path)→column mapping is the shared
-- registry in app/ehrbase/src/storage/promoted.rs, consumed identically by the
-- write path (node population) and the AQL lowering (column substitution).
--
-- timestamptz, populated via ext.openehr_timestamp so a value the query-time
-- cast already accepted yields the byte-identical instant, and partial-precision
-- / malformed values become NULL instead of failing the write (see ext/0003).
ALTER TABLE node ADD COLUMN context_start timestamptz;

-- Backfill every existing COMPOSITION root. jsonb_path_query_first returns NULL
-- when the path is absent (a persistent COMPOSITION with no context, RM ehr
-- master03 §COMPOSITION.context [0..1]) → context_start stays NULL. The
-- fail-safe cast (ext/0003) mirrors the write path exactly, so backfilled and
-- newly-written rows are computed by one and the same conversion.
UPDATE node
   SET context_start = ext.openehr_timestamp(
           jsonb_path_query_first(data, '$.context.start_time.value') #>> '{}')
 WHERE rm_type = 'COMPOSITION' AND num = 0;

-- Partial btree serving the dashboard ORDER BY under the ehr_id scope: the
-- COMPOSITION roots of one EHR, ordered by context start-time. No openEHR spec
-- governs storage indexing — our own design; it targets the AQL ORDER-BY hot
-- path measured in docs/plans/phase-20-optimization.md.
-- The predicate is exactly `rm_type = 'COMPOSITION'` (NOT `... AND num = 0`):
-- COMPOSITION occurs only at the root, so the two select the identical rows,
-- but the AQL lowering emits only the `rm_type` filter (never a `num = 0`
-- clause), and a partial index is used only when the query's predicates imply
-- the index predicate — an unmatchable `num = 0` term would leave the index
-- unused for the very query it exists to serve.
CREATE INDEX idx_node_context_start ON node (ehr_id, context_start)
    WHERE rm_type = 'COMPOSITION';

COMMENT ON COLUMN node.context_start IS 'Promoted EVENT_CONTEXT.start_time.value (timestamptz) on the COMPOSITION root (num = 0); NULL elsewhere and for context-less persistent compositions. Serves the AQL dashboard ORDER BY (our own storage design — no openEHR spec governs it; mapping in storage/promoted.rs).';
