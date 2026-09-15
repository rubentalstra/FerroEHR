-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- ext: the openEHR value helpers the AQL emitter and the node decomposer call.
--
-- No openEHR spec governs storage helpers: our own design. What they compute is
-- spec-grounded — the ordered magnitude of a DV_ORDERED (RM data_types
-- master06-quantity_package.adoc) and the ISO 8601 date/time/duration forms
-- openEHR uses (BASE foundation_types master06-time_types.adoc and the
-- Iso8601_date / Iso8601_date_time class tables under BASE docs/UML/classes/) —
-- but nothing in openEHR says a database should carry them.
--
-- NO HELPER TRAPS AN ERROR. Not one carries an `EXCEPTION` clause, because the
-- documentation is direct about what that costs: "A block containing an
-- EXCEPTION clause is significantly more expensive to enter and exit than a
-- block without one. Therefore, don't use EXCEPTION without need."
-- (PostgreSQL 18, "Control Structures",
-- https://www.postgresql.org/docs/18/plpgsql-control-structures.html). A helper
-- is read once per candidate row per AQL predicate, so a per-call subtransaction
-- is exactly the cost this file exists to avoid. Every cast is instead reachable
-- only through a guard that proves it cannot raise, and a value the guard
-- refuses reads as NULL — a comparison miss, never an error the caller sees as
-- a 500.
--
-- The guards are ordered, never conjunctions: SQL fixes no evaluation order
-- between the operands of AND (PostgreSQL 18, "Expression Evaluation Rules",
-- https://www.postgresql.org/docs/18/sql-expressions.html), so a cast is only
-- ever reached from inside a CASE arm or a nested IF whose condition has
-- already proved it safe.
--
-- LANGUAGE IS A MEASURED CHOICE PER HELPER, not a house style. A `LANGUAGE sql`
-- body folds into the calling query, which removes the call but substitutes the
-- argument expression at every parameter reference; a PL/pgSQL body reads its
-- argument into a local variable once but is always a call. Which wins depends
-- on how many times the body reads its input, so each helper ships in the form
-- that measured faster over a 50 000-row corpus of the values it actually sees:
-- `sql` for the four that read their input once or twice, `plpgsql` for the two
-- date/time parsers that read it a dozen times. The corpus test pins the
-- results, not the languages.
--
-- The PL/pgSQL bodies SCHEMA-QUALIFY every helper they call. PL/pgSQL resolves
-- function names at execution time against the caller's search_path
-- (PostgreSQL 18, "Writing SECURITY DEFINER Functions Safely",
-- https://www.postgresql.org/docs/18/sql-createfunction.html), so an
-- unqualified call can be captured by a schema the caller put first; the
-- `LANGUAGE sql` bodies bind at definition time and need no such care.
--
-- Volatility is unchanged from the first generation, and it is load-bearing:
-- the parsers and ext.openehr_magnitude are IMMUTABLE, so they are legal in an
-- index expression, while ext.openehr_timestamp is STABLE because its
-- offset-less reading depends on the session TimeZone — "a function that
-- manipulates timestamps might well have results that depend on the TimeZone
-- setting. For safety, such functions should be labeled STABLE instead."
-- (PostgreSQL 18, "Function Volatility Categories",
-- https://www.postgresql.org/docs/18/xfunc-volatility.html).
--
-- Runs with search_path = ext.

-- The numeric value of a canonical JSON number's text.
--
-- substring() with a pattern yields NULL when the pattern does not match, so
-- the guard and the extraction are the same operation and the parameter is read
-- once, which is what lets this fold into its caller for free. The accepted
-- shape is JSON's own number grammar (RFC 8259 §6,
-- https://www.rfc-editor.org/rfc/rfc8259#section-6) plus the leading minus.
CREATE FUNCTION openehr_numeric(t text) RETURNS numeric
LANGUAGE sql IMMUTABLE PARALLEL SAFE
RETURN substring(t FROM '^-?[0-9]+(?:\.[0-9]+)?(?:[eE][-+]?[0-9]+)?$')::numeric;

-- Days since 0001-01-01 for an ISO 8601 date in either format.
--
-- BASE foundation_types master06-time_types.adoc admits reduced precision down
-- to the year, so `YYYY` and `YYYY-MM` read as the first month and first day;
-- removing the hyphens folds the extended format (`YYYY-MM-DD`, the form BASE
-- calls preferred) onto the basic one, so one reading serves both.
--
-- PL/pgSQL, because the reading looks at its input a dozen times and a folded
-- body would re-evaluate the caller's jsonb extraction once per look.
--
-- make_date() raises on a triple that is not a real date, so the calendar is
-- checked first, as text: the fields are zero-padded, so a text comparison
-- orders them exactly as the numbers do and costs less than a cast. A day at or
-- below 28 is valid in every month and needs no month length at all; only a
-- later day pays for one, with February resolved by the Gregorian leap rule,
-- which PostgreSQL applies to every year because it "uses the Gregorian
-- calendar for all dates and times" (PostgreSQL 18, "Date/Time Support",
-- https://www.postgresql.org/docs/18/datetime-appendix.html). translate() is
-- the digit check: what it leaves after removing digits is empty only when
-- every character was one. Year 0000 is refused because make_date() has no
-- year zero.
CREATE FUNCTION openehr_date_days(v text) RETURNS numeric
LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE AS $$
DECLARE
    s text := replace(v, '-', '');
BEGIN
    IF length(s) >= 8 THEN
        IF translate(substr(s, 1, 8), '0123456789', '') = ''
           AND substr(s, 1, 4) <> '0000'
           AND substr(s, 5, 2) BETWEEN '01' AND '12'
           AND substr(s, 7, 2) >= '01'
        THEN
        IF substr(s, 7, 2) <= '28'
           OR substr(s, 7, 2) <= (CASE substr(s, 5, 2)
                   WHEN '02' THEN CASE WHEN (substr(s, 1, 4)::integer % 4 = 0
                                             AND substr(s, 1, 4)::integer % 100 <> 0)
                                            OR substr(s, 1, 4)::integer % 400 = 0
                                       THEN '29' ELSE '28' END
                   WHEN '04' THEN '30' WHEN '06' THEN '30'
                   WHEN '09' THEN '30' WHEN '11' THEN '30' ELSE '31' END)
        THEN
            RETURN make_date(substr(s, 1, 4)::integer, substr(s, 5, 2)::integer,
                             substr(s, 7, 2)::integer) - DATE '0001-01-01';
        END IF;
        END IF;
        RETURN NULL;
    END IF;
    IF length(s) >= 6 THEN
        IF translate(substr(s, 1, 6), '0123456789', '') <> ''
           OR substr(s, 1, 4) = '0000'
           OR substr(s, 5, 2) NOT BETWEEN '01' AND '12'
        THEN RETURN NULL; END IF;
        RETURN make_date(substr(s, 1, 4)::integer, substr(s, 5, 2)::integer, 1)
               - DATE '0001-01-01';
    END IF;
    IF length(s) >= 4 THEN
        IF translate(substr(s, 1, 4), '0123456789', '') <> ''
           OR substr(s, 1, 4) = '0000'
        THEN RETURN NULL; END IF;
        RETURN make_date(substr(s, 1, 4)::integer, 1, 1) - DATE '0001-01-01';
    END IF;
    RETURN NULL;
END $$;

-- Seconds since the start of the day for an ISO 8601 time, ignoring any zone
-- suffix — the caller applies the offset through openehr_tz_offset_seconds.
--
-- PL/pgSQL for the same reason as the date reading. The `T` designator and the
-- zone suffix are cut positionally rather than by substitution, and the two
-- field forms BASE foundation_types master06-time_types.adoc defines are split
-- on whether a colon is present: the basic `hhmmss` (which needs a length above
-- two, or `hh` would read as the first field of the extended form) and the
-- extended `hh:mm:ss`. A field is accepted when it is digits with at most one
-- decimal point, which is the fractional second the same section allows on the
-- last field; fields the input omits read as zero, which is the reduced
-- precision it also admits.
CREATE FUNCTION openehr_time_seconds(v text) RETURNS numeric
LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE AS $$
DECLARE
    t     text := v;
    zone  text;
    field text;
    parts text[];
    h numeric := 0; m numeric := 0; s numeric := 0;
BEGIN
    IF left(t, 1) IN ('T', 't') THEN t := substr(t, 2); END IF;
    zone := substring(t FROM '([Zz]|[+-][0-9]{2}:?[0-9]{0,2})$');
    IF zone IS NOT NULL THEN t := substr(t, 1, length(t) - length(zone)); END IF;

    IF position(':' in t) = 0 AND length(t) > 2 THEN
        IF translate(substr(t, 1, 2), '0123456789', '') <> '' THEN RETURN NULL; END IF;
        h := substr(t, 1, 2)::numeric;
        IF length(t) >= 4 THEN
            IF translate(substr(t, 3, 2), '0123456789', '') <> '' THEN RETURN NULL; END IF;
            m := substr(t, 3, 2)::numeric;
        END IF;
        IF length(t) >= 6 THEN
            field := substr(t, 5);
            IF field = '.' OR translate(field, '0123456789', '') NOT IN ('', '.')
            THEN RETURN NULL; END IF;
            s := field::numeric;
        END IF;
        RETURN h * 3600 + m * 60 + s;
    END IF;

    parts := string_to_array(t, ':');
    IF parts[1] <> '' THEN
        IF parts[1] = '.' OR translate(parts[1], '0123456789', '') NOT IN ('', '.')
        THEN RETURN NULL; END IF;
        h := parts[1]::numeric;
    END IF;
    IF array_length(parts, 1) >= 2 THEN
        IF parts[2] = '' OR parts[2] = '.'
           OR translate(parts[2], '0123456789', '') NOT IN ('', '.')
        THEN RETURN NULL; END IF;
        m := parts[2]::numeric;
    END IF;
    IF array_length(parts, 1) >= 3 THEN
        IF parts[3] = '' OR parts[3] = '.'
           OR translate(parts[3], '0123456789', '') NOT IN ('', '.')
        THEN RETURN NULL; END IF;
        s := parts[3]::numeric;
    END IF;
    RETURN h * 3600 + m * 60 + s;
END $$;

-- The zone offset in seconds carried by an ISO 8601 time's suffix; zero when
-- the value carries none.
--
-- `Z` is zero by definition (BASE foundation_types master06-time_types.adoc:
-- "'Z' and 'T' are literals"), and testing the last character for it answers
-- the common case before any pattern runs. `±hh`, `±hhmm` and `±hh:mm` are read
-- from the end of the value. NULL input is answered first, because the trailing
-- zero-offset arm would otherwise turn it into a number.
CREATE FUNCTION openehr_tz_offset_seconds(v text) RETURNS numeric
LANGUAGE sql IMMUTABLE PARALLEL SAFE
RETURN CASE
    WHEN v IS NULL THEN NULL
    WHEN right(v, 1) IN ('Z', 'z') THEN 0
    WHEN v ~ '[+-][0-9]{2}:?([0-9]{2})?$' THEN
        (CASE WHEN substring(v FROM '([+-])[0-9]{2}:?(?:[0-9]{2})?$') = '-' THEN -1 ELSE 1 END)
      * (substring(v FROM '[+-]([0-9]{2}):?(?:[0-9]{2})?$')::numeric * 3600
         + coalesce(substring(v FROM '[+-][0-9]{2}:?([0-9]{2})$'), '0')::numeric * 60)
    ELSE 0
END;

-- Seconds since 0001-01-01T00:00:00Z for an ISO 8601 date-time.
--
-- An unreadable date makes the whole value unreadable; an absent or unreadable
-- time part leaves the reading at the start of the day, which is the reduced
-- precision BASE foundation_types master06-time_types.adoc admits.
CREATE FUNCTION openehr_date_time_seconds(v text) RETURNS numeric
LANGUAGE sql IMMUTABLE PARALLEL SAFE
RETURN openehr_date_days(split_part(v, 'T', 1)) * 86400
     + CASE WHEN split_part(v, 'T', 2) <> ''
            THEN coalesce(openehr_time_seconds(split_part(v, 'T', 2)), 0)
                 - openehr_tz_offset_seconds(split_part(v, 'T', 2))
            ELSE 0 END;

-- Seconds for an ISO 8601 duration, using openEHR's definite-arithmetic
-- averages.
--
-- BASE foundation_types master06-time_types.adoc §Computational Functions
-- distinguishes definite arithmetic, which treats all values as "exact and
-- invariant, based on constant values for length of year and month, defined by
-- Time_definitions.Average_days_in_month and
-- Time_definitions.Average_days_in_year", from nominal calendar arithmetic; an
-- ordering key can only be the definite one, so the averages below are those
-- two constants. The designator letters are literals of the same section.
CREATE FUNCTION openehr_duration_seconds(v text) RETURNS numeric
LANGUAGE sql IMMUTABLE PARALLEL SAFE
RETURN CASE WHEN v ~ '^-?P[^T]*(T.*)?$' THEN
    (CASE WHEN left(v, 1) = '-' THEN -1 ELSE 1 END)
  * (coalesce(substring(substring(v FROM '^-?P([^T]*)') FROM '([0-9]+(\.[0-9]+)?)Y')::numeric, 0) * 365.24 * 86400
   + coalesce(substring(substring(v FROM '^-?P([^T]*)') FROM '([0-9]+(\.[0-9]+)?)M')::numeric, 0) * 30.42 * 86400
   + coalesce(substring(substring(v FROM '^-?P([^T]*)') FROM '([0-9]+(\.[0-9]+)?)W')::numeric, 0) * 7 * 86400
   + coalesce(substring(substring(v FROM '^-?P([^T]*)') FROM '([0-9]+(\.[0-9]+)?)D')::numeric, 0) * 86400
   + coalesce(substring(substring(v FROM '^-?P[^T]*T(.*)$') FROM '([0-9]+(\.[0-9]+)?)H')::numeric, 0) * 3600
   + coalesce(substring(substring(v FROM '^-?P[^T]*T(.*)$') FROM '([0-9]+(\.[0-9]+)?)M')::numeric, 0) * 60
   + coalesce(substring(substring(v FROM '^-?P[^T]*T(.*)$') FROM '([0-9]+(\.[0-9]+)?)S')::numeric, 0))
END;

-- The ordered magnitude of a canonical DV_ORDERED value.
--
-- RM data_types master06-quantity_package.adoc §Requirements: "The most basic
-- characteristic of all values typically called 'quantities' is that they are
-- ordered, meaning that the operator '<' is defined between any two values in
-- the domain"; the per-subtype key follows the same chapter — the magnitude of
-- a DV_QUANTIFIED, the value of an ordinal or scale, and for DV_PROPORTION "a
-- magnitude function which is computed as the result of the numerator/
-- denominator division". A zero denominator has no magnitude and reads as NULL
-- rather than as an extremum, so it orders as absent. The date/time subtypes
-- take the readings above. A subtype with no ordering, and a value whose
-- carrier is not a number, read as NULL.
--
-- The simple CASE form evaluates its operand once, which matters because the
-- argument at an AQL call site is a jsonb path extraction and this body folds
-- into the statement.
CREATE FUNCTION openehr_magnitude(dv jsonb) RETURNS numeric
LANGUAGE sql IMMUTABLE PARALLEL SAFE
RETURN CASE dv->>'_type'
    WHEN 'DV_QUANTITY'   THEN openehr_numeric(dv->>'magnitude')
    WHEN 'DV_COUNT'      THEN openehr_numeric(dv->>'magnitude')
    WHEN 'DV_ORDINAL'    THEN openehr_numeric(dv->>'value')
    WHEN 'DV_SCALE'      THEN openehr_numeric(dv->>'value')
    WHEN 'DV_PROPORTION' THEN openehr_numeric(dv->>'numerator')
                              / nullif(openehr_numeric(dv->>'denominator'), 0)
    WHEN 'DV_DATE'       THEN openehr_date_days(dv->>'value')
    WHEN 'DV_TIME'       THEN openehr_time_seconds(dv->>'value')
    WHEN 'DV_DATE_TIME'  THEN openehr_date_time_seconds(dv->>'value')
    WHEN 'DV_DURATION'   THEN openehr_duration_seconds(dv->>'value')
END;

-- A total ISO 8601 text -> timestamptz reading.
--
-- This is the ONE partial-precision semantics the product carries on the
-- temporal paths: the promoted timestamp columns (node.context_start) are
-- populated through it at write, and the AQL engine's temporal comparisons and
-- orderings extract through it (app/ferroehr/src/aql/sql/value.rs,
-- Coercion::Temporal), so the promoted fast path and the jsonb lowering cannot
-- disagree.
--
-- Two readings. The first is the full-precision extended form, which
-- PostgreSQL's own date/time input reads correctly and far more cheaply than a
-- decomposition can; what makes the cast unable to raise is the guard before
-- it, which pins the separator positions with a LIKE pattern, proves every
-- other character of the first nineteen is a digit with one translate(), and
-- then bounds each field as text. The second completes reduced precision, which
-- the cast cannot read at all: a partial date assumes the first month and day,
-- a partial time assumes zero, and a time-only value anchors on 0001-01-01 —
-- the same floor openehr_date_days and openehr_time_seconds document.
-- Mixed-precision ordering is genuinely unspecified upstream, and the floor is
-- our own recorded semantics that does not depend on its resolution.
--
-- The offset-less reading takes the session TimeZone, spelled as AT TIME ZONE
-- over current_setting('TimeZone') rather than a cast, so the completion is one
-- expression with the local timestamp read once.
--
-- Malformed input returns NULL — a comparison miss, never an error. Input that
-- is not ISO 8601 at all, including the words PostgreSQL's own parser accepts
-- (`now`, `today`, `infinity`), is malformed here: a stored value must not read
-- as the current time.
CREATE FUNCTION openehr_timestamp(v text) RETURNS timestamptz
LANGUAGE plpgsql STABLE STRICT PARALLEL SAFE AS $$
DECLARE
    s text := replace(v, ',', '.');
BEGIN
    IF s LIKE '____-__-_____:__:__%'
       AND translate(substr(s, 1, 19), '0123456789', '') IN ('--T::', '--t::', '-- ::')
       AND substr(s, 20) IN ('', 'Z', 'z')
    THEN
        IF substr(s, 1, 4) <> '0000'
           AND substr(s, 6, 2) BETWEEN '01' AND '12'
           AND substr(s, 12, 2) <= '23'
           AND substr(s, 15, 2) <= '59'
           AND substr(s, 18, 2) <= '59'
           AND substr(s, 9, 2) >= '01'
           AND substr(s, 9, 2) <= (CASE substr(s, 6, 2)
                   WHEN '02' THEN CASE WHEN (substr(s, 1, 4)::integer % 4 = 0
                                             AND substr(s, 1, 4)::integer % 100 <> 0)
                                            OR substr(s, 1, 4)::integer % 400 = 0
                                       THEN '29' ELSE '28' END
                   WHEN '04' THEN '30' WHEN '06' THEN '30'
                   WHEN '09' THEN '30' WHEN '11' THEN '30' ELSE '31' END)
        THEN
            RETURN s::timestamptz;
        END IF;
    END IF;

    DECLARE
        date_part text;
        time_part text;
        sep       integer;
        days      numeric;
        sod       numeric;
        local_ts  timestamp;
    BEGIN
        sep := coalesce(nullif(position('T' in s), 0),
                        nullif(position('t' in s), 0),
                        nullif(position(' ' in s), 0), 0);
        IF sep = 1 OR s ~ '^[0-9]{1,2}:' THEN
            time_part := CASE WHEN sep = 1 THEN substr(s, 2) ELSE s END;
            days := 0;
        ELSE
            date_part := CASE WHEN sep = 0 THEN s ELSE substr(s, 1, sep - 1) END;
            time_part := CASE WHEN sep = 0 THEN '' ELSE substr(s, sep + 1) END;
            IF date_part !~ '^[0-9]{4}(-[0-9]{2}(-[0-9]{2})?|[0-9]{2}([0-9]{2})?)?$'
            THEN RETURN NULL; END IF;
            days := ext.openehr_date_days(date_part);
            IF days IS NULL THEN RETURN NULL; END IF;
        END IF;
        sod := ext.openehr_time_seconds(time_part);
        IF sod IS NULL THEN RETURN NULL; END IF;
        local_ts := (DATE '0001-01-01' + days::integer)::timestamp
                    + make_interval(secs => (sod - ext.openehr_tz_offset_seconds(time_part))::double precision);
        IF time_part ~ '([Zz]|[+-][0-9]{2}:?[0-9]{0,2})$' THEN
            RETURN local_ts AT TIME ZONE 'UTC';
        END IF;
        RETURN local_ts AT TIME ZONE current_setting('TimeZone');
    END;
END $$;

-- ── Function documentation ─────────────────────────────────────
COMMENT ON FUNCTION ext.openehr_numeric(text) IS
    'The numeric value of a canonical JSON number''s text (RFC 8259 §6), NULL for anything else. IMMUTABLE — index-legal.';
COMMENT ON FUNCTION ext.openehr_date_days(text) IS
    'Days since 0001-01-01 for an ISO 8601 date in either format; NULL when the text is not one, or names no real calendar date. Reduced precision assumes the first month/day. IMMUTABLE — index-legal.';
COMMENT ON FUNCTION ext.openehr_time_seconds(text) IS
    'Seconds since the start of the day for an ISO 8601 time, ignoring any zone suffix (callers apply the offset); NULL on unreadable input. Reduced precision assumes 0. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_tz_offset_seconds(text) IS
    'The zone offset in seconds carried by an ISO 8601 time suffix (Z / ±hh[[:]mm]); 0 when the value carries none. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_date_time_seconds(text) IS
    'Seconds since 0001-01-01T00:00:00Z for an ISO 8601 date-time; NULL when the date is unreadable. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_duration_seconds(text) IS
    'Seconds for an ISO 8601 duration using openEHR''s definite-arithmetic averages (year = 365.24 d, month = 30.42 d); NULL on unreadable input. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_magnitude(jsonb) IS
    'The ordered magnitude (numeric) of a canonical DV_ORDERED value, per the per-subtype comparison the RM defines (RM data_types master06-quantity_package.adoc). NULL for non-ordered or unreadable values. IMMUTABLE + PARALLEL SAFE, so it is legal in a btree expression index.';
COMMENT ON FUNCTION ext.openehr_timestamp(text) IS
    'Total ISO-8601 text -> timestamptz: PostgreSQL''s own parser for the full-precision extended form, floor completion for reduced precision (partial dates assume the first month/day, partial times 0, time-only values anchor on 0001-01-01), NULL for anything else — never an error. The ONE partial-temporal semantics: feeds the promoted timestamp columns (node.context_start) AND the AQL temporal coercion. STABLE (TimeZone-dependent for offset-less inputs); not legal in index expressions.';

-- The runtime writers execute these on the write path (promoted-column
-- population) and the readers on the AQL path; ext/0001 already installed
-- ALTER DEFAULT PRIVILEGES for functions the migrator creates in this schema,
-- so no further grant is issued here.
