// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The declared deployment posture: what this deployment is allowed to hold,
//! and which separations it has actually made (#3226).
//!
//! FerroEHR has a set of properties a deployment either has or does not have:
//! separated database credentials, separated database clusters, a declared
//! subject pseudonym namespace, an audit trail with a durable sink, schema
//! preparation on its own credential. Every one is a configuration key an
//! operator can leave unset, and a deployment that quietly has none of them
//! looks, from its own logs and its own API, exactly like one that has all of
//! them. The top-level `deployment_profile` key gives the server a declared
//! posture: `production` refuses to start when a property is missing and not
//! explicitly accepted, `sandbox` says which are missing, loudly and on every
//! surface, so it cannot be mistaken for the other.
//!
//! **This is FerroEHR's own posture, not a legal requirement.** GDPR Art. 4(5)
//! asks that the additional information be "kept separately and … subject to
//! technical and organisational measures"
//! (<https://eur-lex.europa.eu/eli/reg/2016/679/oj>), not that it sit on a
//! separate server; one cluster with separated schemas, roles and no role that
//! spans them is a defensible reading. Two clusters are materially stronger
//! all the same, because a superuser, an instance-wide point-in-time recovery
//! and a single compromise are bridges no grant can close, and that is a
//! choice a deployment should make deliberately. There is no `research`
//! value: the flag controls RIGOUR, not purpose, and EHDS Chapter IV secondary
//! use runs on real patient data under a data permit, which is production
//! rigour whatever it is for.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::config::FerroEhrConfig;
use crate::db::MigrationMode;

/// How rigorously this deployment is held to the separations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentProfile {
    /// Must not hold real personal data. One cluster and one credential are
    /// fine, and the server says so on every surface. The default, so no
    /// existing deployment changes behaviour by upgrading.
    #[default]
    Sandbox,
    /// Holds real personal data. The server refuses to start while a
    /// [`DeploymentGap`] is open and not accepted by name.
    Production,
}

impl DeploymentProfile {
    /// The configuration token (`"sandbox"` / `"production"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::Production => "production",
        }
    }
}

impl fmt::Display for DeploymentProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A `deployment_profile` token that names no profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentProfileParseError {
    /// The token as written.
    pub unrecognized: String,
}

impl fmt::Display for DeploymentProfileParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown deployment_profile `{}` (expected `sandbox` or `production`)",
            self.unrecognized
        )
    }
}

impl std::error::Error for DeploymentProfileParseError {}

impl std::str::FromStr for DeploymentProfile {
    type Err = DeploymentProfileParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sandbox" => Ok(Self::Sandbox),
            "production" => Ok(Self::Production),
            other => Err(DeploymentProfileParseError {
                unrecognized: other.to_owned(),
            }),
        }
    }
}

/// A separation a production deployment is expected to have made.
///
/// Each is a real, checkable property; none is a box ticked by being present.
/// The token is what `deployment_accepts` names to run `production` without
/// it, which is then said loudly at boot and on `/rest/status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentGap {
    /// The clinical, demographic and linkage domains do not each connect on
    /// their own database credential.
    SharedCredential,
    /// Two of the three domains reach the same `PostgreSQL` cluster, read from
    /// `pg_control_system().system_identifier` on each pool, never from the
    /// DSN text: two DSNs can reach one cluster through different names, a
    /// proxy or a pooler.
    SharedCluster,
    /// `privacy.subject_namespaces` is empty, so the rule that refuses a real
    /// identifier as an `EHR_STATUS` subject is out of force.
    OpenSubjectNamespace,
    /// Auditing is off, or on with no durable sink, so access is not recorded
    /// (EHDS Annex II 3.2 is not satisfiable by a deployment that logs nothing).
    AuditOff,
    /// Schema preparation runs on the clinical runtime credential
    /// (`db.migrate = "apply"` with no `db.migrate_url`), so the credential
    /// that serves requests can also alter the schema.
    MigrateOnRuntimeCredential,
}

impl DeploymentGap {
    /// The configuration token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SharedCredential => "shared_credential",
            Self::SharedCluster => "shared_cluster",
            Self::OpenSubjectNamespace => "open_subject_namespace",
            Self::AuditOff => "audit_off",
            Self::MigrateOnRuntimeCredential => "migrate_on_runtime_credential",
        }
    }

    /// What is missing, what was found, and what to change: the sentence the
    /// banner, the boot log, the refusal and `/rest/status` all carry.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::SharedCredential => {
                "the clinical, demographic and linkage domains share a database credential; set \
                 [db] demographic_url and linkage_url to roles that hold one domain each"
            }
            Self::SharedCluster => {
                "two domains reach the same PostgreSQL cluster (pg_control_system().system_identifier \
                 is equal); give the demographic and linkage domains their own clusters"
            }
            Self::OpenSubjectNamespace => {
                "[privacy] subject_namespaces is empty, so an EHR_STATUS subject may carry a real \
                 identifier; declare the pseudonym namespaces this deployment issues"
            }
            Self::AuditOff => {
                "no access log is written: [audit] is disabled or has no durable sink; enable it \
                 with the local store, syslog or the FHIR feed on"
            }
            Self::MigrateOnRuntimeCredential => {
                "[db] migrate is `apply` on the clinical runtime credential; set [db] migrate_url to \
                 the credential that prepares the schema, or run migrate = \"verify\""
            }
        }
    }
}

impl fmt::Display for DeploymentGap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The cluster each domain pool reached, as `pg_control_system().system_identifier`.
///
/// `None` where the identifier could not be read; a gap is never inferred from
/// an unknown.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClusterIdentities {
    /// The clinical pool's cluster.
    pub clinical: Option<String>,
    /// The demographic pool's cluster.
    pub demographic: Option<String>,
    /// The linkage pool's cluster.
    pub linkage: Option<String>,
}

impl ClusterIdentities {
    /// Whether any two known identifiers are equal.
    #[must_use]
    pub fn any_shared(&self) -> bool {
        let known: Vec<&String> = [&self.clinical, &self.demographic, &self.linkage]
            .into_iter()
            .flatten()
            .collect();
        known
            .iter()
            .enumerate()
            .any(|(i, a)| known.iter().skip(i + 1).any(|b| a == b))
    }
}

/// The evaluated posture: what the profile asserts, what is open, what was
/// accepted by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeploymentPosture {
    /// The declared profile.
    pub profile: DeploymentProfile,
    /// The separations this deployment has not made, whether or not accepted.
    pub gaps: Vec<DeploymentGap>,
    /// The open gaps `deployment_accepts` named, so the server runs with them.
    pub accepted: Vec<DeploymentGap>,
}

impl DeploymentPosture {
    /// Evaluate the posture from the resolved configuration and the clusters
    /// the pools reached.
    #[must_use]
    pub fn evaluate(config: &FerroEhrConfig, clusters: &ClusterIdentities) -> Self {
        let mut gaps = Vec::new();
        if !(config.db.roles_are_separated() && config.db.linkage_role_is_separated()) {
            gaps.push(DeploymentGap::SharedCredential);
        }
        if clusters.any_shared() {
            gaps.push(DeploymentGap::SharedCluster);
        }
        if config.privacy.subject_namespaces.is_empty() {
            gaps.push(DeploymentGap::OpenSubjectNamespace);
        }
        let audit = &config.audit;
        if !audit.enabled
            || !(audit.store.enabled || audit.syslog.enabled || audit.fhir_feed.enabled)
        {
            gaps.push(DeploymentGap::AuditOff);
        }
        if config.db.migrate == MigrationMode::Apply && !config.db.migrator_is_separated() {
            gaps.push(DeploymentGap::MigrateOnRuntimeCredential);
        }
        let accepted = gaps
            .iter()
            .copied()
            .filter(|gap| config.deployment_accepts.contains(gap))
            .collect();
        Self {
            profile: config.deployment_profile,
            gaps,
            accepted,
        }
    }

    /// The open gaps `production` refuses on: those not accepted by name.
    #[must_use]
    pub fn refusals(&self) -> Vec<DeploymentGap> {
        self.gaps
            .iter()
            .copied()
            .filter(|gap| !self.accepted.contains(gap))
            .collect()
    }

    /// Whether the server may start under this posture.
    #[must_use]
    pub fn permits_boot(&self) -> bool {
        self.profile == DeploymentProfile::Sandbox || self.refusals().is_empty()
    }

    /// The refusal, when [`Self::permits_boot`] is false: every open gap, its
    /// finding and its remedy, so one boot fixes them all.
    #[must_use]
    pub fn refusal_message(&self) -> String {
        let mut out = String::from(
            "deployment_profile = \"production\" refuses to start: this deployment has not made \
             the separations production asserts. Make them, or accept each one by name in \
             deployment_accepts (which is then stated on every boot and on /rest/status):",
        );
        for gap in self.refusals() {
            out.push_str("\n  - ");
            out.push_str(gap.as_str());
            out.push_str(": ");
            out.push_str(gap.describe());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::secret::SecretUrl;

    fn separated() -> FerroEhrConfig {
        FerroEhrConfig {
            db: crate::db::DbConfig {
                demographic_url: Some(SecretUrl::new("postgres://d@h/x")),
                linkage_url: Some(SecretUrl::new("postgres://l@h/x")),
                migrate_url: Some(SecretUrl::new("postgres://m@h/x")),
                ..crate::db::DbConfig::default()
            },
            privacy: crate::privacy::config::PrivacyConfig {
                subject_namespaces: vec!["urn:example:pseudonym".to_owned()],
                ..crate::privacy::config::PrivacyConfig::default()
            },
            ..FerroEhrConfig::default()
        }
    }

    fn production(base: FerroEhrConfig) -> FerroEhrConfig {
        FerroEhrConfig {
            deployment_profile: DeploymentProfile::Production,
            ..base
        }
    }

    fn distinct_clusters() -> ClusterIdentities {
        ClusterIdentities {
            clinical: Some("1".to_owned()),
            demographic: Some("2".to_owned()),
            linkage: Some("3".to_owned()),
        }
    }

    /// The shipped default is `sandbox`, and a stock configuration on one
    /// cluster is exactly the deployment that must not hold real data: every
    /// gap but the audit one is open, and boot is permitted.
    #[test]
    fn the_default_is_sandbox_and_a_stock_deployment_names_its_gaps() {
        let config = FerroEhrConfig::default();
        assert_eq!(config.deployment_profile, DeploymentProfile::Sandbox);
        let clusters = ClusterIdentities {
            clinical: Some("1".to_owned()),
            demographic: Some("1".to_owned()),
            linkage: Some("1".to_owned()),
        };
        let posture = DeploymentPosture::evaluate(&config, &clusters);
        assert_eq!(
            posture.gaps,
            [
                DeploymentGap::SharedCredential,
                DeploymentGap::SharedCluster,
                DeploymentGap::OpenSubjectNamespace,
                DeploymentGap::MigrateOnRuntimeCredential,
            ],
            "auditing is on by default with the local store, so it is not a gap"
        );
        assert!(posture.permits_boot());
    }

    /// `production` refuses every open gap by name and lists the remedy.
    #[test]
    fn production_refuses_each_open_gap_and_names_the_remedy() {
        let config = production(FerroEhrConfig {
            audit: crate::system_log::config::AuditConfig {
                enabled: false,
                ..crate::system_log::config::AuditConfig::default()
            },
            ..FerroEhrConfig::default()
        });
        let posture = DeploymentPosture::evaluate(&config, &ClusterIdentities::default());
        assert!(!posture.permits_boot());
        let message = posture.refusal_message();
        for gap in [
            DeploymentGap::SharedCredential,
            DeploymentGap::OpenSubjectNamespace,
            DeploymentGap::AuditOff,
            DeploymentGap::MigrateOnRuntimeCredential,
        ] {
            assert!(message.contains(gap.as_str()), "{message}");
            assert!(message.contains(gap.describe()), "{message}");
        }
        assert!(
            !posture.gaps.contains(&DeploymentGap::SharedCluster),
            "an unknown cluster identity is never read as a gap"
        );
    }

    /// A fully separated deployment on three clusters boots as `production`
    /// with nothing open.
    #[test]
    fn a_separated_deployment_boots_as_production() {
        let config = production(separated());
        let posture = DeploymentPosture::evaluate(&config, &distinct_clusters());
        assert!(posture.gaps.is_empty(), "{posture:?}");
        assert!(posture.permits_boot());
    }

    /// Two DSNs reaching one cluster are refused by the cluster identity, not
    /// by the host string; accepting the gap by name permits boot and keeps
    /// it listed.
    #[test]
    fn a_shared_cluster_is_refused_by_identity_and_accepted_only_by_name() {
        let config = production(separated());
        let same_cluster = ClusterIdentities {
            clinical: Some("7".to_owned()),
            demographic: Some("7".to_owned()),
            linkage: Some("8".to_owned()),
        };
        let posture = DeploymentPosture::evaluate(&config, &same_cluster);
        assert_eq!(posture.gaps, [DeploymentGap::SharedCluster]);
        assert!(!posture.permits_boot());

        let config = FerroEhrConfig {
            deployment_accepts: vec![DeploymentGap::SharedCluster],
            ..config
        };
        let accepted = DeploymentPosture::evaluate(&config, &same_cluster);
        assert!(accepted.permits_boot());
        assert_eq!(accepted.accepted, [DeploymentGap::SharedCluster]);
        assert_eq!(
            accepted.gaps,
            [DeploymentGap::SharedCluster],
            "accepting hides nothing"
        );
    }

    /// `Display`/`FromStr` round-trip the tokens, and serde spells them the
    /// same way.
    #[test]
    fn tokens_round_trip() {
        for profile in [DeploymentProfile::Sandbox, DeploymentProfile::Production] {
            let token = profile.to_string();
            assert_eq!(token.parse::<DeploymentProfile>().ok(), Some(profile));
            assert_eq!(
                serde_json::to_string(&profile).ok(),
                Some(format!("\"{token}\""))
            );
        }
        assert!("research".parse::<DeploymentProfile>().is_err());
        for gap in [
            DeploymentGap::SharedCredential,
            DeploymentGap::SharedCluster,
            DeploymentGap::OpenSubjectNamespace,
            DeploymentGap::AuditOff,
            DeploymentGap::MigrateOnRuntimeCredential,
        ] {
            assert_eq!(
                serde_json::to_string(&gap).ok(),
                Some(format!("\"{}\"", gap.as_str()))
            );
        }
    }
}
