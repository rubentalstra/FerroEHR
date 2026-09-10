-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1
--
-- Protected national identifiers: the value leaves the versioned body and
-- lives here, sealed, with a keyed digest beside it for equality lookup.
--
-- The RM models identifiers generically (PARTY_IDENTITY, DV_IDENTIFIER) and
-- says nothing about protecting the value, so no openEHR spec governs this —
-- our own design. The obligations are external: GDPR Art. 32(1)(a) names
-- encryption as an appropriate measure
-- (https://eur-lex.europa.eu/eli/reg/2016/679/oj), NEN 7510-2 carries the
-- cryptographic controls, and UAVG Art. 46 with the Wabvpz permit BSN
-- processing in care only for identification and under the act's conditions
-- (https://wetten.overheid.nl/BWBR0040940, https://wetten.overheid.nl/BWBR0023864).
--
-- Two columns rather than one, because storage and lookup want opposite
-- properties. `ciphertext` is AES-256-GCM with a fresh nonce per record, so
-- two parties with the same identifier store different bytes and neither
-- reveals the value. `lookup_digest` is HMAC-SHA-256 under a separate
-- per-tenant subkey, so equality search works without decrypting — and,
-- because it is KEYED, the database, a backup or a read replica cannot
-- reverse it by enumerating a nine-digit space.

-- ── the scheme registry ──────────────────────────────────────────────────────
-- A closed set rather than free text: an identifier whose scheme nobody
-- declared is an identifier nobody agreed to hold, and the registry is what a
-- reviewer reads to see which kinds this deployment stores at all.

CREATE TABLE demographic.identifier_scheme (
    -- The scheme code, `<iso-3166-1-alpha-2 lowercased>-<local name>`, the same
    -- shape the write-path scanner keys its rules by.
    code        text        NOT NULL,
    -- The issuing jurisdiction, ISO 3166-1 alpha-2.
    jurisdiction text       NOT NULL,
    -- The identifier's own name in its register.
    label       text        NOT NULL,
    -- The register or authority that defines it.
    source      text        NOT NULL,
    CONSTRAINT pk_identifier_scheme PRIMARY KEY (code),
    CONSTRAINT ck_identifier_scheme_code
        CHECK (code ~ '^[a-z]{2}-[a-z0-9-]+$'),
    CONSTRAINT ck_identifier_scheme_jurisdiction
        CHECK (jurisdiction ~ '^[A-Z]{2}$')
);

COMMENT ON TABLE demographic.identifier_scheme IS
    'The national personal-identifier kinds this deployment may store, by code. A closed set: an identifier whose scheme is not registered here cannot be written.';

INSERT INTO demographic.identifier_scheme (code, jurisdiction, label, source) VALUES
    ('nl-bsn', 'NL', 'burgerservicenummer (BSN)',
     'Rijksdienst voor Identiteitsgegevens, Logisch Ontwerp BSN, https://www.rvig.nl/logisch-ontwerp-bsn');

-- ── the protected values ─────────────────────────────────────────────────────

CREATE TABLE demographic.national_identifier (
    id            uuid        NOT NULL DEFAULT uuidv7(),
    -- The party this identifier belongs to (the demographic VERSIONED_OBJECT
    -- id). No foreign key: a party's versions come and go under change
    -- control, and an identifier outliving a particular version is correct.
    party_id      uuid        NOT NULL,
    -- The registered scheme.
    scheme        text        NOT NULL,
    -- The owning tenant. Part of the key derivation and of the AEAD associated
    -- data, so a record cannot be read under another tenant's key.
    tenant_id     uuid        NOT NULL,
    -- AES-256-GCM: the 96-bit nonce and the sealed value.
    nonce         bytea       NOT NULL,
    ciphertext    bytea       NOT NULL,
    -- HMAC-SHA-256 of the value under this tenant's lookup subkey.
    lookup_digest bytea       NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_national_identifier PRIMARY KEY (id),
    CONSTRAINT fk_national_identifier_scheme
        FOREIGN KEY (scheme) REFERENCES demographic.identifier_scheme (code),
    CONSTRAINT ck_national_identifier_nonce CHECK (octet_length(nonce) = 12),
    CONSTRAINT ck_national_identifier_digest CHECK (octet_length(lookup_digest) = 32)
);

-- One party holds one value per scheme per tenant, and one value identifies one
-- party: both directions are the point of a national identifier, and a
-- duplicate in either direction is a data defect the store refuses rather than
-- a resolution that silently picks a row.
CREATE UNIQUE INDEX uq_national_identifier_party
    ON demographic.national_identifier (tenant_id, scheme, party_id);
CREATE UNIQUE INDEX uq_national_identifier_value
    ON demographic.national_identifier (tenant_id, scheme, lookup_digest);

COMMENT ON TABLE demographic.national_identifier IS
    'National personal identifiers, sealed under a per-tenant AES-256-GCM key with an HMAC-SHA-256 lookup digest beside them. The versioned body carries a reference to a row here, never the value.';
COMMENT ON COLUMN demographic.national_identifier.ciphertext IS
    'AES-256-GCM sealed value; the scheme and tenant are the associated data, so a row moved between either fails to open.';
COMMENT ON COLUMN demographic.national_identifier.lookup_digest IS
    'HMAC-SHA-256 of the value under the tenant lookup subkey: deterministic for equality search, keyed so the small value space cannot be enumerated.';

-- ── the resolution path ──────────────────────────────────────────────────────
-- The only way to get from an identifier to a party without holding the
-- ciphertext. It takes the DIGEST rather than the value, so the caller proves
-- it already knows the identifier and the plaintext never crosses this
-- boundary; it returns the party and nothing else.

CREATE FUNCTION demographic.resolve_national_identifier(
    p_tenant  uuid,
    p_scheme  text,
    p_digest  bytea
) RETURNS uuid
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path = demographic, pg_catalog
    AS $$
    SELECT party_id
    FROM demographic.national_identifier
    WHERE tenant_id = p_tenant
      AND scheme = p_scheme
      AND lookup_digest = p_digest;
$$;

COMMENT ON FUNCTION demographic.resolve_national_identifier(uuid, text, bytea) IS
    'The sole identifier-to-party path for a role without ciphertext access. Takes the keyed lookup digest, never the value, and returns the party id alone. Every call is recorded as a linkage-domain access event by the caller.';

-- ── grants ───────────────────────────────────────────────────────────────────
-- Only the demographic writer reaches the ciphertext. The clinical roles are
-- revoked explicitly rather than merely never granted, for the reason the
-- baseline's grant block states: PUBLIC holds EXECUTE on functions by default
-- and an earlier blanket grant may exist
-- (https://www.postgresql.org/docs/18/sql-grant.html).
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_demographic') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON demographic.national_identifier
            TO ferroehr_demographic;
        GRANT SELECT ON demographic.identifier_scheme
            TO ferroehr_demographic, ferroehr_demographic_reader;
        -- The read-only twin sees that a protected identifier EXISTS and which
        -- party holds it, which its reporting role needs, but never the sealed
        -- value: the two sensitive columns are withheld by column-level grant
        -- rather than by trusting every future query.
        --
        -- The REVOKE first is load-bearing, not defensive tidiness. The
        -- baseline's `ALTER DEFAULT PRIVILEGES IN SCHEMA demographic GRANT
        -- SELECT ON TABLES TO ferroehr_demographic_reader` applies to every
        -- table created here afterwards, so this one arrived with a
        -- table-level SELECT already on it — which covers every column and
        -- would silently outrank the column list below (PostgreSQL 18, GRANT:
        -- table-level privileges apply to all columns,
        -- https://www.postgresql.org/docs/18/sql-grant.html).
        REVOKE SELECT ON demographic.national_identifier
            FROM ferroehr_demographic_reader;
        GRANT SELECT (id, party_id, scheme, tenant_id, created_at)
            ON demographic.national_identifier TO ferroehr_demographic_reader;

        REVOKE ALL ON demographic.national_identifier, demographic.identifier_scheme
            FROM ferroehr_ehr, ferroehr_ehr_reader;

        -- The resolve function is SECURITY DEFINER, so EXECUTE on it is the
        -- whole permission: PUBLIC is revoked and each role named explicitly.
        REVOKE ALL ON FUNCTION
            demographic.resolve_national_identifier(uuid, text, bytea) FROM PUBLIC;
        GRANT EXECUTE ON FUNCTION
            demographic.resolve_national_identifier(uuid, text, bytea)
            TO ferroehr_demographic;
    ELSE
        RAISE NOTICE 'skipping national-identifier grants (roles absent — see the baseline role block)';
    END IF;
END $$;
