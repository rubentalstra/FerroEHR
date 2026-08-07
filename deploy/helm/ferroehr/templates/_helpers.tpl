{{/*
Expand the name of the chart.
*/}}
{{- define "ferroehr.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Fully qualified app name (max 63 chars for DNS labels).
*/}}
{{- define "ferroehr.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Chart name and version, for the helm.sh/chart label.
*/}}
{{- define "ferroehr.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels.
*/}}
{{- define "ferroehr.labels" -}}
helm.sh/chart: {{ include "ferroehr.chart" . }}
{{ include "ferroehr.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/part-of: ferroehr
{{- end }}

{{/*
Selector labels.
*/}}
{{- define "ferroehr.selectorLabels" -}}
app.kubernetes.io/name: {{ include "ferroehr.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
ServiceAccount name.
*/}}
{{- define "ferroehr.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "ferroehr.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
The container port the ops-introspection surface (prometheus / metrics / info /
env / loggers) is reachable on. When management.port is set the surface runs on
its own listener; otherwise it shares the main API port. The health probes are
never on this port-selection path: they are always served on the main HTTP port.
*/}}
{{- define "ferroehr.managementPort" -}}
{{- if .Values.config.management.port }}
{{- .Values.config.management.port }}
{{- else }}
{{- 8080 }}
{{- end }}
{{- end }}

{{/*
The container image reference.

A digest wins over a tag, and the separator differs: `repository@sha256:…` versus
`repository:tag`. A digest is what a build-provenance attestation is made over, so
deploying by digest is what makes `gh attestation verify` bind to the image that
is actually running — a tag can be moved after it is verified.

`sha256:` is accepted with or without the prefix, because both spellings are in
circulation and the one-character difference between them is an unhelpful way to
fail a deployment.
*/}}
{{- define "ferroehr.image" -}}
{{- if .Values.image.digest }}
{{- $digest := .Values.image.digest }}
{{- if not (hasPrefix "sha256:" $digest) }}{{- $digest = printf "sha256:%s" $digest }}{{- end }}
{{- printf "%s@%s" .Values.image.repository $digest }}
{{- else }}
{{- printf "%s:%s" .Values.image.repository (.Values.image.tag | default .Chart.AppVersion) }}
{{- end }}
{{- end }}

{{/*
The chart-managed Secret name (env-borne secrets: DB DSN, passphrases, keys).
*/}}
{{- define "ferroehr.secretName" -}}
{{- printf "%s-env" (include "ferroehr.fullname" .) }}
{{- end }}

{{/*
The chart-managed config-files Secret name (mounted TOML/PEM at /etc/ferroehr).
*/}}
{{- define "ferroehr.configSecretName" -}}
{{- printf "%s-config" (include "ferroehr.fullname" .) }}
{{- end }}

{{/*
Whether the chart-managed env Secret has any content (so we only create/mount it
when there is at least one secret value to carry).
*/}}
{{- define "ferroehr.hasChartSecret" -}}
{{- $inlineDb := and (not .Values.database.existingSecret) .Values.database.url }}
{{- if or $inlineDb .Values.secrets.basicUserPasswordHashes .Values.secrets.authOidcHmacSecret .Values.secrets.signingKeyPassphrase .Values.secrets.eventsUrl .Values.secrets.fhirOutboundUrl .Values.secrets.auditFhirFeedUrl .Values.secrets.multimediaAccessKeyId .Values.secrets.multimediaSecretAccessKey .Values.secrets.terminologyOauth2ClientSecrets -}}
true
{{- end -}}
{{- end }}

{{/*
Whether any secret is carried as a FILE rather than an env value. A secret whose
config key has a `*_file` sibling is mounted from a read-only volume, which the
OWASP Kubernetes Security Cheat Sheet prefers over an environment variable
(https://cheatsheetseries.owasp.org/cheatsheets/Kubernetes_Security_Cheat_Sheet.html).
*/}}
{{- define "ferroehr.hasFileSecrets" -}}
{{- if or (eq (include "ferroehr.hasChartFileSecrets" .) "true") .Values.database.existingSecret -}}
true
{{- end -}}
{{- end }}

{{/*
Whether the CHART-MANAGED Secret carries any file-borne key. Distinct from
`hasFileSecrets`: an operator-supplied `database.existingSecret` puts a file in the
mount without the chart's own Secret existing at all, and projecting a source that
names a Secret nothing created makes the pod fail to mount.
*/}}
{{- define "ferroehr.hasChartFileSecrets" -}}
{{- $inlineDb := and (not .Values.database.existingSecret) .Values.database.url -}}
{{- if or $inlineDb .Values.secrets.authOidcHmacSecret .Values.secrets.signingKeyPassphrase .Values.secrets.multimediaSecretAccessKey .Values.secrets.terminologyOauth2ClientSecrets .Values.secrets.basicUserPasswordHashes .Values.secrets.eventsUrl .Values.secrets.fhirOutboundUrl -}}
true
{{- end -}}
{{- end }}

{{/*
The directory the file-borne secrets are mounted at. Deliberately NOT under
/etc/ferroehr: that path is the config projection, and authz.abac.cedar.policy_dir
commonly points at it, so a secrets subdirectory there would be walked as policy.
*/}}
{{- define "ferroehr.secretMountPath" -}}
/etc/ferroehr-secrets
{{- end }}

{{/*
Secret-shaped leaf key names, as one regex. A ConfigMap is not a sensitive
object — it is readable with namespace read, collected by backup tooling that
excludes Secrets, and not covered by Secret encryption at rest — so a credential
must never reach one (Kubernetes ConfigMap docs: "A ConfigMap is not designed to
hold large chunks of data … If you want to store data that is confidential, use
a Secret", https://kubernetes.io/docs/concepts/configuration/configmap/).

Deny-by-default is the point: matching on the NAME rather than on a fixed list of
today's keys is what makes a secret key added to the server's config tree
tomorrow fail the render instead of leaking silently. The `_file`/`_path`/`_dir`
suffixes are the one exemption class, and they are provably safe: those keys hold
a PATH, never a credential.
*/}}
{{- define "ferroehr.secretKeyPattern" -}}
(password|passphrase|secret|credential|private_key|api_key|(^|_)token$)
{{- end }}

{{/*
Recursive scan of a config subtree for secret-bearing keys, emitting one
`class\tpath\tremedy` line per finding. Call with (dict "node" <tree> "path" "").

Two classes, because there are two right answers. `routed` = the chart already
carries this secret through a `secrets:` key, so a value under `config:` is an
operator mistake and the render fails naming the key that belongs there.
`unrouted` = no `secrets:` route exists, so the whole rendered ferroehr.toml moves
into the chart Secret and no ConfigMap is created — the safe direction, taken
automatically, for a key nobody has routed yet.

NOTE: the `unrouted` class currently has NO members — every secret the server
models now has either a `*_file` sibling or a Secret-borne env route, including a
Basic user's hash since `password_hash_file` landed. The branch is kept precisely
because that can change: a secret key added upstream with no file sibling must
fail safe by default rather than land in a ConfigMap, and deleting the branch is
what would make the next one leak.

The four SecretUrl leaves are matched by PATH because their name (`url`) carries
no shape a classifier can see — a URL's userinfo component is the credential.
*/}}
{{- define "ferroehr.secretScan" -}}
{{- $urlPaths := list "db.url" "events.url" "fhir.outbound.url" "audit.fhir_feed.url" -}}
{{- $node := .node -}}
{{- $path := .path -}}
{{- $kind := kindOf $node -}}
{{- if eq $kind "map" -}}
{{- range $key, $value := $node -}}
{{- $child := ternary $key (printf "%s.%s" $path $key) (eq $path "") -}}
{{- if or (has $child $urlPaths) (and (regexMatch (include "ferroehr.secretKeyPattern" $) $key) (not (regexMatch "(_file|_path|_dir)$" $key))) -}}
{{- printf "%s\t%s\t%s\n" (include "ferroehr.secretClass" $key) $child (include "ferroehr.secretRemedy" $key) -}}
{{- else -}}
{{- include "ferroehr.secretScan" (dict "node" $value "path" $child) -}}
{{- end -}}
{{- end -}}
{{- else if eq $kind "slice" -}}
{{- range $index, $value := $node -}}
{{- include "ferroehr.secretScan" (dict "node" $value "path" (printf "%s[%d]" $path $index)) -}}
{{- end -}}
{{- end -}}
{{- end }}

{{/*
Whether the chart has a dedicated `secrets:` route for a secret-bearing config
key, keyed on its leaf name. Unknown ⇒ `unrouted`, which is what makes a secret
key added upstream tomorrow move to the Secret instead of leaking.
*/}}
{{- define "ferroehr.secretClass" -}}
{{- if has . (list "url" "hmac_secret" "key_passphrase" "secret_access_key" "client_secret" "password_hash") -}}
routed
{{- else -}}
unrouted
{{- end -}}
{{- end }}

{{/*
The `secrets:` key that carries a routed secret, keyed on its leaf name.
*/}}
{{- define "ferroehr.secretRemedy" -}}
{{- $routes := dict
  "url" "route it through the matching `secrets:` key — `eventsUrl` or `fhirOutboundUrl` (mounted as files via events.url_file / fhir.outbound.url_file), `auditFhirFeedUrl` (env; that key still has no `*_file` sibling), or `database.existingSecret` for the DSN (mounted via db.url_file)"
  "hmac_secret" "set `secrets.authOidcHmacSecret` instead"
  "key_passphrase" "set `secrets.signingKeyPassphrase` instead"
  "secret_access_key" "set `secrets.multimediaSecretAccessKey` instead"
  "client_secret" "set `secrets.terminologyOauth2ClientSecrets.<client name>` instead"
  "password_hash" "set `secrets.basicUserPasswordHashes.<username>` instead, and leave only `username`/`roles` under `config:` (the chart mounts the hash and injects password_hash_file)"
-}}
{{- get $routes . | default "no `secrets:` route exists for this key" -}}
{{- end }}

{{/*
"true" when the rendered ferroehr.toml carries a secret the chart cannot route
elsewhere, in which case it is delivered by the Secret and NO ConfigMap exists.
*/}}
{{- define "ferroehr.configInSecret" -}}
{{- $findings := include "ferroehr.secretScan" (dict "node" (omit .Values.config "files") "path" "") | trim -}}
{{- range $finding := splitList "\n" $findings -}}
{{- if hasPrefix "unrouted\t" $finding -}}
true
{{- end -}}
{{- end -}}
{{- end }}

{{/*
The rendered ferroehr.toml body: `.Values.config` minus the separately-mounted
`files`, with the file-borne secret PATHS the chart owns injected, and a routed
secret VALUE refused outright.
*/}}
{{- define "ferroehr.configToml" -}}
{{- $config := omit .Values.config "files" -}}
{{- $findings := include "ferroehr.secretScan" (dict "node" $config "path" "") | trim -}}
{{- $lines := list -}}
{{- range $finding := splitList "\n" $findings -}}
{{- $parts := splitn "\t" 3 $finding -}}
{{- if eq $parts._0 "routed" -}}
{{- $lines = append $lines (printf "  - config.%s: %s" $parts._1 $parts._2) -}}
{{- end -}}
{{- end -}}
{{- if $lines -}}
{{- fail (printf "refusing to render a secret into the ConfigMap (a ConfigMap is not a sensitive object — it is readable with namespace read, collected by tooling that skips Secrets, and not covered by Secret encryption at rest):\n%s" (join "\n" $lines)) -}}
{{- end -}}
{{- $rendered := deepCopy $config -}}
{{- range $user, $hash := .Values.secrets.basicUserPasswordHashes -}}
{{- $users := dig "auth" "basic" "users" (list) $rendered -}}
{{- $matched := false -}}
{{- range $entry := $users -}}
{{- if eq (get $entry "username") $user -}}
{{- $_ := set $entry "password_hash_file" (printf "%s/auth.basic.users.%s.password_hash" (include "ferroehr.secretMountPath" $) $user) -}}
{{- $matched = true -}}
{{- end -}}
{{- end -}}
{{- if not $matched -}}
{{- fail (printf "secrets.basicUserPasswordHashes.%s has no matching entry at config.auth.basic.users[] with username: %s (declare the username and roles there; only the hash belongs in secrets:)" $user $user) -}}
{{- end -}}
{{- end -}}
{{- range $name, $value := .Values.secrets.terminologyOauth2ClientSecrets -}}
{{- $clients := dig "terminology" "external" "oauth2_clients" (dict) $rendered -}}
{{- if not (hasKey $clients $name) -}}
{{- fail (printf "secrets.terminologyOauth2ClientSecrets.%s has no client declared at config.terminology.external.oauth2_clients.%s (declare its token_url/client_id there; only the secret belongs here)" $name $name) -}}
{{- end -}}
{{- $_ := set (get $clients $name) "client_secret_file" (printf "%s/terminology.external.oauth2_clients.%s.client_secret" (include "ferroehr.secretMountPath" $) $name) -}}
{{- end -}}
{{- toToml $rendered -}}
{{- end }}

{{/*
Whether the config-files Secret exists: any `config.files` entry, or the whole
rendered TOML having moved there because it carries an unroutable secret.
*/}}
{{- define "ferroehr.hasConfigSecret" -}}
{{- if or .Values.config.files (eq (include "ferroehr.configInSecret" .) "true") -}}
true
{{- end -}}
{{- end }}
