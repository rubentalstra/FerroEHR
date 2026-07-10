-- ext schema: openEHR support functions + the cluster role/grant baseline
-- (ADR-008 functions; ADR-013 enterprise baseline).
--
-- Re-authored as the single squashed `ext` baseline (ADR-013 §1, review doc
-- 02 §5.1 — append-only forever after). This migrator runs BEFORE `ehr`
-- (db/migrate.rs), so the three application roles are created HERE, ahead of
-- every GRANT in either schema.
--
-- All functions are IMMUTABLE + PARALLEL SAFE so they are legal in btree
-- expression indexes — the ADR-008 pattern: magnitudes are computed (and
-- indexed on demand for measured hot paths, see the ehr baseline's
-- `idx_node_magnitude`), never stored as synthetic fields inside the canonical
-- data. Runs with search_path = ext.
--
-- Magnitude semantics follow the openEHR RM spec for DV_ORDERED:
--   DV_QUANTITY / DV_COUNT   -> magnitude
--   DV_ORDINAL / DV_SCALE    -> value
--   DV_PROPORTION            -> numerator / denominator (NULL for /0)
--   DV_DATE                  -> days since 0001-01-01
--   DV_TIME                  -> seconds since start of day (fractions kept)
--   DV_DATE_TIME             -> seconds since 0001-01-01T00:00:00Z
--   DV_DURATION              -> seconds, with openEHR *nominal* lengths
--                               (year = 365.24 d, month = 30.42 d)
-- Partial dates assume the first month/day; partial times assume 0.

-- ── Roles (ADR-013 §3, review doc 02 §3.1) ───────────────────────────────────
-- Three NOLOGIN group roles, granted at deploy time to the concrete LOGIN
-- roles (passwords/LOGIN/pg_hba/TLS are deployment-layer, review doc 02 §3.6):
--   * ehrbase_migrator — owns the schema objects and runs DDL (this migration);
--   * ehrbase_app      — the runtime writer (DML on ehr.*);
--   * ehrbase_reader   — read-only (SELECT), e.g. reporting/analytics.
-- Never run the application as a superuser. Idempotent so re-running the
-- migrator (or the ehr baseline's mirror block) is a no-op.
DO $$
BEGIN
    -- Graceful degradation (deployment reality): when the migration runs as a
    -- user without CREATEROLE (dev/compose/testcontainers or a managed PG
    -- without role rights), the role architecture is skipped with a NOTICE —
    -- it is then a deployment-layer setup step (review doc 02 §3.1). When the
    -- migrator has the privilege (production), roles are created idempotently.
    BEGIN
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ehrbase_migrator') THEN
            CREATE ROLE ehrbase_migrator NOLOGIN;
        END IF;
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ehrbase_app') THEN
            CREATE ROLE ehrbase_app NOLOGIN;
        END IF;
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ehrbase_reader') THEN
            CREATE ROLE ehrbase_reader NOLOGIN;
        END IF;
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'skipping role creation (no CREATEROLE privilege): create ehrbase_migrator/ehrbase_app/ehrbase_reader at deployment';
    END;
END $$;

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
            IF denom = 0 THEN RETURN NULL; END IF; -- PORT NOTE: NULL, not MAX
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

-- ── Function documentation (ADR-013 §12) ─────────────────────────────────────
COMMENT ON FUNCTION ext.openehr_date_days(text) IS
    'Days since 0001-01-01 for an ISO-8601 (possibly partial) date string; NULL on unparseable input. Partial dates assume the first month/day. IMMUTABLE — index-legal (ADR-008).';
COMMENT ON FUNCTION ext.openehr_time_seconds(text) IS
    'Seconds since start of day for an ISO-8601 (possibly partial) time string, ignoring any timezone suffix (callers apply the offset). Partial times assume 0. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_tz_offset_seconds(text) IS
    'Timezone offset in seconds parsed from an ISO-8601 suffix (Z / ±HH[:MM]); 0 when absent. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_date_time_seconds(text) IS
    'Seconds since 0001-01-01T00:00:00Z for an ISO-8601 (possibly partial) date-time string; NULL on unparseable date. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_duration_seconds(text) IS
    'Seconds for an ISO-8601 duration using openEHR *nominal* lengths (year = 365.24 d, month = 30.42 d); NULL on unparseable input. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_magnitude(jsonb) IS
    'The ordered magnitude (numeric) of a canonical DV_ORDERED value, per the RM DV_ORDERED comparison semantics (ADR-008 §2). NULL for non-ordered or unparseable values. IMMUTABLE + PARALLEL SAFE so it is legal in btree expression indexes (see ehr.idx_node_magnitude).';

-- ── Grants (ADR-013 §3, review doc 02 §3.1/§3.6) ─────────────────────────────
-- The `ext` functions are on the READ path (AQL magnitude ordering); the app
-- and reader roles need USAGE on the schema and EXECUTE on the functions, and
-- nothing more (functions are plain, not SECURITY DEFINER — review doc 02 §3.6).
-- The migrator owns them. Idempotent by construction (GRANT is a no-op if
-- already held).
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ehrbase_app') THEN
        GRANT USAGE ON SCHEMA ext TO ehrbase_app, ehrbase_reader;
        GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA ext TO ehrbase_app, ehrbase_reader;
        -- Future ext functions reachable without a manual grant (doc 02 §3.2).
        ALTER DEFAULT PRIVILEGES IN SCHEMA ext
            GRANT EXECUTE ON FUNCTIONS TO ehrbase_app, ehrbase_reader;
    ELSE
        RAISE NOTICE 'skipping ext grants (roles absent — see the role block NOTICE)';
    END IF;
END $$;
