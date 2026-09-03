-- SPDX-FileCopyrightText: Ruben Talstra
-- SPDX-License-Identifier: BUSL-1.1

-- ehr schema: the event-filter subscription store (an extension — no openEHR
-- spec governs eventing; "Event
-- Trigger" parity).
--
-- Append-only on the baseline (0001) + the eventing outbox (0002). A
-- subscription is a server-side filter over the PHI-free event stream: its
-- predicates select which committed-version events a consumer wants.
-- maps each subscription to an AMQP topic binding key on the `ferroehr.events`
-- exchange (key <kind>.<change_type>.<template_id|->), so the broker does the
-- fan-out: the publisher declares a durable per-subscription queue
-- `ferroehr.events.<name>` bound with that key. An AQL-shaped condition language
-- is explicitly deferred. Subscription CRUD is a config-gated admin
-- extension surface (ferroehr-rest). Follows the baseline discipline: named
-- constraints (pk_/uq_), COMMENT ON everything, role-guarded grants. Runs with
-- search_path = ehr, ext.

-- ── event_subscription ──────────────────────────────────────────
CREATE TABLE event_subscription (
    -- Stable subscription identity (uuidv7, PG18): time-ordered, index-friendly,
    -- and the addressable id of the admin CRUD surface.
    id           uuid NOT NULL DEFAULT uuidv7(),
    -- The human-chosen subscription name — also the suffix of its broker queue
    -- (`ferroehr.events.<name>`). UNIQUE so one name = one queue.
    name         text NOT NULL,
    -- ── Predicates ──────────────────────────────────────────────
    -- Each predicate is a versioned-object facet the subscription matches on;
    -- NULL means "wildcard — match any value for this facet". The three broker-
    -- routable facets (kind/change_type/template_id) form the topic binding key,
    -- with the `*` single-word topic wildcard substituted for a NULL predicate
    -- (never `-`, which is only the *routing* key's empty-template rendering).
    --
    -- kind: the versioned-object RM type (COMPOSITION / EHR_STATUS / FOLDER /
    -- EHR_ACCESS). NULL = any kind.
    kind         text,
    -- change_type: the audit change-type group code (249 creation, 251
    -- modification, 523 deleted, 666 attestation, …). NULL = any change type.
    change_type  text,
    -- template_id: the OPT template a COMPOSITION was committed against (NULL for
    -- EHR_STATUS/FOLDER/deletes). A predicate NULL = any template.
    template_id  text,
    -- Whether the subscription is active: the publisher declares/binds a queue
    -- only for enabled subscriptions. Disabled = retained but not bound.
    enabled      boolean NOT NULL DEFAULT true,
    -- Creation instant (audit/ordering).
    created_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT pk_event_subscription PRIMARY KEY (id),
    CONSTRAINT uq_event_subscription_name UNIQUE (name)
);

-- The publisher's startup/refresh scan: the enabled subscriptions to declare +
-- bind queues for.
CREATE INDEX idx_event_subscription_enabled ON event_subscription (enabled)
    WHERE enabled;

COMMENT ON TABLE event_subscription IS 'Event-filter subscriptions (an extension — no openEHR spec governs eventing; "Event Trigger" parity): server-side predicate filters over the PHI-free event stream. Each enabled row maps to an AMQP topic binding key <kind>.<change_type>.<template_id|-> (NULL predicate → the `*` topic wildcard) and a durable queue ferroehr.events.<name>; the broker does the fan-out.';
COMMENT ON COLUMN event_subscription.name IS 'Subscription name; also the suffix of its broker queue (ferroehr.events.<name>). UNIQUE.';
COMMENT ON COLUMN event_subscription.kind IS 'Predicate: versioned-object RM type (COMPOSITION/EHR_STATUS/FOLDER/EHR_ACCESS). NULL = wildcard (any kind).';
COMMENT ON COLUMN event_subscription.change_type IS 'Predicate: audit change-type group code (249/251/523/666/…). NULL = wildcard (any change type).';
COMMENT ON COLUMN event_subscription.template_id IS 'Predicate: OPT template id a COMPOSITION was committed against. NULL = wildcard (any template).';
COMMENT ON COLUMN event_subscription.enabled IS 'Whether the subscription is active (its queue is declared/bound). Disabled rows are retained but unbound.';

-- ── Grants ──────────────────────────────────────────────────────
-- The baseline set ALTER DEFAULT PRIVILEGES for ferroehr_app/ferroehr_reader, so a
-- table the migrator creates afterwards is auto-granted; repeated explicitly
-- (role-guarded, like the baseline + 0002) so this migration is self-contained
-- and a no-op on the normal run order.
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'ferroehr_app') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON event_subscription TO ferroehr_app;
        GRANT SELECT ON event_subscription TO ferroehr_reader;
    ELSE
        RAISE NOTICE 'skipping event_subscription grants (roles absent — see the baseline role block NOTICE)';
    END IF;
END $$;
