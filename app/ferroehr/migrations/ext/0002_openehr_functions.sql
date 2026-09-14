-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- ext: the openEHR value helpers the AQL emitter and the node decomposer call.
--
-- Carried unchanged from the first storage generation: the bodies are the
-- shipped ones.
-- TODO(#3351): re-author these without subtransactions (LANGUAGE sql,
-- regex-validated input, no EXCEPTION block).
--
-- Runs with search_path = ext.
-- days since 0001-01-01 for an ISO-8601 (possibly partial) date string
CREATE FUNCTION openehr_date_days(v text) RETURNS numeric
LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE AS $$
DECLARE
    y integer; m integer := 1; d integer := 1;
    s text := replace(v, '-', '');
BEGIN
    IF length(s) < 4 THEN RETURN NULL; END IF;
    y := substring(s FROM 1 FOR 4)::integer;
    IF length(s) >= 6 THEN m := substring(s FROM 5 FOR 2)::integer; END IF;
    IF length(s) >= 8 THEN d := substring(s FROM 7 FOR 2)::integer; END IF;
    RETURN make_date(y, m, d) - DATE '0001-01-01';
EXCEPTION WHEN others THEN
    RETURN NULL;
END $$;

-- seconds since start of day for an ISO-8601 (possibly partial) time
-- string, ignoring any timezone suffix (callers handle offsets)
CREATE FUNCTION openehr_time_seconds(v text) RETURNS numeric
LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE AS $$
DECLARE
    t text := regexp_replace(v, '([Zz]|[+-]\d{2}:?\d{0,2})$', '');
    parts text[];
    h numeric := 0; m numeric := 0; s numeric := 0;
BEGIN
    t := regexp_replace(t, '^[Tt]', '');
    IF t !~ ':' AND length(t) > 2 THEN
        -- compact HH[MM[SS]]
        h := substring(t FROM 1 FOR 2)::numeric;
        IF length(t) >= 4 THEN m := substring(t FROM 3 FOR 2)::numeric; END IF;
        IF length(t) >= 6 THEN s := substring(t FROM 5)::numeric; END IF;
    ELSE
        parts := string_to_array(t, ':');
        IF array_length(parts, 1) >= 1 AND parts[1] <> '' THEN h := parts[1]::numeric; END IF;
        IF array_length(parts, 1) >= 2 THEN m := parts[2]::numeric; END IF;
        IF array_length(parts, 1) >= 3 THEN s := parts[3]::numeric; END IF;
    END IF;
    RETURN h * 3600 + m * 60 + s;
EXCEPTION WHEN others THEN
    RETURN NULL;
END $$;

-- timezone offset in seconds from an ISO-8601 suffix (Z / ±HH[:MM]); 0 when absent
CREATE FUNCTION openehr_tz_offset_seconds(v text) RETURNS numeric
LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE AS $$
DECLARE
    m text[];
BEGIN
    IF v ~ '[Zz]$' THEN RETURN 0; END IF;
    m := regexp_match(v, '([+-])(\d{2}):?(\d{2})?$');
    IF m IS NULL THEN RETURN 0; END IF;
    RETURN (CASE m[1] WHEN '-' THEN -1 ELSE 1 END)
         * (m[2]::numeric * 3600 + coalesce(m[3], '0')::numeric * 60);
EXCEPTION WHEN others THEN
    RETURN 0;
END $$;

-- seconds since 0001-01-01T00:00:00Z for an ISO-8601 (possibly partial)
-- date-time string
CREATE FUNCTION openehr_date_time_seconds(v text) RETURNS numeric
LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE AS $$
DECLARE
    date_part text := split_part(v, 'T', 1);
    time_part text := split_part(v, 'T', 2);
    days numeric;
    sod numeric := 0;
    off numeric := 0;
BEGIN
    days := openehr_date_days(date_part);
    IF days IS NULL THEN RETURN NULL; END IF;
    IF time_part <> '' THEN
        sod := coalesce(openehr_time_seconds(time_part), 0);
        off := openehr_tz_offset_seconds(time_part);
    END IF;
    RETURN days * 86400 + sod - off;
END $$;

-- seconds for an ISO-8601 duration, using openEHR nominal year/month lengths
CREATE FUNCTION openehr_duration_seconds(v text) RETURNS numeric
LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE AS $$
DECLARE
    m text[];
    sign numeric := 1;
    date_part text;
    time_part text;
    total numeric := 0;
BEGIN
    m := regexp_match(v, '^(-)?P([^T]*)(?:T(.*))?$');
    IF m IS NULL THEN RETURN NULL; END IF;
    IF m[1] = '-' THEN sign := -1; END IF;
    date_part := coalesce(m[2], '');
    time_part := coalesce(m[3], '');

    total := total + coalesce((regexp_match(date_part, '(\d+(?:\.\d+)?)Y'))[1]::numeric, 0) * 365.24 * 86400;
    total := total + coalesce((regexp_match(date_part, '(\d+(?:\.\d+)?)M'))[1]::numeric, 0) * 30.42 * 86400;
    total := total + coalesce((regexp_match(date_part, '(\d+(?:\.\d+)?)W'))[1]::numeric, 0) * 7 * 86400;
    total := total + coalesce((regexp_match(date_part, '(\d+(?:\.\d+)?)D'))[1]::numeric, 0) * 86400;
    total := total + coalesce((regexp_match(time_part, '(\d+(?:\.\d+)?)H'))[1]::numeric, 0) * 3600;
    total := total + coalesce((regexp_match(time_part, '(\d+(?:\.\d+)?)M'))[1]::numeric, 0) * 60;
    total := total + coalesce((regexp_match(time_part, '(\d+(?:\.\d+)?)S'))[1]::numeric, 0);
    RETURN sign * total;
EXCEPTION WHEN others THEN
    RETURN NULL;
END $$;

-- the ordered magnitude of a canonical DV_ORDERED JSON value
CREATE FUNCTION openehr_magnitude(dv jsonb) RETURNS numeric
LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE AS $$
DECLARE
    t text := dv->>'_type';
    denom numeric;
BEGIN
    CASE t
        WHEN 'DV_QUANTITY', 'DV_COUNT' THEN
            RETURN (dv->>'magnitude')::numeric;
        WHEN 'DV_ORDINAL', 'DV_SCALE' THEN
            RETURN (dv->>'value')::numeric;
        WHEN 'DV_PROPORTION' THEN
            denom := (dv->>'denominator')::numeric;
            IF denom = 0 THEN RETURN NULL; END IF; -- NOTE: NULL, not MAX
            RETURN (dv->>'numerator')::numeric / denom;
        WHEN 'DV_DATE' THEN
            RETURN openehr_date_days(dv->>'value');
        WHEN 'DV_TIME' THEN
            RETURN openehr_time_seconds(dv->>'value');
        WHEN 'DV_DATE_TIME' THEN
            RETURN openehr_date_time_seconds(dv->>'value');
        WHEN 'DV_DURATION' THEN
            RETURN openehr_duration_seconds(dv->>'value');
        ELSE
            RETURN NULL;
    END CASE;
EXCEPTION WHEN others THEN
    RETURN NULL;
END $$;

-- ── Function documentation ─────────────────────────────────────
COMMENT ON FUNCTION ext.openehr_date_days(text) IS
    'Days since 0001-01-01 for an ISO-8601 (possibly partial) date string; NULL on unparseable input. Partial dates assume the first month/day. IMMUTABLE — index-legal.';
COMMENT ON FUNCTION ext.openehr_time_seconds(text) IS
    'Seconds since start of day for an ISO-8601 (possibly partial) time string, ignoring any timezone suffix (callers apply the offset). Partial times assume 0. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_tz_offset_seconds(text) IS
    'Timezone offset in seconds parsed from an ISO-8601 suffix (Z / ±HH[:MM]); 0 when absent. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_date_time_seconds(text) IS
    'Seconds since 0001-01-01T00:00:00Z for an ISO-8601 (possibly partial) date-time string; NULL on unparseable date. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_duration_seconds(text) IS
    'Seconds for an ISO-8601 duration using openEHR *nominal* lengths (year = 365.24 d, month = 30.42 d); NULL on unparseable input. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_magnitude(jsonb) IS
    'The ordered magnitude (numeric) of a canonical DV_ORDERED value, per the per-subtype comparison the RM defines (RM data_types master06-quantity_package.adoc + the DV_* class tables). NULL for non-ordered or unparseable values. IMMUTABLE + PARALLEL SAFE so it is legal in btree expression indexes.';

-- ext.openehr_timestamp — a total ISO-8601 → timestamptz reading.
--
-- No openEHR spec governs storage columns or their derivation — this is our own
-- design (docs/architecture.md §Storage). It is the ONE partial-precision
-- semantics the product carries on the temporal read/write paths: the promoted
-- timestamp columns (node.context_start) are populated through it at write +
-- backfill, and the AQL engine's temporal comparisons/orderings
-- (app/ferroehr/src/aql/sql/value.rs, Coercion::Temporal) extract through it —
-- so the promoted fast path and the jsonb lowering can never disagree.
--
-- Full-precision input takes PostgreSQL's own text::timestamptz parser
-- verbatim. RM data_types master07 §Partial Date/Times admits reduced
-- precision (`2021`, `1985-06`, `12:00`), which that parser refuses — a valid
-- stored value must never make a query error (SQLSTATE 22007 answered the
-- caller a 400 for a data property) — so partial precision is FLOOR-completed
-- instead: a partial date assumes the first month/day, a partial time assumes
-- 0, a time-only value anchors on 0001-01-01 (the same floor the
-- openehr_date_days/time_seconds family documents, and the completion
-- strategy prior art converged on). Mixed-precision ordering is genuinely
-- unspecified upstream (confirmed report #1493 / register AMB-29); the floor
-- is our own recorded semantics and does not depend on its resolution.
-- Malformed input returns NULL (a comparison miss), never an error.
-- STRICT short-circuits NULL input (a persistent COMPOSITION without context,
-- RM ehr master03 §COMPOSITION.context [0..1]) with no subtransaction cost.
--
-- STABLE, not IMMUTABLE: text::timestamptz (and the offset-less completion
-- branch) depend on the session TimeZone, so this must not back an expression
-- index; it runs in DML and query expressions, where STABLE is correct.
-- Values carrying an explicit offset/Z are TimeZone-independent.
-- ISO 8601 permits a COMMA decimal sign on the fractional second (BASE
-- foundation_types master06), which PostgreSQL rejects; a comma cannot occur
-- elsewhere in a valid ISO value, so it normalizes to the dot first.
CREATE FUNCTION openehr_timestamp(v text) RETURNS timestamptz
LANGUAGE plpgsql STABLE STRICT PARALLEL SAFE AS $$
DECLARE
    s text := replace(v, ',', '.');
    date_part text;
    time_part text;
    dcomp text;
    y int; mo int := 1; d int := 1;
    sod numeric := 0;
    ts timestamp;
BEGIN
    BEGIN
        RETURN s::timestamptz;
    EXCEPTION WHEN others THEN
        NULL; -- reduced precision or malformed: fall through to the completion
    END;
    IF s ~ '^[Tt]' OR s ~ '^\d{1,2}:' THEN
        -- time-only: floor onto 0001-01-01
        time_part := regexp_replace(s, '^[Tt]', '');
        y := 1;
    ELSE
        date_part := split_part(s, 'T', 1);
        time_part := split_part(s, 'T', 2);
        dcomp := replace(date_part, '-', '');
        IF dcomp !~ '^(\d{4}|\d{6}|\d{8})$' THEN
            RETURN NULL;
        END IF;
        y := substring(dcomp FROM 1 FOR 4)::int;
        IF length(dcomp) >= 6 THEN mo := substring(dcomp FROM 5 FOR 2)::int; END IF;
        IF length(dcomp) >= 8 THEN d := substring(dcomp FROM 7 FOR 2)::int; END IF;
    END IF;
    IF time_part <> '' THEN
        sod := openehr_time_seconds(time_part);
        IF sod IS NULL THEN RETURN NULL; END IF;
    END IF;
    ts := make_date(y, mo, d)::timestamp + make_interval(secs => sod::double precision);
    IF time_part ~ '([Zz]|[+-]\d{2}:?\d{0,2})$' THEN
        RETURN (ts - make_interval(secs => openehr_tz_offset_seconds(time_part)::double precision))
               AT TIME ZONE 'UTC';
    END IF;
    -- offset-less: session-TimeZone reading, matching the native-cast branch
    RETURN ts::timestamptz;
EXCEPTION WHEN others THEN
    RETURN NULL;
END;
$$;

COMMENT ON FUNCTION ext.openehr_timestamp(text) IS
    'Total ISO-8601 text -> timestamptz: PostgreSQL''s own parser for full precision, floor completion for reduced precision (partial dates assume the first month/day, partial times 0, time-only values anchor on 0001-01-01), NULL for malformed input — never an error. The ONE partial-temporal semantics: feeds the promoted timestamp columns (node.context_start) AND the AQL temporal coercion. STABLE (TimeZone-dependent for offset-less inputs); not legal in index expressions.';


-- The runtime writers execute these on the write path (promoted-column
-- population) and the readers on the AQL path; ext/0001 already installed
-- ALTER DEFAULT PRIVILEGES for functions the migrator creates in this schema,
-- so no further grant is issued here.
