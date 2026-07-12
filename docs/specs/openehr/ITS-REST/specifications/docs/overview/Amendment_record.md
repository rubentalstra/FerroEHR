[comment]: # (title: Amendment Record)

<table>
    <colgroup>
        <col style="width: 9%;">
        <col style="width: 55%;">
        <col style="width: 18%;">
        <col style="width: 18%;">
    </colgroup>
    <thead>
    <tr>
        <th>Issue</th>
        <th>Details</th>
        <th>Raiser, Implementer</th>
        <th>Completed</th>
    </tr>
    </thead>
    <tbody>
    <tr>
        <td>5.9</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-95" target="_blank" rel="noopener">SPECITS-95</a>:
            Fix <code>UPDATE_AUDIT.change_type</code> typing - use <code>DV_CODED_TEXT</code> instead of <code>TERMINOLOGY_CODE</code> (same correction applied to <code>UPDATE_VERSION.lifecycle_state</code> in the ehr and demographic APIs); add optional <code>system_id</code>, make the examples consistent with the schema, clarify that <code>UPDATE_AUDIT</code> is <code>AUDIT_DETAILS</code> minus server-assigned attributes including the <code>_type</code> handling, and remove the now-unused <code>TERMINOLOGY_CODE</code> base schema</td>
        <td>H Heiser, S Iancu</td>
        <td>11 Jun 2026</td>
    </tr>
    <tr>
        <td>5.8</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECPR-472" target="_blank" rel="noopener">SPECPR-472</a>:
            Document support for the <code>system_id</code> attribute on the <code>openehr-audit-details</code> request header</td>
        <td>P Pazos, S Iancu</td>
        <td>28 Apr 2026</td>
    </tr>
    <tr>
        <td>5.7</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-92" target="_blank" rel="noopener">SPECITS-92</a>:
            Fix alignment issues and sync OpenAPI resource and type schemas with openEHR Reference Model type specifications (mostly RM 1.1.0 related)</td>
        <td>S Iancu</td>
        <td>27 Apr 2026</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-84" target="_blank" rel="noopener">SPECITS-84</a>:
            Add support for FLAT/SDT to Contribution REST API calls - accept the Simplified Formats MIME types on CONTRIBUTION endpoints; the Simplified Formats serialization applies to <code>versions[].data</code> only, the CONTRIBUTION envelope remains canonical</td>
        <td>REST WG</td>
        <td>27 Apr 2026</td>
    </tr>
    <tr>
        <td>5.6</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-88" target="_blank" rel="noopener">SPECITS-88</a>:
            Add support for filters in listTemplates operation</td>
        <td>REST WG</td>
        <td>11 Feb 2026</td>
    </tr>
    <tr>
        <td>5.5</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-58" target="_blank" rel="noopener">SPECITS-58</a>:
            Add support for /example sub-resource under template-definition endpoint</td>
        <td>REST WG</td>
        <td>17 Dec 2025</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-34" target="_blank" rel="noopener">SPECITS-34</a>:
            Use SM derived models as REST API resources</td>
        <td>REST WG</td>
        <td>17 Dec 2025</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-87" target="_blank" rel="noopener">SPECITS-87</a>:
            Deprecate ADL2 “Get template at version” and the version query parameter from upload template</td>
        <td>REST WG</td>
        <td>17 Dec 2025</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-46" target="_blank" rel="noopener">SPECITS-46</a>:
            Fix conflictual query execution endpoint-resolution when query name is 'aql'</td>
        <td>REST WG</td>
        <td>17 Dec 2025</td>
    </tr>
    <tr>
        <td>5.4</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-61" target="_blank" rel="noopener">SPECITS-61</a>:
            Cleanup and consolidate Simplified Formats specifications; adding Accept and Content-Type headers to support the Simplified Formats by relevant resource endpoints</td>
        <td>REST WG</td>
        <td>05 Nov 2025</td>
    </tr>
    <tr>
        <td>5.3</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-74" target="_blank" rel="noopener">SPECITS-74</a>:
            Deprecate some of the use of Location response header</td>
        <td>REST WG</td>
        <td>25 Sep 2025</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-82" target="_blank" rel="noopener">SPECITS-82</a>:
            Add ETag weakness indicator W/ prefix</td>
        <td>REST WG</td>
        <td>25 Sep 2025</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-50" target="_blank" rel="noopener">SPECITS-50</a>:
            Return very minimal REST responses to avoid 204 No content codes.</td>
        <td>REST WG</td>
        <td>25 Sep 2025</td>
    </tr>
    <tr>
        <td>5.2</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-77" target="_blank" rel="noopener">SPECITS-77</a>:
            Add support for ITEM_TAGs</td>
        <td>REST WG</td>
        <td>14 Mar 2025</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-80" target="_blank" rel="noopener">SPECITS-80</a>:
            Add admin support to delete EHR</td>
        <td>S Iancu</td>
        <td>14 Mar 2025</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-75" target="_blank" rel="noopener">SPECITS-75</a>:
            Fix inconsistent and invalid header names</td>
        <td>E Astrom, J Grisham, S Iancu</td>
        <td>14 Mar 2025</td>
    </tr>
    <tr>
        <td>5.1</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-73" target="_blank" rel="noopener">SPECITS-73</a>:
            Add Demographic API endpoint</td>
        <td>P Pazos, S Iancu</td>
        <td>11 Sep 2024</td>
    </tr>
    <tr>
        <td>5.0</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-75" target="_blank" rel="noopener">SPECITS-75</a>:
            Change structure and improve yaml specs</td>
        <td>S Iancu</td>
        <td>11 Sep 2024</td>
    </tr>
    <tr>
        <th colspan="4"><a href="https://specifications.openehr.org/releases/ITS-REST/latest" target="_blank" rel="noopener">Release-1.0.3</a></th>
    </tr>
    <tr>
        <td>4.1</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-66" target="_blank" rel="noopener">SPECITS-66</a>:
            Migrate REST API specs to OpenAPI Specification</td>
        <td>S Iancu</td>
        <td>19 Dec 2022</td>
    </tr>
    <tr>
        <th colspan="4"><a href="https://specifications.openehr.org/releases/ITS-REST/Release-1.0.2" target="_blank" rel="noopener">Release-1.0.2</a></th>
    </tr>
    <tr>
        <td>3.4</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-59" target="_blank" rel="noopener">SPECITS-59</a>:
            Specify mimeType used by adl2 template</td>
        <td>S Iancu</td>
        <td>26 Mar 2021</td>
    </tr>
    <tr>
        <td>3.3</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-41" target="_blank" rel="noopener">SPECITS-41</a>:
            Add double quotes to ETag and If-Match headers</td>
        <td>S Iancu</td>
        <td>21 Mar 2021</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-42" target="_blank" rel="noopener">SPECITS-42</a>:
            Fix TEMPLATE_ID value format in examples</td>
        <td>S Iancu</td>
        <td>21 Mar 2021</td>
    </tr>
    <tr>
        <td>3.2</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-56" target="_blank" rel="noopener">SPECITS-56</a>:
            Fixing typos, formatting and minor documentary errors</td>
        <td>S Iancu</td>
        <td>15 Mar 2021</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-45" target="_blank" rel="noopener">SPECITS-45</a>:
            Describe more clearly the datetime format for REST API</td>
        <td>P Pazos, S Iancu</td>
        <td>15 Mar 2021</td>
    </tr>
    <tr>
        <td>3.1</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-57" target="_blank" rel="noopener">SPECITS-57</a>:
            Updating info about simplified JSON formats</td>
        <td>E Sundvall, S Iancu</td>
        <td>13 Mar 2021</td>
    </tr>
    <tr>
        <td>3.0</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-49" target="_blank" rel="noopener">SPECITS-49</a>:
            Describing Resource Identification; add clarification over getting COMPOSITIONs by `version_uid` vs `versioned_object_uid`</td>
        <td>J Smolka, M Polajnar, S Iancu</td>
        <td>08 Mar 2021</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-47" target="_blank" rel="noopener">SPECITS-47</a>: 
            Fix inconsistency over query_parameter(s)</td>
        <td>P Pazos, S Iancu</td>
        <td>06 Mar 2021</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-52" target="_blank" rel="noopener">SPECITS-52</a>:
            Fix wrong example on revision history of the VERSIONED_COMPOSITION and VERSIONED_EHR_STATUS</td>
        <td>J Smolka, S Iancu</td>
        <td>06 Mar 2021</td>
    </tr>
    <tr>
        <th colspan="4"><a href="https://specifications.openehr.org/releases/ITS-REST/Release-1.0.1" target="_blank" rel="noopener">Release-1.0.1</a></th>
    </tr>
    <tr>
        <td>2.5</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-37" target="_blank" rel="noopener">SPECITS-37</a>: Fix query definition endpoint and content-type errors</td>
        <td>P Pazos, J Smolka, S Iancu</td>
        <td>01 Oct 2019</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-38" target="_blank" rel="noopener">SPECITS-38</a>:
            Fix response status code for semantic validation errors (fixes also
            <a href="https://specifications.openehr.org/tickets/SPECPR-298" target="_blank" rel="noopener">SPECPR-298</a>)</td>
        <td>P Pazos, S Iancu</td>
        <td>01 Oct 2019</td>
    </tr>
    <tr>
        <td>2.4</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-33" target="_blank" rel="noopener">SPECITS-33</a>: Add reference to Simplified Data Template (SDT) format specification</td>
        <td>I McNicoll, T Beale, S Iancu</td>
        <td>17 Oct 2019</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-40" target="_blank" rel="noopener">SPECITS-40</a>:
            Fix and improve documentation on EHR (fixes also
            <a href="https://specifications.openehr.org/tickets/SPECPR-312" target="_blank" rel="noopener">SPECPR-312</a>)</td>
        <td>P Pazos, S Iancu</td>
        <td>28 Sep 2019</td>
    </tr>
    <tr>
        <td>2.3</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-32" target="_blank" rel="noopener">SPECITS-32</a>: Fix typos and minor documentary errors (fixes
            <a href="https://specifications.openehr.org/tickets/SPECPR-252" target="_blank" rel="noopener">SPECPR-252</a>,
            <a href="https://specifications.openehr.org/tickets/SPECPR-252" target="_blank" rel="noopener">SPECPR-253</a>,
            <a href="https://specifications.openehr.org/tickets/SPECPR-255" target="_blank" rel="noopener">SPECPR-255</a>,
            <a href="https://specifications.openehr.org/tickets/SPECPR-298" target="_blank" rel="noopener">SPECPR-298</a>,
            <a href="https://specifications.openehr.org/tickets/SPECPR-333" target="_blank" rel="noopener">SPECPR-333</a>,
            <a href="https://specifications.openehr.org/tickets/SPECPR-337" target="_blank" rel="noopener">SPECPR-337</a>)</td>
        <td>J Smolka, P Pazos, E Sundvall, T Beale, S Iancu</td>
        <td>1 Sep 2019</td>
    </tr>
    <tr>
        <td>2.2</td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-24" target="_blank" rel="noopener">SPECITS-24</a>: Added changelog</td>
        <td>J Smolka, S Iancu</td>
        <td>12 May 2019</td>
    </tr>
    <tr>
        <td></td>
        <td><a href="https://specifications.openehr.org/tickets/SPECITS-25" target="_blank" rel="noopener">SPECITS-25</a>,
            <a href="https://specifications.openehr.org/tickets/SPECITS-29" target="_blank" rel="noopener">SPECITS-29</a>: Change layout and structure</td>
        <td>J Smolka, S Iancu</td>
        <td>12 May 2019</td>
    </tr>
    <tr>
        <td>2.1</td>
        <td>Update links to new openEHR specifications website</td>
        <td>S Iancu</td>
        <td>16 Dec 2018</td>
    </tr>
    <tr>
        <th colspan="4"><a href="https://specifications.openehr.org/releases/ITS-REST/Release-1.0.0" target="_blank" rel="noopener">Release-1.0.0</a></th>
    </tr>
    </tbody>
</table>
