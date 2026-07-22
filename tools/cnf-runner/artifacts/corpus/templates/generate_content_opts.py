#!/usr/bin/env python3
"""Deterministic generator for the CNF content-chapter corpus OPTs.

Regenerates every `cnf.tpl.*` operational template under this directory from a
single carrier skeleton plus a constraint table. Each generated OPT is a real,
self-contained OPT 1.4 XML that a conformant server accepts (201), carrying the
value/structural constraint the matching content case declares.

Carrier skeleton: the openEHR CNF Robot minimal_observation.opt
(docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/
valid_templates/minimal/minimal_observation.opt) — a COMPOSITION
(openEHR-EHR-COMPOSITION.minimal.v1) whose single content OBSERVATION
(openEHR-EHR-OBSERVATION.minimal.v1) carries the
data/events/data/items/value ELEMENT (at0004) the content cases constrain.

Constraint XML shapes are grounded in:
  * AM AOM 1.4 (docs/specs/openehr/AM/) — C_PRIMITIVE_OBJECT / C_INTEGER /
    C_REAL / C_STRING / C_BOOLEAN / C_DATE / C_TIME / C_DATE_TIME / C_DURATION,
    C_CODE_PHRASE, C_DV_ORDINAL, C_DV_QUANTITY (+ C_QUANTITY_ITEM),
    CONSTRAINT_REF, C_MULTIPLE_ATTRIBUTE.cardinality, C_ATTRIBUTE.existence,
    C_OBJECT.rm_type_name.
  * RM data_types (docs/specs/openehr/RM/) — DV_* value shapes.
  * The ITS-XML XSD family (crates/openehr-its/schemas/xml/) and the vendored
    real OPTs (validation/proportion.opt) for exact element ordering.

Design notes (no openEHR spec governs these — our own corpus-authoring design):
  * Each template's `template_id` VALUE == its manifest key (cnf.tpl.<name>).
  * DV_SCALE has no dedicated AOM 1.4 domain constraint (C_DV_SCALE does not
    exist in AOM 1.4 — DV_SCALE is an RM >= 1.1.0 type postdating ADL 1.4);
    it is expressed as a plain C_COMPLEX_OBJECT over DV_SCALE with a C_REAL
    `value` constraint. Flagged in the summary.
  * Where several content cases share one template key with differing
    per-row constraints (the runner's known constraint-axis-in-rows model),
    the template bakes the single representative constraint its name/manifest
    provenance declares.

Run:  python3 generate_content_opts.py
It rewrites every cnf.tpl.* OPT in this directory in place, deterministically.
"""

import hashlib
import os
import uuid

HERE = os.path.dirname(os.path.abspath(__file__))

# Deterministic UUID namespace (stable across runs; our own, spec-silent).
_NS = uuid.UUID("6f9619ff-8b86-d011-b42d-00cf4fc964ff")


def det_uid(template_id: str) -> str:
    return str(uuid.uuid5(_NS, template_id))


def xesc(s: str) -> str:
    return (
        s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


# ---------------------------------------------------------------------------
# Interval / occurrence / existence / cardinality primitives (AOM 1.4).
# ---------------------------------------------------------------------------
def interval(lower, upper, lower_included=True, upper_included=True):
    """IntervalOfInteger. None bound => unbounded on that side."""
    lu = "true" if lower is None else "false"
    uu = "true" if upper is None else "false"
    parts = [
        f"<lower_included>{'true' if lower_included else 'false'}</lower_included>",
        f"<upper_included>{'true' if upper_included else 'false'}</upper_included>",
        f"<lower_unbounded>{lu}</lower_unbounded>",
        f"<upper_unbounded>{uu}</upper_unbounded>",
    ]
    if lower is not None:
        parts.append(f"<lower>{lower}</lower>")
    if upper is not None:
        parts.append(f"<upper>{upper}</upper>")
    return "".join(parts)


def occ(lower, upper):
    return f"<occurrences>{interval(lower, upper)}</occurrences>"


def existence(lower, upper):
    return f"<existence>{interval(lower, upper)}</existence>"


def cardinality(lower, upper, ordered=False, unique=False):
    li = "true"
    lu = "true" if lower is None else "false"
    uu = "true" if upper is None else "false"
    body = [
        f"<is_ordered>{'true' if ordered else 'false'}</is_ordered>",
        f"<is_unique>{'true' if unique else 'false'}</is_unique>",
        "<interval>",
        f"<lower_included>{li}</lower_included>",
        f"<lower_unbounded>{lu}</lower_unbounded>",
        f"<upper_unbounded>{uu}</upper_unbounded>",
    ]
    if lower is not None:
        body.append(f"<lower>{lower}</lower>")
    if upper is not None:
        body.append(f"<upper>{upper}</upper>")
    body.append("</interval>")
    return f"<cardinality>{''.join(body)}</cardinality>"


def real_interval(lower, upper, lower_included=True, upper_included=True):
    lu = "true" if lower is None else "false"
    uu = "true" if upper is None else "false"
    parts = [
        f"<lower_included>{'true' if lower_included else 'false'}</lower_included>",
        f"<upper_included>{'true' if upper_included else 'false'}</upper_included>",
        f"<lower_unbounded>{lu}</lower_unbounded>",
        f"<upper_unbounded>{uu}</upper_unbounded>",
    ]
    if lower is not None:
        parts.append(f"<lower>{lower}</lower>")
    if upper is not None:
        parts.append(f"<upper>{upper}</upper>")
    return "".join(parts)


# ---------------------------------------------------------------------------
# C_* constraint builders (return the <children xsi:type=...> object block, or,
# for the primitive item, the <item> block).
# ---------------------------------------------------------------------------
def c_single_attr(name, child_xml, exist=(1, 1)):
    return (
        '<attributes xsi:type="C_SINGLE_ATTRIBUTE">'
        f"<rm_attribute_name>{name}</rm_attribute_name>"
        f"{existence(*exist)}"
        f"{child_xml}"
        "</attributes>"
    )


def c_multiple_attr(name, children_xml, card, exist=(0, None)):
    return (
        '<attributes xsi:type="C_MULTIPLE_ATTRIBUTE">'
        f"<rm_attribute_name>{name}</rm_attribute_name>"
        f"{existence(*exist)}"
        f"{children_xml}"
        f"{card}"
        "</attributes>"
    )


def c_complex(rm_type, attrs_xml="", node_id="", occ_lu=(1, 1)):
    nid = f"<node_id>{node_id}</node_id>" if node_id else "<node_id />"
    return (
        '<children xsi:type="C_COMPLEX_OBJECT">'
        f"<rm_type_name>{xesc(rm_type)}</rm_type_name>"
        f"{occ(*occ_lu)}"
        f"{nid}"
        f"{attrs_xml}"
        "</children>"
    )


def c_primitive_object(prim_rm_type, item_xml):
    return (
        '<children xsi:type="C_PRIMITIVE_OBJECT">'
        f"<rm_type_name>{prim_rm_type}</rm_type_name>"
        f"{occ(1, 1)}"
        "<node_id />"
        f"{item_xml}"
        "</children>"
    )


def item_c_integer_list(vals):
    lists = "".join(f"<list>{v}</list>" for v in vals)
    return f'<item xsi:type="C_INTEGER">{lists}</item>'


def item_c_integer_range(lo, hi):
    return f'<item xsi:type="C_INTEGER"><range>{interval(lo, hi)}</range></item>'


def item_c_real_range(lo, hi):
    return f'<item xsi:type="C_REAL"><range>{real_interval(lo, hi)}</range></item>'


def item_c_real_list(vals):
    lists = "".join(f"<list>{v}</list>" for v in vals)
    return f'<item xsi:type="C_REAL">{lists}</item>'


def item_c_string_pattern(pattern):
    return f'<item xsi:type="C_STRING"><pattern>{xesc(pattern)}</pattern></item>'


def item_c_string_list(vals):
    lists = "".join(f"<list>{xesc(v)}</list>" for v in vals)
    return f'<item xsi:type="C_STRING">{lists}<list_open>false</list_open></item>'


def item_c_boolean(true_valid, false_valid):
    return (
        '<item xsi:type="C_BOOLEAN">'
        f"<true_valid>{'true' if true_valid else 'false'}</true_valid>"
        f"<false_valid>{'true' if false_valid else 'false'}</false_valid>"
        "</item>"
    )


def item_c_date(pattern):
    return f'<item xsi:type="C_DATE"><pattern>{pattern}</pattern></item>'


def item_c_date_range(lo, hi):
    # Interval of Date (date strings as bounds).
    parts = [
        "<lower_included>true</lower_included>",
        "<upper_included>true</upper_included>",
        "<lower_unbounded>false</lower_unbounded>",
        "<upper_unbounded>false</upper_unbounded>",
        f"<lower>{lo}</lower>",
        f"<upper>{hi}</upper>",
    ]
    return f'<item xsi:type="C_DATE"><range>{"".join(parts)}</range></item>'


def item_c_time(pattern):
    return f'<item xsi:type="C_TIME"><pattern>{pattern}</pattern></item>'


def item_c_date_time(pattern):
    return f'<item xsi:type="C_DATE_TIME"><pattern>{pattern}</pattern></item>'


def item_c_duration(pattern):
    return f'<item xsi:type="C_DURATION"><pattern>{pattern}</pattern></item>'


def code_phrase(term, code):
    return (
        f"<terminology_id><value>{xesc(term)}</value></terminology_id>"
        f"<code_string>{xesc(code)}</code_string>"
    )


def c_code_phrase(term, codes, node_id=""):
    nid = f"<node_id>{node_id}</node_id>" if node_id else "<node_id />"
    code_list = "".join(f"<code_list>{xesc(c)}</code_list>" for c in codes)
    return (
        '<children xsi:type="C_CODE_PHRASE">'
        "<rm_type_name>CODE_PHRASE</rm_type_name>"
        f"{occ(1, 1)}"
        f"{nid}"
        f"<terminology_id><value>{xesc(term)}</value></terminology_id>"
        f"{code_list}"
        "</children>"
    )


def constraint_ref(reference, node_id=""):
    nid = f"<node_id>{node_id}</node_id>" if node_id else "<node_id />"
    return (
        '<children xsi:type="CONSTRAINT_REF">'
        "<rm_type_name>CODE_PHRASE</rm_type_name>"
        f"{occ(1, 1)}"
        f"{nid}"
        f"<reference>{xesc(reference)}</reference>"
        "</children>"
    )


def dv_ordinal_item(value, term, code, label):
    return (
        '<list xsi:type="DV_ORDINAL">'
        f"<value>{value}</value>"
        "<symbol>"
        f"<value>{xesc(label)}</value>"
        f"<defining_code>{code_phrase(term, code)}</defining_code>"
        "</symbol>"
        "</list>"
    )


def c_dv_ordinal(items, node_id=""):
    nid = f"<node_id>{node_id}</node_id>" if node_id else "<node_id />"
    body = "".join(dv_ordinal_item(*it) for it in items)
    return (
        '<children xsi:type="C_DV_ORDINAL">'
        "<rm_type_name>DV_ORDINAL</rm_type_name>"
        f"{occ(1, 1)}"
        f"{nid}"
        f"{body}"
        "</children>"
    )


def c_quantity_item(units, mag_lo=None, mag_hi=None):
    mag = ""
    if mag_lo is not None or mag_hi is not None:
        mag = f'<magnitude>{real_interval(mag_lo, mag_hi)}</magnitude>'
    return (
        '<list xsi:type="C_QUANTITY_ITEM">'
        f"{mag}"
        f"<units>{xesc(units)}</units>"
        "</list>"
    )


def c_dv_quantity(property_term, property_code, items, node_id=""):
    nid = f"<node_id>{node_id}</node_id>" if node_id else "<node_id />"
    prop = ""
    if property_term is not None:
        prop = f"<property>{code_phrase(property_term, property_code)}</property>"
    body = "".join(items)
    return (
        '<children xsi:type="C_DV_QUANTITY">'
        "<rm_type_name>DV_QUANTITY</rm_type_name>"
        f"{occ(1, 1)}"
        f"{nid}"
        f"{prop}"
        f"{body}"
        "</children>"
    )


# ---------------------------------------------------------------------------
# High-level value-constraint builders for the ELEMENT.value slot.
# Each returns the <children ...>...</children> for the value C_SINGLE_ATTRIBUTE.
# extra_terms: dict of code -> (text, description) added to the OBSERVATION
# archetype term_definitions.
# ---------------------------------------------------------------------------
def dv_count(item_xml=None):
    if item_xml is None:  # open: DV_COUNT matches {*}
        return c_complex("DV_COUNT")
    return c_complex("DV_COUNT", c_single_attr("magnitude", c_primitive_object("INTEGER", item_xml)))


def dv_boolean(item_xml=None):
    if item_xml is None:
        return c_complex("DV_BOOLEAN")
    return c_complex("DV_BOOLEAN", c_single_attr("value", c_primitive_object("BOOLEAN", item_xml)))


def dv_string_type(rm_type, item_xml=None, field="value"):
    if item_xml is None:
        return c_complex(rm_type)
    return c_complex(rm_type, c_single_attr(field, c_primitive_object("STRING", item_xml)))


def dv_date(item_xml=None):
    if item_xml is None:
        return c_complex("DV_DATE")
    return c_complex("DV_DATE", c_single_attr("value", c_primitive_object("DATE", item_xml)))


def dv_time(item_xml=None):
    if item_xml is None:
        return c_complex("DV_TIME")
    return c_complex("DV_TIME", c_single_attr("value", c_primitive_object("TIME", item_xml)))


def dv_date_time(item_xml=None):
    if item_xml is None:
        return c_complex("DV_DATE_TIME")
    return c_complex("DV_DATE_TIME", c_single_attr("value", c_primitive_object("DATE_TIME", item_xml)))


def dv_duration(item_xml=None):
    if item_xml is None:
        return c_complex("DV_DURATION")
    return c_complex("DV_DURATION", c_single_attr("value", c_primitive_object("DURATION", item_xml)))


def dv_coded_text_local(codes, term="local"):
    dc = c_single_attr("defining_code", c_code_phrase(term, codes))
    return c_complex("DV_CODED_TEXT", dc)


def dv_coded_text_ref(reference):
    dc = c_single_attr("defining_code", constraint_ref(reference))
    return c_complex("DV_CODED_TEXT", dc)


def dv_ordinal_open():
    return c_complex("DV_ORDINAL")


def dv_ordinal_list():
    items = [
        (1, "local", "at0005", "mild"),
        (2, "local", "at0006", "severe"),
    ]
    return c_dv_ordinal(items)


def dv_scale_open():
    return c_complex("DV_SCALE")


def dv_scale_list():
    # AOM 1.4 has no C_DV_SCALE; express as C_COMPLEX_OBJECT over DV_SCALE with
    # a C_REAL value constraint (representative). Spec-silence flagged above.
    val = c_single_attr("value", c_primitive_object("REAL", item_c_real_list([1.5, 2.0])))
    return c_complex("DV_SCALE", val)


def dv_quantity_open():
    return c_complex("DV_QUANTITY")


def dv_multimedia(media_codes=None, size_item=None):
    attrs = []
    if media_codes is not None:
        attrs.append(c_single_attr("media_type", c_code_phrase("IANA_media-types", media_codes)))
    if size_item is not None:
        attrs.append(c_single_attr("size", c_primitive_object("INTEGER", size_item)))
    return c_complex("DV_MULTIMEDIA", "".join(attrs))


def dv_interval(inner_rm_type, inner_lower_xml=None, inner_upper_xml=None):
    """DV_INTERVAL<inner_rm_type>. inner_*_xml are full <children> blocks for
    the lower/upper limit objects; None => open interval (no limit constraints)."""
    attrs = []
    if inner_lower_xml is not None:
        attrs.append(c_single_attr("lower", inner_lower_xml, exist=(0, 1)))
    if inner_upper_xml is not None:
        attrs.append(c_single_attr("upper", inner_upper_xml, exist=(0, 1)))
    return c_complex(f"DV_INTERVAL<{inner_rm_type}>", "".join(attrs))


# ---------------------------------------------------------------------------
# The OBSERVATION-carrier assembler (mirrors the vendored minimal_observation
# skeleton exactly; only the ELEMENT.value child and term_definitions vary).
# ---------------------------------------------------------------------------
def obs_term_defs(extra_terms=None):
    base = [
        ("at0000", "Minimal", "unknown"),
        ("at0001", "Event Series", "@ internal @"),
        ("at0002", "Any event", "*"),
        ("at0003", "Tree", "@ internal @"),
        ("at0004", "value", "*"),
    ]
    if extra_terms:
        for code in sorted(extra_terms):
            text, desc = extra_terms[code]
            base.append((code, text, desc))
    out = []
    for code, text, desc in base:
        out.append(
            f'<term_definitions code="{code}">'
            f'<items id="description">{xesc(desc)}</items>'
            f'<items id="text">{xesc(text)}</items>'
            "</term_definitions>"
        )
    return "".join(out)


def observation_root(value_children, extra_terms=None, element_occ=(0, 1)):
    """The <children xsi:type="C_ARCHETYPE_ROOT">OBSERVATION...</children>."""
    value_attr = c_single_attr("value", value_children, exist=(0, 1))
    element = (
        '<children xsi:type="C_COMPLEX_OBJECT">'
        "<rm_type_name>ELEMENT</rm_type_name>"
        f"{occ(*element_occ)}"
        "<node_id>at0004</node_id>"
        f"{value_attr}"
        "</children>"
    )
    items_attr = c_multiple_attr("items", element, cardinality(0, None), exist=(0, 1))
    item_tree = c_complex("ITEM_TREE", items_attr, node_id="at0003")
    data_attr = c_single_attr("data", item_tree, exist=(1, 1))
    event = c_complex("EVENT", data_attr, node_id="at0002", occ_lu=(0, 1))
    events_attr = c_multiple_attr("events", event, cardinality(1, None), exist=(1, 1))
    history = c_complex("HISTORY", events_attr, node_id="at0001")
    hist_data = c_single_attr("data", history, exist=(1, 1))
    return (
        '<children xsi:type="C_ARCHETYPE_ROOT">'
        "<rm_type_name>OBSERVATION</rm_type_name>"
        "<occurrences><lower_included>true</lower_included><lower_unbounded>false</lower_unbounded>"
        "<upper_unbounded>true</upper_unbounded><lower>0</lower></occurrences>"
        "<node_id>at0000</node_id>"
        f"{hist_data}"
        "<archetype_id><value>openEHR-EHR-OBSERVATION.minimal.v1</value></archetype_id>"
        f"{obs_term_defs(extra_terms)}"
        "</children>"
    )


def blood_pressure_root():
    """The AQL-corpus blood-pressure OBSERVATION: the exact archetype/node
    ids the bp_series recipe commits (openEHR-EHR-OBSERVATION.blood_pressure.v2;
    HISTORY at0001 / POINT_EVENT at0006 / ITEM_TREE at0003 / systolic at0004 +
    diastolic at0005 DV_QUANTITY) — recipe contract corpus/recipes/bp_series.md."""

    def element(node_id, label_code):
        qty = c_complex(
            "DV_QUANTITY",
            c_single_attr(
                "units", c_primitive_object("STRING", item_c_string_list(["mm[Hg]"]))
            ),
        )
        return (
            '<children xsi:type="C_COMPLEX_OBJECT">'
            "<rm_type_name>ELEMENT</rm_type_name>"
            f"{occ(0, 1)}"
            f"<node_id>{node_id}</node_id>"
            f"{c_single_attr('value', qty, exist=(0, 1))}"
            "</children>"
        )

    items_attr = c_multiple_attr(
        "items", element("at0004", "") + element("at0005", ""), cardinality(0, None), exist=(0, 1)
    )
    item_tree = c_complex("ITEM_TREE", items_attr, node_id="at0003")
    data_attr = c_single_attr("data", item_tree, exist=(1, 1))
    event = c_complex("POINT_EVENT", data_attr, node_id="at0006", occ_lu=(0, 1))
    events_attr = c_multiple_attr("events", event, cardinality(1, None), exist=(1, 1))
    history = c_complex("HISTORY", events_attr, node_id="at0001")
    hist_data = c_single_attr("data", history, exist=(1, 1))
    terms = (
        '<term_definitions code="at0000"><items id="description">unknown</items><items id="text">Blood pressure</items></term_definitions>'
        '<term_definitions code="at0001"><items id="description">@ internal @</items><items id="text">history</items></term_definitions>'
        '<term_definitions code="at0003"><items id="description">@ internal @</items><items id="text">blood pressure</items></term_definitions>'
        '<term_definitions code="at0004"><items id="description">systolic pressure</items><items id="text">Systolic</items></term_definitions>'
        '<term_definitions code="at0005"><items id="description">diastolic pressure</items><items id="text">Diastolic</items></term_definitions>'
        '<term_definitions code="at0006"><items id="description">any event</items><items id="text">any event</items></term_definitions>'
    )
    return (
        '<children xsi:type="C_ARCHETYPE_ROOT">'
        "<rm_type_name>OBSERVATION</rm_type_name>"
        "<occurrences><lower_included>true</lower_included><lower_unbounded>false</lower_unbounded>"
        "<upper_unbounded>true</upper_unbounded><lower>0</lower></occurrences>"
        "<node_id>at0000</node_id>"
        f"{hist_data}"
        "<archetype_id><value>openEHR-EHR-OBSERVATION.blood_pressure.v2</value></archetype_id>"
        f"{terms}"
        "</children>"
    )


def composition(template_id, obs_children, content_card=None, context_exist=None):
    uid = det_uid(template_id)
    card = content_card if content_card is not None else cardinality(0, None)
    content_attr = c_multiple_attr("content", obs_children, card, exist=(0, 1))
    category = c_single_attr(
        "category",
        c_complex(
            "DV_CODED_TEXT",
            c_single_attr("defining_code", c_code_phrase("openehr", ["433"])),
        ),
    )
    context_attr = ""
    if context_exist is not None:
        context_attr = c_single_attr(
            "context", c_complex("EVENT_CONTEXT", occ_lu=(1, 1)), exist=context_exist
        )
    return f"""<?xml version="1.0" encoding="utf-8"?>
<template xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns="http://schemas.openehr.org/v1">
  <language><terminology_id><value>ISO_639-1</value></terminology_id><code_string>en</code_string></language>
  <description><original_author id="Original Author">CNF corpus</original_author><lifecycle_state>Initial</lifecycle_state><details><language><terminology_id><value>ISO_639-1</value></terminology_id><code_string>en</code_string></language><purpose>CNF content constraint template</purpose></details></description>
  <uid><value>{uid}</value></uid>
  <template_id><value>{xesc(template_id)}</value></template_id>
  <concept>CNF content constraint</concept>
  <definition>
    <rm_type_name>COMPOSITION</rm_type_name>
    {occ(1, 1)}
    <node_id>at0000</node_id>
    {category}
    {context_attr}
    {content_attr}
    <archetype_id><value>openEHR-EHR-COMPOSITION.minimal.v1</value></archetype_id>
    <template_id><value>{xesc(template_id)}</value></template_id>
    <term_definitions code="at0000"><items id="description">unknown</items><items id="text">Minimal</items></term_definitions>
  </definition>
</template>
"""


def value_template(template_id, value_children, extra_terms=None):
    obs = observation_root(value_children, extra_terms)
    return composition(template_id, obs)


# ---------------------------------------------------------------------------
# EVALUATION carrier (for the ITEM_STRUCTURE type-narrowing case).
# ---------------------------------------------------------------------------
def evaluation_template(template_id, data_rm_type):
    data_child = c_complex(data_rm_type, node_id="at0003")
    data_attr = c_single_attr("data", data_child, exist=(1, 1))
    eval_root = (
        '<children xsi:type="C_ARCHETYPE_ROOT">'
        "<rm_type_name>EVALUATION</rm_type_name>"
        "<occurrences><lower_included>true</lower_included><lower_unbounded>false</lower_unbounded>"
        "<upper_unbounded>true</upper_unbounded><lower>0</lower></occurrences>"
        "<node_id>at0000</node_id>"
        f"{data_attr}"
        "<archetype_id><value>openEHR-EHR-EVALUATION.minimal.v1</value></archetype_id>"
        '<term_definitions code="at0000"><items id="description">unknown</items><items id="text">Minimal</items></term_definitions>'
        '<term_definitions code="at0003"><items id="description">@ internal @</items><items id="text">structure</items></term_definitions>'
        "</children>"
    )
    return composition(template_id, eval_root)


# ---------------------------------------------------------------------------
# Structural templates (ecc_*): occurrences / existence / cardinality / type.
# ---------------------------------------------------------------------------
def structural_content_cardinality(template_id):
    # C_MULTIPLE_ATTRIBUTE.cardinality on COMPOSITION.content = 3..5 (representative).
    obs = observation_root(dv_string_type("DV_TEXT"))
    return composition(template_id, obs, content_card=cardinality(3, 5))


def structural_context_existence(template_id):
    # C_ATTRIBUTE.existence on COMPOSITION.context = 1..1 (archetype-tightened).
    obs = observation_root(dv_string_type("DV_TEXT"))
    return composition(template_id, obs, context_exist=(1, 1))


def structural_observation_state_protocol(template_id):
    # OBSERVATION.state + protocol existence mandatory (1..1).
    value_attr = c_single_attr("value", dv_string_type("DV_TEXT"), exist=(0, 1))
    element = (
        '<children xsi:type="C_COMPLEX_OBJECT"><rm_type_name>ELEMENT</rm_type_name>'
        f"{occ(0, 1)}<node_id>at0004</node_id>{value_attr}</children>"
    )
    items_attr = c_multiple_attr("items", element, cardinality(0, None), exist=(0, 1))
    item_tree = c_complex("ITEM_TREE", items_attr, node_id="at0003")
    data_attr = c_single_attr("data", item_tree, exist=(1, 1))
    event = c_complex("EVENT", data_attr, node_id="at0002", occ_lu=(0, 1))
    events_attr = c_multiple_attr("events", event, cardinality(1, None), exist=(1, 1))
    history = c_complex("HISTORY", events_attr, node_id="at0001")
    hist_data = c_single_attr("data", history, exist=(1, 1))
    # state (HISTORY) and protocol (ITEM_STRUCTURE) mandatory.
    state_hist = c_complex("HISTORY", events_attr, node_id="at0005")
    state_attr = c_single_attr("state", state_hist, exist=(1, 1))
    protocol = c_complex("ITEM_TREE", node_id="at0006")
    protocol_attr = c_single_attr("protocol", protocol, exist=(1, 1))
    obs = (
        '<children xsi:type="C_ARCHETYPE_ROOT"><rm_type_name>OBSERVATION</rm_type_name>'
        "<occurrences><lower_included>true</lower_included><lower_unbounded>false</lower_unbounded>"
        "<upper_unbounded>true</upper_unbounded><lower>0</lower></occurrences>"
        "<node_id>at0000</node_id>"
        f"{hist_data}{state_attr}{protocol_attr}"
        "<archetype_id><value>openEHR-EHR-OBSERVATION.minimal.v1</value></archetype_id>"
        '<term_definitions code="at0000"><items id="description">unknown</items><items id="text">Minimal</items></term_definitions>'
        '<term_definitions code="at0001"><items id="description">@ internal @</items><items id="text">Event Series</items></term_definitions>'
        '<term_definitions code="at0002"><items id="description">*</items><items id="text">Any event</items></term_definitions>'
        '<term_definitions code="at0003"><items id="description">@ internal @</items><items id="text">Tree</items></term_definitions>'
        '<term_definitions code="at0004"><items id="description">*</items><items id="text">value</items></term_definitions>'
        '<term_definitions code="at0005"><items id="description">@ internal @</items><items id="text">State</items></term_definitions>'
        '<term_definitions code="at0006"><items id="description">@ internal @</items><items id="text">Protocol</items></term_definitions>'
        "</children>"
    )
    return composition(template_id, obs)


def structural_history(template_id, events_card=None, summary_exist=None):
    value_attr = c_single_attr("value", dv_string_type("DV_TEXT"), exist=(0, 1))
    element = (
        '<children xsi:type="C_COMPLEX_OBJECT"><rm_type_name>ELEMENT</rm_type_name>'
        f"{occ(0, 1)}<node_id>at0004</node_id>{value_attr}</children>"
    )
    items_attr = c_multiple_attr("items", element, cardinality(0, None), exist=(0, 1))
    item_tree = c_complex("ITEM_TREE", items_attr, node_id="at0003")
    data_attr = c_single_attr("data", item_tree, exist=(1, 1))
    event = c_complex("EVENT", data_attr, node_id="at0002", occ_lu=(0, 1))
    card = events_card if events_card is not None else cardinality(1, None)
    events_attr = c_multiple_attr("events", event, card, exist=(1, 1))
    summary_attr = ""
    if summary_exist is not None:
        summary_attr = c_single_attr(
            "summary", c_complex("ITEM_TREE", node_id="at0007"), exist=summary_exist
        )
    history = (
        '<children xsi:type="C_COMPLEX_OBJECT"><rm_type_name>HISTORY</rm_type_name>'
        f"{occ(1, 1)}<node_id>at0001</node_id>{events_attr}{summary_attr}</children>"
    )
    hist_data = c_single_attr("data", history, exist=(1, 1))
    extra = ""
    if summary_exist is not None:
        extra = '<term_definitions code="at0007"><items id="description">@ internal @</items><items id="text">Summary</items></term_definitions>'
    obs = (
        '<children xsi:type="C_ARCHETYPE_ROOT"><rm_type_name>OBSERVATION</rm_type_name>'
        "<occurrences><lower_included>true</lower_included><lower_unbounded>false</lower_unbounded>"
        "<upper_unbounded>true</upper_unbounded><lower>0</lower></occurrences>"
        "<node_id>at0000</node_id>"
        f"{hist_data}"
        "<archetype_id><value>openEHR-EHR-OBSERVATION.minimal.v1</value></archetype_id>"
        '<term_definitions code="at0000"><items id="description">unknown</items><items id="text">Minimal</items></term_definitions>'
        '<term_definitions code="at0001"><items id="description">@ internal @</items><items id="text">Event Series</items></term_definitions>'
        '<term_definitions code="at0002"><items id="description">*</items><items id="text">Any event</items></term_definitions>'
        '<term_definitions code="at0003"><items id="description">@ internal @</items><items id="text">Tree</items></term_definitions>'
        '<term_definitions code="at0004"><items id="description">*</items><items id="text">value</items></term_definitions>'
        f"{extra}"
        "</children>"
    )
    return composition(template_id, obs)


def structural_event(template_id, events_slot_type="EVENT", state_exist=None):
    value_attr = c_single_attr("value", dv_string_type("DV_TEXT"), exist=(0, 1))
    element = (
        '<children xsi:type="C_COMPLEX_OBJECT"><rm_type_name>ELEMENT</rm_type_name>'
        f"{occ(0, 1)}<node_id>at0004</node_id>{value_attr}</children>"
    )
    items_attr = c_multiple_attr("items", element, cardinality(0, None), exist=(0, 1))
    item_tree = c_complex("ITEM_TREE", items_attr, node_id="at0003")
    data_attr = c_single_attr("data", item_tree, exist=(1, 1))
    state_attr = ""
    if state_exist is not None:
        state_attr = c_single_attr(
            "state", c_complex("ITEM_TREE", node_id="at0005"), exist=state_exist
        )
    event = c_complex(events_slot_type, data_attr + state_attr, node_id="at0002", occ_lu=(0, 1))
    events_attr = c_multiple_attr("events", event, cardinality(1, None), exist=(1, 1))
    history = c_complex("HISTORY", events_attr, node_id="at0001")
    hist_data = c_single_attr("data", history, exist=(1, 1))
    extra = ""
    if state_exist is not None:
        extra = '<term_definitions code="at0005"><items id="description">@ internal @</items><items id="text">State</items></term_definitions>'
    obs = (
        '<children xsi:type="C_ARCHETYPE_ROOT"><rm_type_name>OBSERVATION</rm_type_name>'
        "<occurrences><lower_included>true</lower_included><lower_unbounded>false</lower_unbounded>"
        "<upper_unbounded>true</upper_unbounded><lower>0</lower></occurrences>"
        "<node_id>at0000</node_id>"
        f"{hist_data}"
        "<archetype_id><value>openEHR-EHR-OBSERVATION.minimal.v1</value></archetype_id>"
        '<term_definitions code="at0000"><items id="description">unknown</items><items id="text">Minimal</items></term_definitions>'
        '<term_definitions code="at0001"><items id="description">@ internal @</items><items id="text">Event Series</items></term_definitions>'
        '<term_definitions code="at0002"><items id="description">*</items><items id="text">Any event</items></term_definitions>'
        '<term_definitions code="at0003"><items id="description">@ internal @</items><items id="text">Tree</items></term_definitions>'
        '<term_definitions code="at0004"><items id="description">*</items><items id="text">value</items></term_definitions>'
        f"{extra}"
        "</children>"
    )
    return composition(template_id, obs)


# ---------------------------------------------------------------------------
# Interval limit-object builders (the inner lower/upper C_* objects).
# ---------------------------------------------------------------------------
def _limit_count_range():
    return dv_count(item_c_integer_range(0, 100))


def _limit_count_list():
    return dv_count(item_c_integer_list([0, 5, 10, 100]))


def _limit_quantity_temp():
    return c_dv_quantity("openehr", "127", [c_quantity_item("Cel", 0.0, 100.0)])


def _limit_date_range():
    return dv_date(item_c_date_range("1900-01-01", "2030-12-31"))


def _limit_date_validity():
    return dv_date(item_c_date("yyyy-mm-??"))


def _limit_time_range():
    return dv_time(item_c_time("HH:MM:SS"))


def _limit_time_validity():
    return dv_time(item_c_time("HH:MM:??"))


def _limit_date_time_range():
    return dv_date_time(item_c_date_time("yyyy-mm-ddTHH:MM:SS"))


def _limit_date_time_validity():
    return dv_date_time(item_c_date_time("yyyy-mm-ddTHH:MM:??"))


def _limit_duration(pattern="PYMWDTHMS"):
    return dv_duration(item_c_duration(pattern))


def _limit_ordinal_list():
    return dv_ordinal_list()


def _limit_proportion_type(type_vals):
    tp = c_single_attr("type", c_primitive_object("INTEGER", item_c_integer_list(type_vals)))
    return c_complex("DV_PROPORTION", tp)


def _limit_proportion_ratio_range():
    tp = c_single_attr("type", c_primitive_object("INTEGER", item_c_integer_list([0])))
    num = c_single_attr("numerator", c_primitive_object("REAL", item_c_real_range(5.0, 20.0)))
    den = c_single_attr("denominator", c_primitive_object("REAL", item_c_real_range(200.0, 600.0)))
    return c_complex("DV_PROPORTION", tp + num + den)


# ---------------------------------------------------------------------------
# Proportion / scale value builders (non-interval).
# ---------------------------------------------------------------------------
def dv_proportion_open():
    return c_complex("DV_PROPORTION")


def dv_proportion_type(type_vals):
    return _limit_proportion_type(type_vals)


def dv_proportion_ratio_range():
    return _limit_proportion_ratio_range()


# ---------------------------------------------------------------------------
# The constraint table: filename -> (builder callable). Each entry documents
# the content case(s) it serves.
# ---------------------------------------------------------------------------
def build_all():
    T = {}

    # --- DV_COUNT (CONT-DV_COUNT-validate_{list,range,open}) ---
    T["count_list"] = lambda k: value_template(k, dv_count(item_c_integer_list([10, 15, 20])))
    T["count_range"] = lambda k: value_template(k, dv_count(item_c_integer_range(10, 20)))
    T["count_open"] = lambda k: value_template(k, dv_count())

    # --- DV_BOOLEAN (CONT-DV_BOOLEAN-{anything_allowed,only_true,only_false}) ---
    # Shared key: bake C_BOOLEAN true-only (true_valid, !false_valid).
    T["dv_boolean_c_boolean"] = lambda k: value_template(k, dv_boolean(item_c_boolean(True, False)))

    # --- DV_CODED_TEXT ---
    # local codes (CONT-DV_CODED_TEXT-validate_{local_codes,open}) ---
    T["dv_coded_text_c_code_phrase"] = lambda k: value_template(k, dv_coded_text_local(["ABC", "OPQ"]))
    # external terminology via CONSTRAINT_REF (CONT-DV_CODED_TEXT-validate_ext_term) ---
    T["dv_coded_text_constraint_ref"] = lambda k: value_template(k, dv_coded_text_ref("ac0001"))

    # --- DV_DATE (CONT-DV_DATE-validate_{constraint,open,range}) ---
    T["dv_date_c_date"] = lambda k: value_template(k, dv_date(item_c_date("yyyy-mm-??")))

    # --- DV_DATE_TIME (CONT-DV_DATE_TIME-validate_{constraint,open,range}) ---
    T["dv_date_time_c_date_time"] = lambda k: value_template(k, dv_date_time(item_c_date_time("yyyy-mm-ddTHH:MM:??")))

    # --- DV_DURATION (CONT-DV_DURATION-validate_{fields,fields_range,open,range}) ---
    T["dv_duration_c_duration"] = lambda k: value_template(k, dv_duration(item_c_duration("PYMWDTHMS")))

    # --- DV_EHR_URI (CONT-DV_EHR_URI-validate_{list,open,pattern}) ---
    T["dv_ehr_uri_c_string"] = lambda k: value_template(k, dv_string_type("DV_EHR_URI", item_c_string_pattern("ehr://.*")))

    # --- DV_IDENTIFIER (CONT-DV_IDENTIFIER-validate_all_{list,pattern}) ---
    T["dv_identifier_c_string"] = lambda k: value_template(k, dv_string_type("DV_IDENTIFIER", item_c_string_pattern(".*"), field="id"))

    # --- DV_MULTIMEDIA (CONT-DV_MULTIMEDIA-validate_{media_type,open}) ---
    T["dv_multimedia_constraints"] = lambda k: value_template(
        k, dv_multimedia(media_codes=["application/dicom", "text/plain", "text/html"], size_item=item_c_integer_range(0, 1000))
    )

    # --- DV_PARSABLE (CONT-DV_PARSABLE-validate_{open,value_formalism}) ---
    T["dv_parsable_c_string"] = lambda k: value_template(k, dv_string_type("DV_PARSABLE", item_c_string_pattern(".*"), field="formalism"))

    # --- DV_TEXT (CONT-DV_TEXT-validate_{list,open}) ---
    T["dv_text_c_string"] = lambda k: value_template(k, dv_string_type("DV_TEXT", item_c_string_list(["red", "green", "blue"])))

    # --- DV_TIME (CONT-DV_TIME-validate_{constraint,open,range}) ---
    T["dv_time_c_time"] = lambda k: value_template(k, dv_time(item_c_time("HH:MM:??")))

    # --- DV_URI (CONT-DV_URI-validate_{list,open,pattern}) ---
    T["dv_uri_c_string"] = lambda k: value_template(k, dv_string_type("DV_URI", item_c_string_pattern("https?://.*")))

    # --- DV_ORDINAL (CONT-DV_ORDINAL-validate_{constraint,open}) ---
        # ── split keys (2026-07-22): one constraint per case — the shared generic
    # templates baked a single representative constraint and made sibling
    # cases' accepted rows fail (open tables) or rejected rows pass (lists) ──
    # CONT-DV_BOOLEAN-anything_allowed: open C_BOOLEAN (both values valid)
    T["dv_boolean_open"] = lambda k: value_template(k, dv_boolean())
    # CONT-DV_BOOLEAN-only_false_allowed: true invalid, false valid
    T["dv_boolean_only_false"] = lambda k: value_template(k, dv_boolean(item_c_boolean(False, True)))
    # CONT-DV_DATE/TIME/DATE_TIME-validate_open: open temporal values
    T["dv_date_open"] = lambda k: value_template(k, dv_date())
    T["dv_time_open"] = lambda k: value_template(k, dv_time())
    T["dv_date_time_open"] = lambda k: value_template(k, dv_date_time())
    # CONT-DV_URI/DV_EHR_URI-validate_open: open string values (RM invariants only)
    T["dv_uri_open"] = lambda k: value_template(k, dv_string_type("DV_URI", None))
    T["dv_ehr_uri_open"] = lambda k: value_template(k, dv_string_type("DV_EHR_URI", None))
    # CONT-DV_EHR_URI-validate_list: C_STRING.list of the case's ehr URIs
    T["dv_ehr_uri_list"] = lambda k: value_template(
    k,
    dv_string_type(
        "DV_EHR_URI",
        item_c_string_list(
            [
                "ehr:/89c0752e-0815-47d7-8b3c-b3aaea2cea7a",
                "ehr://CLOUD_EHRSERVER/89c0752e-0815-47d7-8b3c-b3aaea2cea7a",
            ]
        ),
    ),
    )
    # CONT-DV_TEXT-validate_list: the case's C_STRING.list {XYZ, OPQ}
    T["dv_text_list"] = lambda k: value_template(k, dv_string_type("DV_TEXT", item_c_string_list(["XYZ", "OPQ"])))
    T["ordinal_list"] = lambda k: value_template(
        k, dv_ordinal_list(), extra_terms={"at0005": ("mild", "mild"), "at0006": ("severe", "severe")}
    )
    T["ordinal_open"] = lambda k: value_template(k, dv_ordinal_open())

    # --- DV_SCALE (CONT-DV_SCALE-validate_{constraint,open}) — C_DV_SCALE not in AOM 1.4 ---
    T["scale_list"] = lambda k: value_template(k, dv_scale_list())
    T["scale_open"] = lambda k: value_template(k, dv_scale_open())

    # --- DV_QUANTITY ---
    T["quantity_open"] = lambda k: value_template(k, dv_quantity_open())
    T["quantity_property"] = lambda k: value_template(k, c_dv_quantity("openehr", "122", []))
    T["quantity_property_units"] = lambda k: value_template(
        k, c_dv_quantity("openehr", "122", [c_quantity_item("cm"), c_quantity_item("m")])
    )
    T["quantity_property_units_mag"] = lambda k: value_template(
        k, c_dv_quantity("openehr", "122", [c_quantity_item("cm", 5.0, 10.0), c_quantity_item("m")])
    )

    # --- DV_PROPORTION (kind = type integer code) ---
    T["proportion_open"] = lambda k: value_template(k, dv_proportion_open())
    T["proportion_type_ratio"] = lambda k: value_template(k, dv_proportion_type([0]))
    T["proportion_type_unitary"] = lambda k: value_template(k, dv_proportion_type([1]))
    T["proportion_type_percent"] = lambda k: value_template(k, dv_proportion_type([2]))
    T["proportion_type_fraction"] = lambda k: value_template(k, dv_proportion_type([3]))
    T["proportion_type_integer_fraction"] = lambda k: value_template(k, dv_proportion_type([4]))
    T["proportion_type_any_fraction"] = lambda k: value_template(k, dv_proportion_type([3, 4]))
    T["proportion_ratio_range"] = lambda k: value_template(k, dv_proportion_ratio_range())

    # --- DV_INTERVAL<DV_COUNT> ---
    T["interval_count_open"] = lambda k: value_template(k, dv_interval("DV_COUNT"))
    T["interval_count_range"] = lambda k: value_template(k, dv_interval("DV_COUNT", _limit_count_range(), _limit_count_range()))
    T["interval_count_list"] = lambda k: value_template(k, dv_interval("DV_COUNT", _limit_count_list(), _limit_count_list()))

    # --- DV_INTERVAL<DV_QUANTITY> ---
    T["interval_quantity_open"] = lambda k: value_template(k, dv_interval("DV_QUANTITY"))
    T["interval_quantity_temp"] = lambda k: value_template(k, dv_interval("DV_QUANTITY", _limit_quantity_temp(), _limit_quantity_temp()))

    # --- DV_INTERVAL<DV_DATE> ---
    T["interval_date_open"] = lambda k: value_template(k, dv_interval("DV_DATE"))
    T["interval_date_range"] = lambda k: value_template(k, dv_interval("DV_DATE", _limit_date_range(), _limit_date_range()))
    T["interval_date_field_validity"] = lambda k: value_template(k, dv_interval("DV_DATE", _limit_date_validity(), _limit_date_validity()))

    # --- DV_INTERVAL<DV_DATE_TIME> ---
    T["interval_date_time_open"] = lambda k: value_template(k, dv_interval("DV_DATE_TIME"))
    T["interval_date_time_range"] = lambda k: value_template(k, dv_interval("DV_DATE_TIME", _limit_date_time_range(), _limit_date_time_range()))
    T["interval_date_time_field_validity"] = lambda k: value_template(k, dv_interval("DV_DATE_TIME", _limit_date_time_validity(), _limit_date_time_validity()))

    # --- DV_INTERVAL<DV_TIME> ---
    T["interval_time_open"] = lambda k: value_template(k, dv_interval("DV_TIME"))
    T["interval_time_range"] = lambda k: value_template(k, dv_interval("DV_TIME", _limit_time_range(), _limit_time_range()))
    T["interval_time_field_validity"] = lambda k: value_template(k, dv_interval("DV_TIME", _limit_time_validity(), _limit_time_validity()))

    # --- DV_INTERVAL<DV_DURATION> ---
    T["interval_duration_open"] = lambda k: value_template(k, dv_interval("DV_DURATION"))
    T["interval_duration_range"] = lambda k: value_template(k, dv_interval("DV_DURATION", _limit_duration(), _limit_duration()))
    T["interval_duration_allowed"] = lambda k: value_template(k, dv_interval("DV_DURATION", _limit_duration("PYMD"), _limit_duration("PYMD")))

    # --- DV_INTERVAL<DV_ORDINAL> ---
    T["interval_ordinal_open"] = lambda k: value_template(k, dv_interval("DV_ORDINAL"))
    T["interval_ordinal_list"] = lambda k: value_template(
        k, dv_interval("DV_ORDINAL", _limit_ordinal_list(), _limit_ordinal_list()),
        extra_terms={"at0005": ("mild", "mild"), "at0006": ("severe", "severe")},
    )

    # --- DV_INTERVAL<DV_SCALE> — C_DV_SCALE not in AOM 1.4 ---
    T["interval_scale_open"] = lambda k: value_template(k, dv_interval("DV_SCALE"))
    T["interval_scale_list"] = lambda k: value_template(
        k, dv_interval("DV_SCALE", dv_scale_list(), dv_scale_list())
    )

    # --- DV_INTERVAL<DV_PROPORTION> ---
    T["interval_proportion_open"] = lambda k: value_template(k, dv_interval("DV_PROPORTION"))
    T["interval_proportion_type_fraction"] = lambda k: value_template(k, dv_interval("DV_PROPORTION", _limit_proportion_type([3]), _limit_proportion_type([3])))
    T["interval_proportion_type_integer_fraction"] = lambda k: value_template(k, dv_interval("DV_PROPORTION", _limit_proportion_type([4]), _limit_proportion_type([4])))
    T["interval_proportion_type_percent"] = lambda k: value_template(k, dv_interval("DV_PROPORTION", _limit_proportion_type([2]), _limit_proportion_type([2])))
    T["interval_proportion_type_ratio"] = lambda k: value_template(k, dv_interval("DV_PROPORTION", _limit_proportion_type([0]), _limit_proportion_type([0])))
    T["interval_proportion_type_unitary"] = lambda k: value_template(k, dv_interval("DV_PROPORTION", _limit_proportion_type([1]), _limit_proportion_type([1])))
    T["interval_proportion_ratio_range"] = lambda k: value_template(k, dv_interval("DV_PROPORTION", _limit_proportion_ratio_range(), _limit_proportion_ratio_range()))

    # --- Structural cases: occurrences / existence / cardinality / type-narrowing.
    # The `ecc_` token in these keys/filenames is inherited verbatim from the
    # existing corpus (the manifest keys + each structural case's
    # constraint_context.template) — the case YAMLs record it as the retired
    # structural battery's placeholder id namespace, not the ECC runner. The
    # filenames/keys are fixed by those references and cannot be renamed here. ---
    T["ecc_composition_content_cardinality"] = lambda k: structural_content_cardinality(k)
    T["ecc_composition_context_existence"] = lambda k: structural_context_existence(k)
    T["ecc_event_state_existence"] = lambda k: structural_event(k, state_exist=(1, 1))
    T["ecc_event_type_narrowing"] = lambda k: structural_event(k, events_slot_type="POINT_EVENT")
    T["ecc_history_events_cardinality"] = lambda k: structural_history(k, events_card=cardinality(3, 5))
    T["ecc_history_summary_existence"] = lambda k: structural_history(k, summary_exist=(1, 1))
    T["ecc_item_structure_type_narrowing"] = lambda k: evaluation_template(k, "ITEM_TREE")
    T["ecc_observation_state_protocol_existence"] = lambda k: structural_observation_state_protocol(k)

    return T


def manifest_sources():
    """Map cnf.tpl.<suffix> key -> source basename, read from the governed
    MANIFEST (authoritative: several keys use a `dt_`/`cnf.tpl.` filename that
    differs from the key suffix). The template_id inside each OPT is the KEY."""
    import re

    path = os.path.join(HERE, "..", "MANIFEST.yaml")
    txt = open(path, encoding="utf-8").read()
    pat = re.compile(
        r"^(cnf\.tpl\.[^\s:]+):\n(?:  .*\n)*?  source: templates/([^\s]+)", re.M
    )
    return {m.group(1): m.group(2) for m in pat.finditer(txt)}


def main():
    table = build_all()
    sources = manifest_sources()
    written = []
    missing = []
    for name in sorted(table):
        key = f"cnf.tpl.{name}"
        fname = sources.get(key)
        if fname is None:
            missing.append(key)
            continue
        xml = table[name](key)
        path = os.path.join(HERE, fname)
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(xml)
        written.append((fname, key))
    # the AQL-corpus committed-set template (cnf.opt.blood_pressure): the
    # template_id is the one the bp_series recipe stamps into its
    # compositions (corpus/recipes/bp_series.md)
    bp_path = os.path.join(HERE, "blood_pressure.opt")
    with open(bp_path, "w", encoding="utf-8") as fh:
        fh.write(composition("cnf.blood_pressure", blood_pressure_root()))
    written.append(("blood_pressure.opt", "cnf.blood_pressure"))
    print(f"generated {len(written)} OPTs")
    for fname, key in written:
        print(f"  {fname} -> template_id {key}")
    if missing:
        print(f"WARNING: {len(missing)} keys in the table have no manifest source:")
        for k in missing:
            print(f"  {k}")
    extra = set(f"cnf.tpl.{n}" for n in table) ^ set(sources)
    if extra:
        print(f"WARNING: key-set mismatch vs manifest: {sorted(extra)}")


if __name__ == "__main__":
    main()
