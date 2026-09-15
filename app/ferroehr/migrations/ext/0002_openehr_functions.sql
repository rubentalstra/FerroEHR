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
-- Every body is ONE expression in `LANGUAGE sql`, validated by a pattern and
-- yielding NULL for input the pattern refuses. None carries an EXCEPTION block:
-- "A block containing an EXCEPTION clause is significantly more expensive to
-- enter and exit than a block without one. Therefore, don't use EXCEPTION
-- without need." (PostgreSQL 18, "Control Structures",
-- https://www.postgresql.org/docs/18/plpgsql-control-structures.html). A helper
-- is read once per candidate row per AQL predicate, so "without need" is the
-- whole of this file: a value that is not of the shape the helper reads is a
-- comparison miss (NULL), never an error the caller sees as a 500.
--
-- Consequence of having no error trap: every cast is reachable only through a
-- guard that proves it cannot raise, and the guards are nested CASE expressions
-- rather than AND chains, because SQL fixes no evaluation order between the
-- operands of AND while CASE evaluates its arms in order (PostgreSQL 18,
-- "Expression Evaluation Rules",
-- https://www.postgresql.org/docs/18/sql-expressions.html).
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
-- The functions are NOT declared STRICT, unlike the first generation's: a
-- strict function whose body holds a CASE is never folded into the calling
-- query, and folding is what removes the per-row call. Each body therefore
-- returns NULL for NULL input by construction, which the ext-function test
-- pins value by value.
--
-- Runs with search_path = ext, so the helpers below resolve each other
-- unqualified and the RETURN bodies bind them at definition time.

-- The numeric value of a canonical JSON number's text.
--
-- substring() with a pattern yields NULL when the pattern does not match, so
-- the guard and the extraction are the same operation and the parameter is read
-- once. The accepted shape is JSON's own number grammar (RFC 8259 §6,
-- https://www.rfc-editor.org/rfc/rfc8259#section-6) plus the leading minus;
-- anything else — a string, an object, an absent key — reads as NULL.
CREATE FUNCTION openehr_numeric(t text) RETURNS numeric
LANGUAGE sql IMMUTABLE PARALLEL SAFE
RETURN substring(t FROM '^-?[0-9]+(?:\.[0-9]+)?(?:[eE][-+]?[0-9]+)?$')::numeric;

-- Days since 0001-01-01 for an ISO 8601 BASIC-format date (`YYYYMMDD`), its
-- reduced-precision prefixes, and trailing text the reading ignores.
--
-- BASE foundation_types master06-time_types.adoc admits reduced precision down
-- to the year, so `YYYY` and `YYYYMM` read as the first month and first day.
-- The first pattern is the length contract the positional reads below need:
-- from eight characters the first eight are digits, from six the first six,
-- from four the first four — matching what the reading actually consumes and
-- nothing more.
--
-- make_date() raises on a triple that is not a real date, so the calendar is
-- checked first: a month in 1..12 and a day within that month's length, with
-- February resolved by the Gregorian leap rule, which PostgreSQL applies to
-- every year because it "uses the Gregorian calendar for all dates and times"
-- (PostgreSQL 18, "Date/Time Support",
-- https://www.postgresql.org/docs/18/datetime-appendix.html). Year 0000 is
-- refused because make_date() has no year zero.
CREATE FUNCTION openehr_basic_date_days(s text) RETURNS numeric
LANGUAGE sql IMMUTABLE PARALLEL SAFE
RETURN CASE
    WHEN s ~ '^([0-9]{8}.*|[0-9]{6}.?|[0-9]{4}.?)$' THEN
        CASE WHEN left(s, 4) <> '0000'
                  AND (length(s) < 6 OR substring(s FROM 5 FOR 2)::integer BETWEEN 1 AND 12)
                  AND (length(s) < 8 OR substring(s FROM 7 FOR 2)::integer BETWEEN 1 AND
                       CASE substring(s FROM 5 FOR 2)
                            WHEN '02' THEN CASE WHEN (left(s, 4)::integer % 4 = 0
                                                      AND left(s, 4)::integer % 100 <> 0)
                                                     OR left(s, 4)::integer % 400 = 0
                                                THEN 29 ELSE 28 END
                            WHEN '04' THEN 30 WHEN '06' THEN 30
                            WHEN '09' THEN 30 WHEN '11' THEN 30
                            ELSE 31 END)
        THEN make_date(left(s, 4)::integer,
                       CASE WHEN length(s) >= 6 THEN substring(s FROM 5 FOR 2)::integer ELSE 1 END,
                       CASE WHEN length(s) >= 8 THEN substring(s FROM 7 FOR 2)::integer ELSE 1 END)
             - DATE '0001-01-01'
        END
END;

-- Days since 0001-01-01 for an ISO 8601 date in either format.
--
-- Removing the hyphens folds the extended format (`YYYY-MM-DD`, the form BASE
-- calls preferred) onto the basic one, so one reading serves both.
CREATE FUNCTION openehr_date_days(v text) RETURNS numeric
LANGUAGE sql IMMUTABLE PARALLEL SAFE
RETURN openehr_basic_date_days(replace(v, '-', ''));

-- Seconds since the start of the day for a clock reading with no zone suffix
-- and no leading `T`.
--
-- Two forms, split on whether a colon is present: the basic `hhmmss` (which
-- needs a length of more than two, or `hh` would read as the first field of the
-- extended form) and the extended `hh:mm:ss`. Each pattern admits exactly the
-- text the reading below casts, including the fractional second BASE
-- foundation_types master06-time_types.adoc allows on the last field. Fields
-- the input omits read as zero, which is the reduced precision the same section
-- admits.
CREATE FUNCTION openehr_hms_seconds(t text) RETURNS numeric
LANGUAGE sql IMMUTABLE PARALLEL SAFE
RETURN CASE
    WHEN position(':' in t) = 0 AND length(t) > 2 THEN
        CASE WHEN t ~ '^([0-9]{4}([0-9]+(\.[0-9]+)?|\.[0-9]+)|[0-9]{4}.?|[0-9]{2}.)$'
        THEN left(t, 2)::numeric * 3600
           + CASE WHEN length(t) >= 4 THEN substring(t FROM 3 FOR 2)::numeric ELSE 0 END * 60
           + CASE WHEN length(t) >= 6 THEN substring(t FROM 5)::numeric ELSE 0 END
        END
    ELSE
        CASE WHEN t ~ '^([0-9]+(\.[0-9]+)?|\.[0-9]+)?(:([0-9]+(\.[0-9]+)?|\.[0-9]+)(:([0-9]+(\.[0-9]+)?|\.[0-9]+)(:.*)?)?)?$'
        THEN coalesce(nullif(split_part(t, ':', 1), '')::numeric, 0) * 3600
           + coalesce(nullif(split_part(t, ':', 2), '')::numeric, 0) * 60
           + coalesce(nullif(split_part(t, ':', 3), '')::numeric, 0)
        END
END;

-- Seconds since the start of the day for an ISO 8601 time, ignoring any zone
-- suffix — the caller applies the offset through openehr_tz_offset_seconds.
--
-- The `T` designator and the zone suffix are cut positionally rather than by
-- substitution; the suffix's length is what the pattern yields, and no suffix
-- yields the empty string, so the reading is the whole text.
CREATE FUNCTION openehr_time_seconds(v text) RETURNS numeric
LANGUAGE sql IMMUTABLE PARALLEL SAFE
RETURN openehr_hms_seconds(
    substring(CASE WHEN left(v, 1) IN ('T', 't') THEN substring(v FROM 2) ELSE v END
              FROM 1 FOR
              length(CASE WHEN left(v, 1) IN ('T', 't') THEN substring(v FROM 2) ELSE v END)
              - length(coalesce(substring(CASE WHEN left(v, 1) IN ('T', 't') THEN substring(v FROM 2) ELSE v END
                                          FROM '([Zz]|[+-][0-9]{2}:?[0-9]{0,2})$'), ''))));

-- The zone offset in seconds carried by an ISO 8601 time's suffix; zero when
-- the value carries none.
--
-- `Z` is zero by definition (BASE foundation_types master06-time_types.adoc:
-- "'Z' and 'T' are literals"); `±hh`, `±hhmm` and `±hh:mm` are read from the
-- end of the value. NULL input is answered first, because the trailing
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

-- Seconds for an ISO 8601 duration, using openEHR's NOMINAL year and month
-- lengths.
--
-- BASE foundation_types master06-time_types.adoc §Computational Functions
-- distinguishes definite arithmetic, which "treats all values as definite,
-- i.e. exact and invariant, based on constant values for length of year and
-- month, defined by Time_definitions.Average_days_in_month and
-- Time_definitions.Average_days_in_year", from nominal calendar arithmetic;
-- an ordering key can only be the definite one, so the averages below are those
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
-- argument at an AQL call site is a jsonb path extraction.
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
-- Three arms, in order.
--
-- The first is the full-precision extended form, which PostgreSQL's own
-- date/time input reads correctly and far more cheaply than a decomposition
-- can. Its pattern is what makes the cast unable to raise: a calendar-valid
-- month and day (February 29 falls through to the completion arm, which
-- resolves the leap year), an hour below 24, a minute and second below 60, and
-- a zone offset within the range PostgreSQL accepts. ISO 8601 permits a COMMA
-- decimal sign on the fractional second (BASE foundation_types master06
-- §Class Definitions, `[(,|.)sss]`), which PostgreSQL rejects, so the comma
-- normalizes to the dot before the cast.
--
-- The second and third complete reduced precision instead, which the cast
-- cannot read at all: a partial date assumes the first month and day, a partial
-- time assumes zero, and a time-only value anchors on 0001-01-01 — the same
-- floor openehr_date_days and openehr_time_seconds document. Mixed-precision
-- ordering is genuinely unspecified upstream, and the floor is our own recorded
-- semantics that does not depend on its resolution.
--
-- The offset-less reading takes the session TimeZone, spelled as AT TIME ZONE
-- over current_setting('TimeZone') rather than a cast, so the whole completion
-- is one expression with the local timestamp read once.
--
-- Malformed input returns NULL — a comparison miss, never an error. Input that
-- is not ISO 8601 at all, including the words PostgreSQL's own parser accepts
-- (`now`, `today`, `infinity`), is malformed here: a stored value must not read
-- as the current time.
CREATE FUNCTION openehr_timestamp(v text) RETURNS timestamptz
LANGUAGE sql STABLE PARALLEL SAFE
RETURN CASE
    WHEN left(v, 4) <> '0000' AND v ~
         '^[0-9]{4}-((0[13578]|1[02])-(0[1-9]|[12][0-9]|3[01])|(0[469]|11)-(0[1-9]|[12][0-9]|30)|02-(0[1-9]|1[0-9]|2[0-8]))[Tt ]([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]([.,][0-9]+)?([Zz]|[+-](0[0-9]|1[0-5])(:?[0-5][0-9])?)?$'
    THEN replace(v, ',', '.')::timestamptz
    WHEN left(translate(replace(v, ',', '.'), 't ', 'TT'), 1) = 'T'
         OR translate(replace(v, ',', '.'), 't ', 'TT') ~ '^[0-9]{1,2}:' THEN
        (TIMESTAMP '0001-01-01 00:00:00'
         + make_interval(secs => (openehr_time_seconds(replace(v, ',', '.'))
                                  - openehr_tz_offset_seconds(replace(v, ',', '.')))::double precision))
        AT TIME ZONE (CASE WHEN replace(v, ',', '.') ~ '([Zz]|[+-][0-9]{2}:?[0-9]{0,2})$'
                           THEN 'UTC' ELSE current_setting('TimeZone') END)
    WHEN split_part(translate(replace(v, ',', '.'), 't ', 'TT'), 'T', 1)
         ~ '^[0-9]{4}(-[0-9]{2}(-[0-9]{2})?|[0-9]{2}([0-9]{2})?)?$' THEN
        ((DATE '0001-01-01'
          + openehr_date_days(split_part(translate(replace(v, ',', '.'), 't ', 'TT'), 'T', 1))::integer)::timestamp
         + make_interval(secs => (openehr_time_seconds(split_part(translate(replace(v, ',', '.'), 't ', 'TT'), 'T', 2))
                                  - openehr_tz_offset_seconds(split_part(translate(replace(v, ',', '.'), 't ', 'TT'), 'T', 2)))::double precision))
        AT TIME ZONE (CASE WHEN split_part(translate(replace(v, ',', '.'), 't ', 'TT'), 'T', 2)
                                ~ '([Zz]|[+-][0-9]{2}:?[0-9]{0,2})$'
                           THEN 'UTC' ELSE current_setting('TimeZone') END)
END;

-- ── Function documentation ─────────────────────────────────────
COMMENT ON FUNCTION ext.openehr_numeric(text) IS
    'The numeric value of a canonical JSON number''s text (RFC 8259 §6), NULL for anything else. IMMUTABLE — index-legal.';
COMMENT ON FUNCTION ext.openehr_basic_date_days(text) IS
    'Days since 0001-01-01 for an ISO 8601 basic-format date (YYYYMMDD) and its reduced-precision prefixes; NULL when the text is not one, or names no real calendar date. Reduced precision assumes the first month/day. IMMUTABLE.';
COMMENT ON FUNCTION ext.openehr_date_days(text) IS
    'Days since 0001-01-01 for an ISO 8601 date in either format; NULL on unreadable input. Reduced precision assumes the first month/day. IMMUTABLE — index-legal.';
COMMENT ON FUNCTION ext.openehr_hms_seconds(text) IS
    'Seconds since the start of the day for a clock reading with no T designator and no zone suffix; NULL on unreadable input. Omitted fields read as 0. IMMUTABLE.';
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
