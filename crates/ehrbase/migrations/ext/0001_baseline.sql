-- ext schema baseline — AQL support functions, aggregates, and collation.
--
-- Squashed end-state of the EHRbase v2.33.0 `ext` Flyway chain (V1..V4),
-- derived by applying the vendored chain to PostgreSQL 18 and dumping the
-- result (ADR-007). The original chain is kept as a test fixture under
-- tests/resources/legacy_schema/ext/; a schema-equality test asserts this
-- baseline produces the identical schema.
--
-- Original SQL: Copyright (c) 2024 vitasystems GmbH, Apache License 2.0.
--
-- Runs with search_path = ext (see src/db/migrate.rs). Extensions
-- (uuid-ossp, pgcrypto, pg_trgm) are created by the bootstrap, not here.

-- The `en_US` ICU collation. NOTE: table columns declared `COLLATE "en_US"`
-- actually resolve to the pg_catalog `en_US` collation (pg_catalog precedes
-- every search_path); this ext-schema copy exists because the legacy chain
-- created it, and is kept for schema equality.
CREATE COLLATION IF NOT EXISTS "en_US" (provider = icu, locale = 'en-US');

-- ── jsonb min/max support (AQL MAX/MIN over raw jsonb values) ───────────────

CREATE FUNCTION jsonb_larger(j1 jsonb, j2 jsonb) RETURNS jsonb
    LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE
    AS $$
BEGIN
    IF j1 > j2 THEN
        RETURN j1;
    ELSE
        RETURN j2;
    END IF;
END;
$$;

CREATE FUNCTION jsonb_smaller(j1 jsonb, j2 jsonb) RETURNS jsonb
    LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE
    AS $$
BEGIN
    IF j1 < j2 THEN
        RETURN j1;
    ELSE
        RETURN j2;
    END IF;
END;
$$;

-- ── jsonb SUM/AVG support (accumulate numeric jsonb values) ─────────────────

CREATE FUNCTION jsonb_avg_acc(s numeric[], j2 jsonb) RETURNS numeric[]
    LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE
    AS $$
BEGIN
    IF jsonb_typeof(j2) = 'number'::text THEN
        RETURN s || j2::numeric;
    ELSE
        RETURN s;
    END IF;
END;
$$;

CREATE FUNCTION jsonb_avg_combine(s1 numeric[], s2 numeric[]) RETURNS numeric[]
    LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE
    AS $$
BEGIN
    RETURN s1 || s2;
END;
$$;

CREATE FUNCTION jsonb_avg(s numeric[]) RETURNS jsonb
    LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE
    AS $$
DECLARE
    len numeric;
    sum numeric := 0;
    x numeric;
BEGIN
    len := COALESCE(array_length(s,1),0)::numeric;
    IF len > 0 THEN
        FOREACH x IN ARRAY s LOOP
                sum := sum + x;
            END LOOP;
        RETURN to_jsonb(sum/len);
    ELSE
        RETURN NULL;
    END IF;
END;
$$;

CREATE FUNCTION jsonb_sum(s numeric[]) RETURNS jsonb
    LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE
    AS $$
DECLARE
    sum numeric := 0;
    x numeric;
BEGIN
    IF COALESCE(array_length(s,1),0) > 0 THEN
        FOREACH x IN ARRAY s LOOP
                sum := sum + x;
            END LOOP;
        RETURN to_jsonb(sum);
    ELSE
        RETURN NULL;
    END IF;
END;
$$;

-- ── DV_ORDERED min/max support (magnitude-aware comparison on the aliased
--    db-format: 'T' = type alias, 'm'/'M'/'V' = magnitude/value fields) ──────

CREATE FUNCTION jsonb_dv_ordered_magnitude(dv jsonb) RETURNS jsonb
    LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE
    AS $$
BEGIN
    CASE dv ->> 'T'
        WHEN 'q', 'co' THEN
            RETURN dv -> 'm';
        WHEN 'pr', 't', 'd', 'dt', 'du' THEN
            RETURN dv -> 'M';
        WHEN 'sc', 'o' THEN
            RETURN dv -> 'V';
        ELSE
            RETURN null;
        END CASE;
END;
$$;

CREATE FUNCTION dv_ordered_larger(j1 jsonb, j2 jsonb) RETURNS jsonb
    LANGUAGE plpgsql IMMUTABLE PARALLEL SAFE
    AS $$
DECLARE
    m1 jsonb:= jsonb_dv_ordered_magnitude(j1);
    m2 jsonb:= jsonb_dv_ordered_magnitude(j2);
    cond boolean := m1 > m2;
BEGIN
    IF cond THEN
        RETURN j1;
    ELSEIF cond IS NOT NULL THEN
        RETURN j2;
    ELSEIF m1 IS NOT NULL THEN
        RETURN j1;
    ELSEIF m2 IS NOT NULL THEN
        RETURN j2;
    ELSE
        RETURN NULL;
    END IF;
END;
$$;

CREATE FUNCTION dv_ordered_smaller(j1 jsonb, j2 jsonb) RETURNS jsonb
    LANGUAGE plpgsql IMMUTABLE PARALLEL SAFE
    AS $$
DECLARE
    m1 jsonb:= jsonb_dv_ordered_magnitude(j1);
    m2 jsonb:= jsonb_dv_ordered_magnitude(j2);
    cond boolean := m1 < m2;
BEGIN
    IF cond THEN
        RETURN j1;
    ELSEIF cond IS NOT NULL THEN
        RETURN j2;
    ELSEIF m1 IS NOT NULL THEN
        RETURN j1;
    ELSEIF m2 IS NOT NULL THEN
        RETURN j2;
    ELSE
        RETURN NULL;
    END IF;
END;
$$;

-- ── The aggregates the AQL engine calls (ext.max/min/sum/avg + dv_ordered) ──

CREATE AGGREGATE max(jsonb) (
    SFUNC = jsonb_larger,
    STYPE = jsonb,
    COMBINEFUNC = jsonb_larger,
    SORTOP = OPERATOR(pg_catalog.>),
    PARALLEL = safe
);

CREATE AGGREGATE min(jsonb) (
    SFUNC = jsonb_smaller,
    STYPE = jsonb,
    COMBINEFUNC = jsonb_smaller,
    SORTOP = OPERATOR(pg_catalog.<),
    PARALLEL = safe
);

CREATE AGGREGATE max_dv_ordered(jsonb) (
    SFUNC = dv_ordered_larger,
    STYPE = jsonb,
    COMBINEFUNC = dv_ordered_larger,
    SORTOP = OPERATOR(pg_catalog.>),
    PARALLEL = safe
);

CREATE AGGREGATE min_dv_ordered(jsonb) (
    SFUNC = dv_ordered_smaller,
    STYPE = jsonb,
    COMBINEFUNC = dv_ordered_smaller,
    SORTOP = OPERATOR(pg_catalog.<),
    PARALLEL = safe
);

CREATE AGGREGATE sum(jsonb) (
    SFUNC = jsonb_avg_acc,
    STYPE = numeric[],
    INITCOND = '{}',
    FINALFUNC = jsonb_sum,
    COMBINEFUNC = jsonb_avg_combine,
    PARALLEL = safe
);

CREATE AGGREGATE avg(jsonb) (
    SFUNC = jsonb_avg_acc,
    STYPE = numeric[],
    INITCOND = '{}',
    FINALFUNC = jsonb_avg,
    COMBINEFUNC = jsonb_avg_combine,
    PARALLEL = safe
);
