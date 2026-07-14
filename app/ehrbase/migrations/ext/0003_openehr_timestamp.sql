-- ext.openehr_timestamp — a fail-safe ISO-8601 → timestamptz conversion.
--
-- No openEHR spec governs storage columns or their derivation — this is our own
-- design (docs/architecture.md §Storage). It exists so promoted timestamp
-- columns (the first being node.context_start, ehr/0008) are populated
-- identically at write time and in their one-time backfill: both feed the raw
-- canonical leaf text (e.g. EVENT_CONTEXT.start_time.value) through this
-- function.
--
-- The body is exactly PostgreSQL's own text::timestamptz input parser, so for
-- every value the AQL engine's query-time cast ((… #>> '{}')::timestamptz)
-- already accepted, this returns the byte-identical instant — the promoted
-- column is a true drop-in for the correlated-subquery lowering
-- (app/ehrbase/src/aql/sql/value.rs). The EXCEPTION handler turns any
-- non-castable text (ISO-8601 partial precision such as `2021`, or malformed
-- input) into NULL instead of erroring, so a write can never fail on a value
-- the query path would merely have rejected (QUERY master03 §Built-in
-- Types/Dates and Times documents partial precision as the boundary). STRICT
-- short-circuits NULL input (a persistent COMPOSITION without context, RM ehr
-- master03 §COMPOSITION.context [0..1]) to NULL with no subtransaction cost.
--
-- STABLE, not IMMUTABLE: text::timestamptz depends on the session TimeZone for
-- offset-less inputs, so this must not back an expression index; it is only
-- used in DML (the write INSERT and the backfill), where STABLE is correct.
-- Canonical DV_DATE_TIME values carrying an explicit offset/Z are
-- TimeZone-independent.
CREATE FUNCTION openehr_timestamp(v text) RETURNS timestamptz
LANGUAGE plpgsql STABLE STRICT PARALLEL SAFE AS $$
BEGIN
    RETURN v::timestamptz;
EXCEPTION WHEN others THEN
    RETURN NULL;
END;
$$;

COMMENT ON FUNCTION ext.openehr_timestamp(text) IS
    'Fail-safe ISO-8601 text -> timestamptz: PostgreSQL''s own timestamptz parser, returning NULL on any non-castable (partial-precision or malformed) input instead of erroring. Feeds promoted timestamp columns (node.context_start) at write + backfill. STABLE (TimeZone-dependent for offset-less inputs); not legal in index expressions.';

-- Grants mirror ext/0001: the runtime writer executes this on the write path
-- (promoted-column population); the reader may see it via the backfill. The
-- ALTER DEFAULT PRIVILEGES in ext/0001 already covers functions the migrator
-- creates, so this explicit grant is the same belt-and-suspenders the baseline
-- uses (idempotent — GRANT is a no-op if already held).
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ehrbase_app') THEN
        GRANT EXECUTE ON FUNCTION ext.openehr_timestamp(text)
            TO ehrbase_app, ehrbase_reader;
    ELSE
        RAISE NOTICE 'skipping ext.openehr_timestamp grant (roles absent — see the role block NOTICE in 0001)';
    END IF;
END $$;
