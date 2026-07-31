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
{{- if or $inlineDb .Values.secrets.authOidcHmacSecret .Values.secrets.signingKeyPassphrase .Values.secrets.eventsUrl .Values.secrets.fhirOutboundUrl .Values.secrets.multimediaAccessKeyId .Values.secrets.multimediaSecretAccessKey -}}
true
{{- end -}}
{{- end }}
