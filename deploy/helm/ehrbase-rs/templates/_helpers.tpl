{{/*
Expand the name of the chart.
*/}}
{{- define "ehrbase-rs.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Fully qualified app name (max 63 chars for DNS labels).
*/}}
{{- define "ehrbase-rs.fullname" -}}
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
{{- define "ehrbase-rs.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels.
*/}}
{{- define "ehrbase-rs.labels" -}}
helm.sh/chart: {{ include "ehrbase-rs.chart" . }}
{{ include "ehrbase-rs.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/part-of: ehrbase-rs
{{- end }}

{{/*
Selector labels.
*/}}
{{- define "ehrbase-rs.selectorLabels" -}}
app.kubernetes.io/name: {{ include "ehrbase-rs.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
ServiceAccount name.
*/}}
{{- define "ehrbase-rs.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "ehrbase-rs.fullname" .) .Values.serviceAccount.name }}
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
{{- define "ehrbase-rs.managementPort" -}}
{{- if .Values.config.management.port }}
{{- .Values.config.management.port }}
{{- else }}
{{- 8080 }}
{{- end }}
{{- end }}

{{/*
The chart-managed Secret name (env-borne secrets: DB DSN, passphrases, keys).
*/}}
{{- define "ehrbase-rs.secretName" -}}
{{- printf "%s-env" (include "ehrbase-rs.fullname" .) }}
{{- end }}

{{/*
The chart-managed config-files Secret name (mounted TOML/PEM at /etc/ehrbase).
*/}}
{{- define "ehrbase-rs.configSecretName" -}}
{{- printf "%s-config" (include "ehrbase-rs.fullname" .) }}
{{- end }}

{{/*
Whether the chart-managed env Secret has any content (so we only create/mount it
when there is at least one secret value to carry).
*/}}
{{- define "ehrbase-rs.hasChartSecret" -}}
{{- $inlineDb := and (not .Values.database.existingSecret) .Values.database.url }}
{{- if or $inlineDb .Values.secrets.authOidcHmacSecret .Values.secrets.signingKeyPassphrase .Values.secrets.eventsUrl .Values.secrets.fhirOutboundUrl .Values.secrets.multimediaAccessKeyId .Values.secrets.multimediaSecretAccessKey -}}
true
{{- end -}}
{{- end }}
